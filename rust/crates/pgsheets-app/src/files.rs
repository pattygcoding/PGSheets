//! Opening and saving files, and the dialogs that choose them.
//!
//! The readers and writers live in `pgsheets-io` and know nothing about a window; this is the thin
//! layer that turns a path into rows and rows into a workbook, and reports what the user has to be told.
//!
//! Reading happens on a worker thread, because that is the operation whose cost is the file's. It hands
//! back plain rows rather than a `Workbook`: a cell's text is held as an `Rc<str>`, so neither the
//! workbook nor a snapshot of it may cross a thread boundary. Writing stays on the caller's thread for
//! the same reason — a writer reads from the workbook, which owns those `Rc`s.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use pgsheets_core::store::ImportSink;
use pgsheets_core::style::CellStyle;
use pgsheets_formulas::Engine;
use pgsheets_io::{ExportOptions, ExportResult, ImportResult, ImporterRegistry, SheetExporter, SheetRef};
use pgsheets_sheet::Workbook;

/// One worksheet a file produced, as rows that can cross a thread boundary.
#[derive(Debug, Default)]
pub struct ImportedSheet {
    /// The name shown on the tab.
    pub name: String,

    /// Non-empty cells, in no particular order.
    pub cells: Vec<(i32, i32, String)>,

    /// Number formats, which a reader passes alongside the values so a date stays a date.
    pub formats: Vec<(i32, i32, String)>,

    /// Styles, which a reader passes alongside the values so a bold cell stays bold.
    pub styles: Vec<(i32, i32, CellStyle)>,
}

impl ImportedSheet {
    /// Writes the rows into a workbook, inside one bulk update.
    ///
    /// The bulk update is what makes an import cost one recalculation rather than one per cell, which is
    /// the whole reason the rows are collected first. Formats and styles go on after the values, because
    /// they only mean something once the cell they belong to exists.
    pub fn write_into(&self, workbook: &mut Workbook) {
        workbook.begin_bulk_update();

        for (row, column, raw_input) in &self.cells {
            workbook.set_raw_input(*row, *column, raw_input);
        }

        for (row, column, number_format) in &self.formats {
            workbook.set_number_format(*row, *column, Some(number_format));
        }

        for (row, column, style) in &self.styles {
            workbook.set_cell_style(*row, *column, *style);
        }

        workbook.end_bulk_update();
    }
}

/// A file that has been read: the worksheets it held, and what the reader reported.
#[derive(Debug)]
pub struct Imported {
    /// Every worksheet, in the order the file listed them.
    pub sheets: Vec<ImportedSheet>,

    /// What the reader reported: the used extent, the sheet names, the warnings.
    pub result: ImportResult,
}

impl ImportSink for Imported {
    /// A cell belongs to the worksheet most recently announced, which is why the rows are grouped by
    /// [`ImportSink::begin_sheet`] rather than all landing in one list.
    fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.cells.push((row, column, raw_input.to_string()));
        }
    }

    fn set_number_format(&mut self, row: i32, column: i32, number_format: Option<&str>) {
        if let (Some(number_format), Some(sheet)) = (number_format, self.sheets.last_mut()) {
            sheet.formats.push((row, column, number_format.to_string()));
        }
    }

    fn set_cell_style(&mut self, row: i32, column: i32, style: Option<&CellStyle>) {
        if let (Some(style), Some(sheet)) =
            (style.filter(|style| !style.is_default()), self.sheets.last_mut())
        {
            sheet.styles.push((row, column, *style));
        }
    }

    fn begin_sheet(&mut self, name: &str) {
        self.sheets.push(ImportedSheet {
            name: name.to_string(),
            ..Default::default()
        });
    }
}

/// A file operation running on a worker thread, so a big workbook does not freeze the window.
pub struct Job<T> {
    receiver: Receiver<Result<T, String>>,
}

impl<T: Send + 'static> Job<T> {
    /// Runs the work on a worker thread. Collect the result with [`Job::poll`] or [`Job::wait`].
    pub fn spawn(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Self {
        let (sender, receiver) = channel();

        std::thread::spawn(move || {
            // A failed send means nobody is listening any more, which is not worth reporting.
            let _ = sender.send(work());
        });

        Self { receiver }
    }

    /// The result, if the worker has finished. Never blocks, because it is called from a frame.
    pub fn poll(&self) -> Option<Result<T, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),

            Err(TryRecvError::Empty) => None,

            // The worker panicked or the sender was dropped.
            Err(TryRecvError::Disconnected) => Some(Err(FAILED.to_string())),
        }
    }

    /// Waits for the worker, for a caller that is not drawing frames.
    pub fn wait(self) -> Result<T, String> {
        self.receiver.recv().unwrap_or_else(|_| Err(FAILED.to_string()))
    }
}

