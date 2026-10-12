//! The window: the grid, the one editor, the menus and the file commands.
//!
//! This is the counterpart of the C# `MainWindow`: it owns the document, positions the single floating
//! editor over the active cell, and turns the grid's [`GridAction`]s into edits. The rule that the grid
//! never edits the workbook itself is what keeps clipboard, undo and file handling in one place.

use std::path::PathBuf;

use eframe::egui::{self, Key};
use pgsheets_core::{CellAddress, CellBuffer, CellRange, CellValue};
use pgsheets_io::export::csv;
use pgsheets_io::{ExportOptions, ExportResult, SheetExporter};
use pgsheets_sheet::{Workbook, sheet_paste};

use crate::files;
use crate::grid::{GridAction, GridView};

/// The cell being edited, and the text in it.
struct Editor {
    row: i32,
    column: i32,
    text: String,

    /// False until the text box has been given focus once. Focusing it in the frame it appears, and
    /// only then, is what makes typing into a cell feel immediate.
    focused: bool,
}

/// What the user asked for, held until they have answered the unsaved-changes question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    /// Start an empty document.
    New,

    /// Open another file over this one.
    Open,

    /// Close the window.
    Exit,
}

/// What the user chose when asked about unsaved changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveChoice {
    /// Save first, then carry on.
    Save,

    /// Carry on and lose the changes.
    Discard,

    /// Do nothing at all. Cancelling is the safe outcome, which is why dismissing the question means it.
    Cancel,
}

/// A question that has to be answered before the document can change.
enum Prompt {
    /// There are unsaved changes and they are about to be lost.
    Unsaved {
        /// What to ask, phrased for the action that is about to happen.
        message: String,

        /// What to do once the question is answered.
        pending: Pending,
    },
}

/// The application state for one window.
pub struct PgSheetsApp {
    workbook: Workbook,
    grid: GridView,
    editor: Option<Editor>,
    path: Option<PathBuf>,
    dirty: bool,

    /// How many worksheets the opened file held. Saving writes one, so a document with more must not be
    /// written back over the file it came from.
    worksheet_count: i32,

    /// The worksheet name to write on save: the one the file had, so a save keeps its name.
    sheet_name: String,

    status: String,

    /// The aggregate of the selection, shown beside the status message. Empty for a single cell, which
    /// is what a spreadsheet shows too.
    selection_text: String,

    prompt: Option<Prompt>,

    /// The cell the formula bar is showing, so it refills only when the active cell moves.
    formula_cell: Option<CellAddress>,

    /// What the formula bar holds.
    formula_text: String,

    /// True while the formula bar is being typed into rather than just showing the active cell.
    ///
    /// It is a label until it is clicked, so that it is not a second focusable widget: a text box that is
    /// always there is exactly what egui's arrow-key focus navigation hands the keyboard to.
    bar_editing: bool,

    /// What was last copied, so a paste inside the application needs no clipboard round trip.
    clipboard_text: String,

    /// The region the last copy came from, and (with the text) what lets a paste shift relative
    /// references. The clipboard itself carries only text, so a paste from another application cannot be
    /// shifted and keeps its references.
    copied_range: Option<CellRange>,

    /// Text waiting to be handed to the clipboard. The command layer is UI-free, so the frame flushes
    /// this where the context is available.
    clipboard_out: Option<String>,

    /// The document revision the file on disk matches.
    ///
    /// Comparing it against the workbook's own revision is how "unsaved changes" is answered. The
    /// revision only ever moves forward, so undoing after a save leaves the document marked unsaved —
    /// the same thing Excel does, because a save point is a point in time rather than a set of contents.
    saved_revision: u64,

    /// The user asked to close, and it is the frame's job to tell the platform.
    exit_requested: bool,

    /// True once the unsaved-changes question has been answered and the close may proceed.
    allow_close: bool,

    /// A file being read on a worker thread, and where it came from.
    opening: Option<(PathBuf, files::Job<files::Imported>)>,
}

impl Default for PgSheetsApp {
    /// A window holding the sample sheet the C# one started with, so it is not empty before a file is
    /// opened. Seeding is not a user edit, so the history is dropped afterwards rather than left with one
    /// step that would undo the sample.
    fn default() -> Self {
        let mut workbook = files::new_workbook();

        seed_sample(&mut workbook);

        // Seeding moved the revision, so the save point has to be taken *after* it: a new window holds a
        // sample sheet that nobody edited, and calling that "unsaved" would prompt on the first close.
        let saved_revision = workbook.revision();

        Self {
            workbook,
            grid: GridView::default(),
            editor: None,
            path: None,
            dirty: false,
            worksheet_count: 1,
            sheet_name: "Sheet1".to_string(),
            status: "Ready".to_string(),
            selection_text: String::new(),
            prompt: None,
            formula_cell: None,
            formula_text: String::new(),
            bar_editing: false,
            clipboard_text: String::new(),
            clipboard_out: None,
            copied_range: None,
            saved_revision,
            exit_requested: false,
            allow_close: false,
            opening: None,
        }
    }
}

/// The small table a new window starts with: a region per row, two quarters, a total each, and a line of
/// guidance under it. It is the C# sample sheet, cell for cell.
fn seed_sample(workbook: &mut Workbook) {
    workbook.begin_bulk_update();

    for (row, column, text) in [
        (0, 0, "Region"),
        (0, 1, "Q1"),
        (0, 2, "Q2"),
        (0, 3, "Total"),
        (1, 0, "North"),
        (1, 1, "1200"),
        (1, 2, "1450"),
        (1, 3, "=B2+C2"),
        (2, 0, "South"),
        (2, 1, "980"),
        (2, 2, "1100"),
        (2, 3, "=B3+C3"),
        (3, 0, "East"),
        (3, 1, "1600"),
        (3, 2, "1300"),
        (3, 3, "=B4+C4"),
        (5, 0, "Totals"),
        (5, 1, "=SUM(B2:B4)"),
        (5, 2, "=SUM(C2:C4)"),
        (5, 3, "=SUM(D2:D4)"),
        (7, 0, "Edit any cell, or use File > Open to import .xlsx / .csv."),
    ] {
        workbook.set_raw_input(row, column, text);
    }

    workbook.end_bulk_update();
    workbook.forget_history();
}

impl PgSheetsApp {
    /// One frame of the window.
    ///
    /// Public because this is the whole window: the host's `App::ui` is a one-line call to it, and a test
    /// can drive it through a headless context the same way to exercise the window's own wiring.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        // A worker may have finished reading a file since the last frame.
        self.collect_opening();

        // The grid holds the keyboard unless the in-place editor or the formula bar is being typed into.
        self.grid
            .set_keyboard_owner(self.editor.is_none() && !self.bar_editing);

        self.apply_shortcuts(ui);

        egui::MenuBar::new().ui(ui, |ui| self.menus(ui));

        ui.separator();
        self.formula_bar(ui);
        ui.separator();