/// What a file operation says when nothing came back.
const FAILED: &str = "the file could not be read";

/// Reads a spreadsheet from a file. Blocking, so a host that cares about responsiveness wants
/// [`spawn_open`] instead.
pub fn open(path: &Path) -> Result<Imported, String> {
    read(path)
}

/// Reads a spreadsheet on a worker thread.
pub fn spawn_open(path: &Path) -> Job<Imported> {
    let path = path.to_path_buf();

    Job::spawn(move || read(&path))
}

/// The reading half, which is what runs on the worker.
fn read(path: &Path) -> Result<Imported, String> {
    let file_name = file_name_of(path)?;

    let mut file = File::open(path).map_err(|error| describe(path, &error))?;

    let registry = ImporterRegistry::standard();
    let mut imported = Imported {
        sheets: Vec::new(),
        result: ImportResult::new(),
    };

    let outcome = registry
        .import(&file_name, &mut file, &mut imported)
        .ok_or_else(|| format!("{file_name} is not a format this application reads"))?;

    imported.result = outcome.map_err(|error| error.to_string())?;

    Ok(imported)
}

/// Writes a document to a file, reporting everything lost on the way.
///
/// `sheets` are in tab order, and `active_name` names the sheet a single-table format such as CSV
/// writes. The document is written beside its destination and then moved into place, so a writer that
/// fails halfway leaves the file that was already there untouched. Creating the destination first
/// would truncate it, and a failed save would then have destroyed a perfectly good file.
pub fn save(path: &Path, sheets: &[(&str, &Workbook)], active_name: &str) -> Result<ExportResult, String> {
    let file_name = file_name_of(path)?;

    let staging = staging_path(path);
    let mut file = File::create(&staging).map_err(|error| describe(&staging, &error))?;

    let options = ExportOptions {
        sheet_name: active_name.to_string(),
        ..Default::default()
    };

    let refs: Vec<SheetRef> = sheets
        .iter()
        .map(|(name, workbook)| SheetRef {
            name,
            source: *workbook,
        })
        .collect();

    let outcome = SheetExporter::standard().export_workbook(&file_name, &mut file, &refs, &options);

    let result = match outcome {
        Some(Ok(result)) => result,

        Some(Err(error)) => {
            let _ = std::fs::remove_file(&staging);

            return Err(error.to_string());
        }

        None => {
            let _ = std::fs::remove_file(&staging);

            return Err(format!("no writer is registered for '{file_name}'"));
        }
    };

    // The handle has to be closed before the file can be moved over its destination.
    drop(file);

    std::fs::rename(&staging, path).map_err(|error| describe(path, &error))?;

    Ok(result)
}

/// A sibling of `path` to write into before it is moved into place, in the same directory so that the
/// move is a rename rather than a copy.
fn staging_path(path: &Path) -> PathBuf {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("book");

    path.with_file_name(format!(".{name}.saving"))
}

/// An empty workbook over the real engine, which is what a new document is.
pub fn new_workbook() -> Workbook {
    Workbook::new(Box::new(Engine::new()))
}

/// File-picker patterns for everything this application can read.
pub fn open_patterns() -> Vec<String> {
    ImporterRegistry::standard().patterns()
}

/// True when some writer claims a file name, which is what decides whether a save can be in place.
pub fn can_export(file_name: &str) -> bool {
    SheetExporter::standard().can_export(file_name)
}

/// Asks the user which file to open.
pub fn ask_open() -> Option<PathBuf> {
    let patterns = open_patterns();
    let patterns: Vec<&str> = patterns.iter().map(String::as_str).collect();

    rfd::FileDialog::new()
        .set_title("Open spreadsheet")
        .add_filter("Spreadsheets", &patterns)
        .pick_file()
}

/// Asks the user where to save, defaulting to the name the document already has.
pub fn ask_save_as(suggested: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Save spreadsheet")
        .set_file_name(suggested)
        .add_filter("Excel workbook", &["xlsx"])
        .add_filter("Delimited text", &["csv", "tsv"])
        .save_file()
}

/// Asks a yes/no question. Blocking, like every native dialog here.
pub fn confirm(title: &str, description: &str) -> bool {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(description)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// Tells the user something they cannot be asked to answer.
///
/// Errors reach the status bar instead — opening a file that is not a spreadsheet, or a save that failed,
/// leaves a message there rather than a modal to dismiss — so this is here for a host that wants a modal
/// anyway.
pub fn tell(title: &str, description: &str) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(description)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// The file name as the registries expect it, which is what they match an extension against.
fn file_name_of(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| format!("{} has no usable file name", path.display()))
}

fn describe(path: &Path, error: &std::io::Error) -> String {
    format!("{} could not be reached: {error}", path.display())
}