        // The status bar is laid out upwards from the bottom of what is left, so the grid gets
        // everything in between without either of them knowing about a panel.
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            self.status_bar(ui);
        });

        let origin = ui.available_rect_before_wrap().min;
        let action = self.grid.show(ui, &self.workbook);

        self.apply_action(action);

        if self.edit_overlay(ui, origin) {
            // The editor is gone, so the keyboard goes back to the grid — otherwise the next key is lost
            // to a text box that no longer exists.
            let grid = crate::grid::focus_id(ui);

            ui.ctx().memory_mut(|memory| memory.request_focus(grid));
        }

        // The window's own close button asks the same question File > Exit does. The close is cancelled
        // and re-issued once the question has been answered, which is what the C# window's OnClosing did.
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.allow_close && self.dirty {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);

            self.prompt = Some(Prompt::Unsaved {
                message: "Save changes before closing?".to_string(),
                pending: Pending::Exit,
            });
        }

        self.ask(ui);
        self.sync_title(ui);

        // One place touches the clipboard, and it is the frame rather than a command.
        if let Some(text) = self.clipboard_out.take() {
            ui.ctx().copy_text(text);
        }

        if std::mem::take(&mut self.exit_requested) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// The menu bar: the same commands the C# window had, in the same three menus.
    fn menus(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    self.run(Command::New);
                }

                if ui.button("Open...").clicked() {
                    self.run(Command::Open);
                }

                if ui.button("Save").clicked() {
                    self.run(Command::Save);
                }

                if ui.button("Save As...").clicked() {
                    self.run(Command::SaveAs);
                }

                ui.separator();

                if ui.button("Exit").clicked() {
                    self.run(Command::Exit);
                }
            });

            ui.menu_button("Edit", |ui| {
                let undo = self.workbook.can_undo();
                let redo = self.workbook.can_redo();

                if ui.add_enabled(undo, egui::Button::new("Undo")).clicked() {
                    self.run(Command::Undo);
                }

                if ui.add_enabled(redo, egui::Button::new("Redo")).clicked() {
                    self.run(Command::Redo);
                }

                ui.separator();

                if ui.button("Cut").clicked() {
                    self.run(Command::Cut);
                }

                if ui.button("Copy").clicked() {
                    self.run(Command::Copy);
                }

                if ui.button("Paste").clicked() {
                    self.run(Command::Paste);
                }

                if ui.button("Clear Contents").clicked() {
                    self.run(Command::Clear);
                }

                ui.separator();

                if ui.button("Select All").clicked() {
                    self.run(Command::SelectUsedRange);
                }
            });

            ui.menu_button("Sheet", |ui| {
                if ui.button("Insert Rows").clicked() {
                    self.run(Command::InsertRows);
                }

                if ui.button("Insert Columns").clicked() {
                    self.run(Command::InsertColumns);
                }

                ui.separator();

                if ui.button("Delete Rows").clicked() {
                    self.run(Command::DeleteRows);
                }

                if ui.button("Delete Columns").clicked() {
                    self.run(Command::DeleteColumns);
                }

                ui.separator();

                ui.menu_button("Number format", |ui| {
                    for (index, (label, code)) in NUMBER_FORMATS.iter().enumerate() {
                        // General stands apart from the formats, as it does in the C# menu.
                        if index == 1 {
                            ui.separator();
                        }

                        if ui.button(*label).clicked() {
                            self.run(Command::NumberFormat(*code));
                        }
                    }
                });
            });
        });
    }

    /// The keyboard, while the in-place editor is closed.
    ///
    /// With the editor open these keys belong to the text box, which is the rule the C# window
    /// followed — otherwise Ctrl+C would copy the cell instead of the text being edited.
    fn apply_shortcuts(&mut self, ui: &egui::Ui) {
        if self.editor.is_some() || self.bar_editing {
            return;
        }

        // The platform's own modifier, so Cmd works on macOS without a second code path.
        let command = ui.input(|i| i.modifiers.command && !i.modifiers.alt);
        let shift = ui.input(|i| i.modifiers.shift);
        let pressed = |key: Key| command && ui.input(|i| i.key_pressed(key));

        if command {
            if pressed(Key::N) {
                self.run(Command::New);
            } else if pressed(Key::O) {
                self.run(Command::Open);
            } else if pressed(Key::S) {
                self.run(if shift { Command::SaveAs } else { Command::Save });
            } else if pressed(Key::Z) {
                self.run(if shift { Command::Redo } else { Command::Undo });
            } else if pressed(Key::Y) {
                self.run(Command::Redo);
            } else if pressed(Key::C) {
                self.copy();
            } else if pressed(Key::X) {
                self.copy();
                self.clear_selection();
            } else if pressed(Key::A) {
                self.run(Command::SelectUsedRange);
            }
        }

        // A paste arrives as an event carrying the text, so there is no clipboard API to call here.
        for event in ui.input(|i| i.events.clone()) {
            if let egui::Event::Paste(text) = event {
                self.paste(&text);
            }
        }
    }
}

/// A menu or keyboard command.
///
/// Public because it is how a host drives the window: every menu item is one of these, which is also what
/// makes the menu-only commands testable without a pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// An empty document.
    New,

    /// Read a file over this document.
    Open,

    /// Write the document back where it came from.
    Save,

    /// Write the document somewhere else.
    SaveAs,

    /// Close the window.
    Exit,

    /// Replay the most recent step backwards.
    Undo,

    /// Replay the most recently undone step.
    Redo,

    /// Copy the selection and empty it.
    Cut,

    /// Copy the selection to the clipboard.
    Copy,

    /// Paste what the clipboard already holds.
    Paste,

    /// Empty the selection.
    Clear,

    /// Select everything the sheet uses.
    SelectUsedRange,

    /// Apply a number format code to the selection, or `None` for General.
    NumberFormat(Option<&'static str>),

    /// Insert as many rows as the selection spans, above it.
    InsertRows,

    /// Insert as many columns as the selection spans, to its left.
    InsertColumns,

    /// Delete the rows the selection spans.
    DeleteRows,

    /// Delete the columns the selection spans.
    DeleteColumns,
}

/// The formats the Sheet menu offers, labelled with the format code itself — which is what a spreadsheet
/// shows too — and `None` for General. The list is the C# window's, unchanged.
const NUMBER_FORMATS: [(&str, Option<&'static str>); 12] = [
    ("General", None),
    ("0", Some("0")),
    ("0.00", Some("0.00")),
    ("#,##0", Some("#,##0")),
    ("#,##0.00", Some("#,##0.00")),
    ("0%", Some("0%")),
    ("0.0%", Some("0.0%")),
    ("$#,##0.00", Some("$#,##0.00")),
    ("0000", Some("0000")),
    ("yyyy-mm-dd", Some("yyyy-mm-dd")),
    ("yyyy-mm-dd hh:mm", Some("yyyy-mm-dd hh:mm")),
    ("hh:mm:ss", Some("hh:mm:ss")),
];

impl PgSheetsApp {
    /// Turns what the grid reported into an edit of the document.
    fn apply_action(&mut self, action: Option<GridAction>) {
        match action {
            Some(GridAction::Edit { row, column, seed }) => self.begin_edit(row, column, seed),

            Some(GridAction::SelectionChanged) => self.refresh(),

            Some(GridAction::SelectUsedRange) => {
                let cells = self.workbook.populated_cells();

                self.grid.layout_mut().select_used_range(cells.into_iter());
                self.refresh();
            }

            Some(GridAction::Clear) => self.clear_selection(),

            None => {}
        }
    }

    /// The name box and the formula bar.
    fn formula_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let selection = self.selection();
            let active = self.grid.layout().active_cell();

            // The name box shows the whole block for a range, the way a spreadsheet does.
            ui.label(egui::RichText::new(range_label(selection)).monospace());

            if self.formula_cell != Some(active) {
                // The cell underneath changed, so the bar follows it rather than keeping text that
                // belongs to another cell.
                self.formula_cell = Some(active);
                self.formula_text = self.workbook.raw_input(active.row, active.column).to_string();
            }

            if self.bar_editing {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.formula_text)
                        .id(egui::Id::new("formula-bar"))
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );

                if !response.has_focus() && !response.lost_focus() {
                    response.request_focus();
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    self.commit_bar(active);
                    self.bar_editing = false;
                }

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.bar_editing = false;
                    self.formula_cell = None;
                }
            } else {
                // A label, not a text box, so that the bar is not a focusable widget the grid's arrow keys
                // can be handed to. Clicking it is what makes it editable.
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(self.formula_text.clone()).monospace())
                        .sense(egui::Sense::click())
                        .truncate(),
                );

                if response.clicked() {
                    self.bar_editing = true;
                }
            }
        });
    }

    /// The status bar: a message on the left, and the aggregate of the selection on the right.
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status);

            // Right-aligned, which is where a spreadsheet puts the count/sum/average.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(&self.selection_text);
            });
        });
    }

    /// The one editor, positioned over the active cell.
    ///
    /// There is exactly one, and it never becomes a control per cell — the layout decides where it
    /// goes, which is why the grid exposes a cell's rectangle at all.
    ///
    /// Returns true when it closed the editor, because the host has to hand the keyboard back to the
    /// grid: without that the first key after a commit is swallowed while egui still thinks the text box
    /// has focus.
    fn edit_overlay(&mut self, ui: &mut egui::Ui, origin: egui::Pos2) -> bool {
        let Some(editor) = self.editor.as_ref() else {
            return false;
        };

        let (row, column, focused) = (editor.row, editor.column, editor.focused);
        let mut text = editor.text.clone();
        let rect = self.grid.cell_rect(origin, row, column);

        // These keys are taken before the text box sees them, so they mean what the C# window meant by
        // them rather than whatever the widget would do.
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.cancel_edit();

            return true;
        }

        if ui.input(|i| i.key_pressed(Key::Enter)) {
            self.commit_edit();
            self.grid.layout_mut().move_active_by(1, 0);

            return true;
        }

        // Tab commits and steps sideways, which is how a row of values is filled in without touching the
        // mouse — the C# handled it in the same place Enter was handled.
        if ui.input(|i| i.key_pressed(Key::Tab)) {
            let shift = ui.input(|i| i.modifiers.shift);

            self.commit_edit();
            self.grid
                .layout_mut()
                .move_active_by(0, if shift { -1 } else { 1 });

            return true;
        }

        let output = ui.put(
            rect,
            egui::TextEdit::singleline(&mut text).id(egui::Id::new("cell-editor")),
        );

        if let Some(editor) = self.editor.as_mut() {
            editor.text = text;
            editor.focused = true;
        }

        if !focused {
            output.request_focus();

            return false;
        }

        if output.lost_focus() {
            // A click elsewhere commits, which is what makes the grid usable without the keyboard.
            self.commit_edit();

            return true;
        }

        false
    }

    /// Opens the in-place editor on a cell, seeded with what the user typed or with the cell's content.
    fn begin_edit(&mut self, row: i32, column: i32, seed: Option<String>) {
        let text = match seed {
            Some(text) => text,
            None => self.workbook.raw_input(row, column).to_string(),
        };

        self.editor = Some(Editor {
            row,
            column,
            text,
            focused: false,
        });
    }

    /// Writes the editor's text through the model, which recalculates dependants.
    fn commit_edit(&mut self) {
        let Some(editor) = self.editor.take() else {
            return;
        };

        self.workbook
            .set_raw_input(editor.row, editor.column, &editor.text);
        self.after_edit();
    }

    /// Throws the editor's text away. Nothing was written, so nothing becomes unsaved.
    fn cancel_edit(&mut self) {
        self.editor = None;
        self.refresh();
    }

    /// Writes what the formula bar holds to the cell it belongs to.
    fn commit_bar(&mut self, active: CellAddress) {
        let text = self.formula_text.clone();

        self.workbook.set_raw_input(active.row, active.column, &text);
        self.after_edit();
    }

    /// A change reached the document: the summary has to be recomputed, and whether it is unsaved is
    /// decided by the revision rather than by the fact that something moved.
    fn after_edit(&mut self) {
        self.dirty = self.workbook.revision() != self.saved_revision;
        self.formula_cell = None;
        self.refresh();
    }

    /// Recomputes the aggregate shown beside the status message.
    ///
    /// The summary walks the populated cells rather than the selected rectangle, so a selection over a
    /// million empty cells costs nothing — and it is skipped entirely once a workbook is big enough
    /// that the walk would run on every step of a drag. The wording is the C# window's, down to the
    /// empty string for a single cell.
    fn refresh(&mut self) {
        const SUMMARY_CELL_LIMIT: usize = 20_000;

        self.formula_cell = None;

        let range = self.grid.layout().selection();

        if range.is_single_cell() {
            self.selection_text.clear();
            return;
        }

        let cells = range.row_count() * range.column_count();

        if self.workbook.cell_count() > SUMMARY_CELL_LIMIT {
            self.selection_text = format!("{} selected", thousands(i64::from(cells)));
            return;
        }

        let mut numbers = 0i32;
        let mut total = 0.0f64;

        for cell in self.workbook.populated_cells() {
            if !range.contains(cell.row, cell.column) {
                continue;
            }

            let value = self.workbook.value(cell.row, cell.column);

            // A number *value* only: a cell holding "5" as text is not counted, which is what the C#
            // window's summary did.
            if let CellValue::Number(number) = value {
                numbers += 1;
                total += number;
            }
        }

        if numbers == 0 {
            self.selection_text = format!("{} cells", thousands(i64::from(cells)));
            return;
        }

        self.selection_text = format!(
            "Count: {}    Sum: {}    Average: {}",
            thousands(i64::from(numbers)),
            pgsheets_core::number_format::apply_general(total),
            pgsheets_core::number_format::apply_general(total / f64::from(numbers))
        );
    }

    /// The document.
    pub fn workbook(&self) -> &Workbook {
        &self.workbook
    }

    /// True when the document has changed since it was last saved or opened.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// What the status bar is showing.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The aggregate of the selection, shown beside the status message.
    pub fn selection_text(&self) -> &str {
        &self.selection_text
    }

    /// The selected block.
    pub fn selection(&self) -> CellRange {
        self.grid.layout().selection()
    }

    /// What was last put on the clipboard, as text.
    pub fn clipboard_text(&self) -> &str {
        &self.clipboard_text
    }

    /// Runs a command, asking first when it would throw away unsaved changes.
    pub fn run(&mut self, command: Command) {
        if self.dirty {
            let message = match command {
                Command::New => "Save changes before starting a new spreadsheet?",

                Command::Open => "Save changes before opening another file?",

                Command::Exit => "Save changes before closing?",

                _ => {
                    self.apply(command);

                    return;
                }
            };

            let pending = match command {
                Command::Open => Pending::Open,
                Command::Exit => Pending::Exit,
                _ => Pending::New,
            };

            self.prompt = Some(Prompt::Unsaved {
                message: message.to_string(),
                pending,
            });

            return;
        }

        self.apply(command);
    }

    /// Runs a command that has already been cleared of its unsaved-changes question.
    fn apply(&mut self, command: Command) {
        match command {
            Command::New => self.new_document(),
            Command::Open => self.open_document(),
            Command::Save => {
                self.save();
            }

            Command::SaveAs => {
                self.save_as();
            }

            Command::Exit => self.exit_requested = true,

            // Undo and redo cancel the editor first: replaying a step underneath an open editor would
            // leave the two disagreeing about what the cell holds.
            Command::Undo | Command::Redo => {
                self.cancel_edit();

                let applied = if command == Command::Undo {
                    self.workbook.undo()
                } else {
                    self.workbook.redo()
                };

                if !applied {
                    self.status = if command == Command::Undo {
                        "Nothing to undo".to_string()
                    } else {
                        "Nothing to redo".to_string()
                    };

                    return;
                }

                self.after_edit();
                self.status = if command == Command::Undo {
                    "Undo".to_string()
                } else {
                    "Redo".to_string()
                };
            }

            Command::Copy => self.copy(),

            Command::Cut => {
                let range = self.selection();

                self.copy();
                self.clear_selection();
                self.status = format!("Cut {}", range_label(range));
            }

            Command::Paste => {
                let text = self.clipboard_text.clone();
                self.paste(&text);
            }

            Command::Clear => {
                self.clear_selection();
                self.status = "Cleared".to_string();
            }

            Command::SelectUsedRange => self.apply_action(Some(GridAction::SelectUsedRange)),

            Command::NumberFormat(code) => self.apply_number_format(code),
            Command::InsertRows => self.apply_structural_edit(true, true),
            Command::InsertColumns => self.apply_structural_edit(true, false),
            Command::DeleteRows => self.apply_structural_edit(false, true),
            Command::DeleteColumns => self.apply_structural_edit(false, false),
        }
    }

    /// Copies the selection as tab-separated **raw input**, which is the flavour of text Excel puts on the
    /// clipboard for a range: formulas survive a copy and paste inside the application.
    fn copy(&mut self) {
        let range = self.selection();

        let options = ExportOptions {
            range: Some(range),
            ..Default::default()
        };

        let mut result = ExportResult::new();

        self.clipboard_text = csv::build(&self.workbook, &options, '\t', &mut result);

        // Where it came from is remembered, because that and the text are what let a paste shift relative
        // references; the clipboard itself carries only text.
        self.copied_range = Some(range);
        self.clipboard_out = Some(self.clipboard_text.clone());

        self.status = format!("Copied {}", range_label(range));
    }

    /// Pastes delimited text at the selection, tiling it to cover the whole of it.
    ///
    /// Relative references shift only when the text is the block this application copied, because only
    /// then is its origin known. Text pasted from somewhere else keeps its references exactly as written.
    fn paste(&mut self, text: &str) {
        if text.is_empty() {
            self.status = "Clipboard is empty".to_string();

            return;
        }

        let mut block = CellBuffer::default();
        pgsheets_io::import::csv::parse(text, &mut block);

        if block.try_get_bounds().is_none() {
            return;
        }

        let target = self.selection();

        let (row_delta, column_delta) = match self.copied_range {
            Some(origin) if text == self.clipboard_text => sheet_paste::shift_for(origin, target),
            _ => (0, 0),
        };

        let (written, covered) =
            sheet_paste::apply(&block, &mut self.workbook, target, row_delta, column_delta);

        // Leave the pasted block selected, the way a spreadsheet does.
        self.grid.layout_mut().select_range(covered);
        self.after_edit();
        self.status = format!("Pasted {written} cell(s)");
    }

    /// Empties every populated cell in the selection, as one undo step.
    fn clear_selection(&mut self) {
        let range = self.selection();

        if range.is_single_cell() {
            let cell = self.grid.layout().active_cell();

            self.workbook.set_raw_input(cell.row, cell.column, "");
            self.after_edit();

            return;
        }

        // Gather first: writing while walking the populated cells would mutate what is being walked.
        let targets: Vec<CellAddress> = self
            .workbook
            .populated_cells()
            .into_iter()
            .filter(|cell| range.contains(cell.row, cell.column))
            .collect();

        if targets.is_empty() {
            return;
        }

        self.workbook.begin_bulk_update();

        for cell in targets {
            self.workbook.set_raw_input(cell.row, cell.column, "");
        }

        self.workbook.end_bulk_update();

        self.after_edit();
    }

    /// Applies a number format to the selection as one undo step.
    ///
    /// Only cells that already exist are touched: formatting a whole selected column would otherwise
    /// create a cell for every row of the sheet, which a sparse model must not do. A value typed into a
    /// formatted cell keeps its format, so the usual intent — format the column, then fill it in — still
    /// works.
    fn apply_number_format(&mut self, code: Option<&'static str>) {
        self.cancel_edit();

        let selection = self.selection();

        // Gathered first, because setting a format can add or remove cells and the walk cannot be over a
        // list that is changing underneath it.
        let targets: Vec<CellAddress> = self
            .workbook
            .populated_cells()
            .into_iter()
            .filter(|cell| selection.contains(cell.row, cell.column))
            .collect();

        let touched = targets.len();

        self.workbook.begin_bulk_update();

        for cell in targets {
            self.workbook.set_number_format(cell.row, cell.column, code);
        }

        self.workbook.end_bulk_update();

        self.after_edit();

        let where_ = range_label(selection);

        self.status = match code {
            None => format!("Cleared the number format on {touched} cell(s) in {where_}"),
            Some(code) => format!("Applied \"{code}\" to {touched} cell(s) in {where_}"),
        };
    }

    /// Inserts or removes as many rows or columns as the selection spans, starting at its first row or
    /// column. Every formula in the sheet is adjusted, and the whole thing is one undo step.
    fn apply_structural_edit(&mut self, insert: bool, rows: bool) {
        self.cancel_edit();

        let selection = self.selection();
        let at = if rows { selection.row1 } else { selection.column1 };
        let count = if rows {
            selection.row_count()
        } else {
            selection.column_count()
        };

        let applied = match (insert, rows) {
            (true, true) => self.workbook.insert_rows(at, count),
            (false, true) => self.workbook.delete_rows(at, count),
            (true, false) => self.workbook.insert_columns(at, count),
            (false, false) => self.workbook.delete_columns(at, count),
        };

        if !applied {
            self.status = "That would push data off the sheet".to_string();

            return;
        }

        // Leave the same block selected, now covering the fresh rows or columns.
        let covered = if rows {
            CellRange::new(at, selection.column1, at + count - 1, selection.column2)
        } else {
            CellRange::new(selection.row1, at, selection.row2, at + count - 1)
        };

        self.grid.layout_mut().select_range(covered);
        self.after_edit();

        let what = if rows { "row(s)" } else { "column(s)" };

        self.status = if insert {
            format!("Inserted {count} {what}")
        } else {
            format!("Deleted {count} {what}")
        };
    }

    /// An empty document. `clear` also drops the undo history, because there is nothing meaningful to
    /// return to.
    fn new_document(&mut self) {
        self.workbook.clear();

        self.path = None;
        self.worksheet_count = 1;
        self.sheet_name = "Sheet1".to_string();
        self.editor = None;
        self.grid.layout_mut().reset_view();
        self.saved_revision = self.workbook.revision();
        self.dirty = false;
        self.refresh();
        self.status = "New spreadsheet".to_string();
    }

    /// Reads a file, on a worker thread.
    ///
    /// The dialog is the blocking part; the reading is not, so the window keeps drawing while a large
    /// workbook arrives.
    fn open_document(&mut self) {
        let Some(path) = files::ask_open() else {
            return;
        };

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("the file");

        self.status = format!("Opening {name}...");
        self.opening = Some((path.clone(), files::spawn_open(&path)));
    }

    /// Applies a file that a worker has finished reading.
    ///
    /// The document is replaced wholesale, so the undo history goes with it: there is nothing meaningful
    /// to return to. The rows are played in inside one bulk update, so the engine recalculates once
    /// rather than once per cell.
    fn collect_opening(&mut self) {
        let Some((path, job)) = self.opening.as_ref() else {
            return;
        };

        let Some(outcome) = job.poll() else {
            return;
        };

        let path = path.clone();
        self.opening = None;

        match outcome {
            Ok(imported) => {
                self.workbook.clear();
                imported.write_into(&mut self.workbook);

                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("the file")
                    .to_string();

                self.sheet_name = imported
                    .result
                    .sheet_name
                    .clone()
                    .unwrap_or_else(|| "Sheet1".to_string());

                self.worksheet_count = imported.result.worksheet_count;
                self.path = Some(path);
                self.editor = None;
                self.grid.layout_mut().reset_view();
                self.saved_revision = self.workbook.revision();
                self.dirty = false;

                self.report(
                    format!(
                        "{name} - {} x {} cells imported ({} non-empty)",
                        imported.result.rows, imported.result.columns, imported.result.cells_imported
                    ),
                    &imported.result.warnings,
                );
            }

            // A failure to import is a message, not a modal: the document is untouched and the user may
            // simply be looking at the wrong file.
            Err(message) => self.status = format!("Import failed: {message}"),
        }
    }

    /// Writes the document back where it came from, or asks where to put it. True when it was written.
    fn save(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            return self.save_as();
        };

        // Sent to Save As rather than written in place when the document came from a file that held more
        // than one worksheet, or when the name is one this application cannot write — an .xlsm workbook,
        // for instance, whose VBA project it could not produce. A document this app cannot fully write is
        // never written over the file it came from.
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();

        if !SheetExporter::can_save_in_place(self.worksheet_count) || !files::can_export(&name) {
            return self.save_as();
        }

        self.write(&path)
    }

    /// Asks where to write, then writes. True when it was written.
    fn save_as(&mut self) -> bool {
        let suggested = self
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("Book1{}", SheetExporter::DEFAULT_EXTENSION));

        let Some(path) = files::ask_save_as(&suggested) else {
            return false;
        };

        // A picker can hand back a bare name when the chosen type implies the extension, so the default
        // one is appended unless the name is already something this application can write.
        let path = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if files::can_export(name) => path,
            _ => {
                let mut name = path.into_os_string();
                name.push(SheetExporter::DEFAULT_EXTENSION);

                PathBuf::from(name)
            }
        };

        self.write(&path)
    }

    /// Writes the document to a path, and remembers that the file now matches it.
    fn write(&mut self, path: &std::path::Path) -> bool {
        // Captured before the count is reset: the note about the worksheets a single-worksheet save left
        // behind is about the file that was *opened*.
        let lost = SheetExporter::describe_lost_worksheets(self.worksheet_count, &self.sheet_name);

        match files::save(path, &self.workbook, &self.sheet_name) {
            Ok(result) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("the file")
                    .to_string();

                self.path = Some(path.to_path_buf());
                self.saved_revision = self.workbook.revision();
                self.dirty = false;

                // The file on disk now holds exactly one worksheet, so later saves can be in place.
                self.worksheet_count = 1;

                let mut warnings = result.warnings.clone();

                if let Some(lost) = lost {
                    warnings.push(lost);
                }

                self.report(
                    format!(
                        "{name} - {} x {} written ({} non-empty)",
                        result.rows, result.columns, result.cells_written
                    ),
                    &warnings,
                );

                true
            }

            Err(message) => {
                self.status = format!("Save failed: {message}");

                false
            }
        }
    }

    /// Puts the outcome of a file operation in the status bar, and the warnings beside it.
    ///
    /// Warnings are counted in the message and then written one by one to stderr, which is the role the
    /// C# window's debug log played: a truncated range or an unsupported function has to be findable
    /// rather than swallowed.
    fn report(&mut self, summary: String, warnings: &[String]) {
        self.refresh();

        self.status = if warnings.is_empty() {
            summary
        } else {
            format!("{summary} - {} warning(s)", warnings.len())
        };

        for warning in warnings {
            eprintln!("{warning}");
        }
    }

    /// Answers the pending question, if there is one.
    ///
    /// A modal inside the window rather than a platform dialog, because the answer has three values and
    /// one of them has to be the safe default: dismissing the question means Cancel, so nothing is ever
    /// discarded without an explicit click.
    fn ask(&mut self, ui: &mut egui::Ui) {
        let Some(Prompt::Unsaved { message, pending }) = self.prompt.as_ref() else {
            return;
        };

        let message = message.clone();
        let pending = *pending;
        let mut choice = None;

        egui::Modal::new(egui::Id::new("save-prompt")).show(ui.ctx(), |ui| {
            ui.set_width(340.0);
            ui.label(message);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(SaveChoice::Save);
                }

                if ui.button("Don't Save").clicked() {
                    choice = Some(SaveChoice::Discard);
                }

                if ui.button("Cancel").clicked() {
                    choice = Some(SaveChoice::Cancel);
                }
            });
        });

        let Some(choice) = choice else {
            return;
        };

        self.prompt = None;

        match choice {
            SaveChoice::Cancel => {}

            SaveChoice::Discard => self.perform(pending),

            // A failed or cancelled save leaves the user exactly where they were.
            SaveChoice::Save => {
                if self.save() {
                    self.perform(pending);
                }
            }
        }
    }

    /// Carries out the action the unsaved-changes question interrupted.
    fn perform(&mut self, pending: Pending) {
        match pending {
            Pending::New => self.new_document(),
            Pending::Open => self.open_document(),
            Pending::Exit => {
                // The close was cancelled once already, so this one has to be allowed through.
                self.allow_close = true;
                self.exit_requested = true;
            }
        }
    }

    /// Keeps the window title in step with the document, and with whether it has unsaved changes.
    fn sync_title(&self, ui: &egui::Ui) {
        let name = self
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let marker = if self.dirty { "*" } else { "" };

        // "PGSheets - Name*", which is the C# window's title bar exactly.
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(format!("PGSheets - {name}{marker}")));
    }
}

impl eframe::App for PgSheetsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }
}

/// How a range reads in the name box and in a status message: `A1`, or `A1:B2` for a block.
fn range_label(range: CellRange) -> String {
    if range.is_single_cell() {
        return CellAddress::to_a1(range.row1, range.column1);
    }

    format!(
        "{}:{}",
        CellAddress::to_a1(range.row1, range.column1),
        CellAddress::to_a1(range.row2, range.column2)
    )
}

/// A count with thousands separators, which is the `N0` format the C# window used.
///
/// Written out rather than pulled from a crate: the numbers here are cell counts, so the only thing that
/// matters is that a big selection reads as "1,234,567" rather than a wall of digits.
fn thousands(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);

    if value < 0 {
        grouped.push('-');
    }

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }

        grouped.push(digit);
    }

    grouped
}
