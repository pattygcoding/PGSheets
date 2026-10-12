//! Reading a spreadsheet from a file.
//!
//! An importer pushes raw input into an [`ImportSink`] and never touches the UI, which is what keeps
//! both formats unit-testable without a window.

pub mod csv;
mod result;
mod xlsx;

use pgsheets_core::store::ImportSink;

use crate::zip::ReadSeek;

pub use csv::CsvImporter;
pub use result::ImportResult;
pub use xlsx::XlsxImporter;

/// Why an import could not begin at all.
///
/// Everything an importer can shrug off becomes a warning in the [`ImportResult`] instead; this is
/// only for a file it cannot make sense of.
#[derive(Debug)]
pub enum ImportError {
    /// The file is not a zip container, or the archive is damaged.
    Container(String),

    /// A part the document needs is not the XML it claims to be.
    Malformed(String),
}

impl core::fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Container(message) => write!(formatter, "the file is not a readable workbook: {message}"),
            Self::Malformed(message) => write!(formatter, "the workbook is damaged: {message}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// A file format reader.
pub trait SpreadsheetImporter {
    /// Lower-case extensions this importer claims, e.g. `".csv"`.
    fn extensions(&self) -> &'static [&'static str];

    /// True when this importer claims a file name.
    fn can_import(&self, file_name: &str) -> bool {
        let Some(extension) = extension_of(file_name) else {
            return false;
        };

        self.extensions().iter().any(|claimed| *claimed == extension)
    }

    /// Reads the file, pushing what it finds into `sink`.
    fn import(
        &self,
        reader: &mut dyn ReadSeek,
        sink: &mut dyn ImportSink,
    ) -> Result<ImportResult, ImportError>;
}

/// The lower-case extension of a file name, including its dot.
pub(crate) fn extension_of(file_name: &str) -> Option<String> {
    let name = std::path::Path::new(file_name).extension()?;
    Some(format!(".{}", name.to_string_lossy().to_lowercase()))
}

/// Chooses an importer for a file name.
///
/// Registering another format is one line here, which is the whole point of the indirection: a new
/// reader grows the import story without the window knowing about it.
pub struct ImporterRegistry {
    importers: Vec<Box<dyn SpreadsheetImporter>>,
}

impl Default for ImporterRegistry {
    fn default() -> Self {
        Self::standard()
    }
}

impl ImporterRegistry {
    /// The formats this application reads.
    pub fn standard() -> Self {
        Self {
            importers: vec![Box::new(XlsxImporter::new()), Box::new(CsvImporter::new())],
        }
    }

    /// An empty registry, for a host that wants to decide the formats itself.
    pub fn empty() -> Self {
        Self {
            importers: Vec::new(),
        }
    }

    /// Adds an importer, which takes precedence over the ones already registered.
    pub fn register(&mut self, importer: Box<dyn SpreadsheetImporter>) {
        self.importers.insert(0, importer);
    }

    /// The importer that claims a file name, or `None`.
    pub fn resolve(&self, file_name: &str) -> Option<&dyn SpreadsheetImporter> {
        self.importers
            .iter()
            .find(|importer| importer.can_import(file_name))
            .map(AsRef::as_ref)
    }

    /// True when some importer claims a file name.
    pub fn can_import(&self, file_name: &str) -> bool {
        self.resolve(file_name).is_some()
    }

    /// Reads a file, or `None` when no importer claims it.
    pub fn import(
        &self,
        file_name: &str,
        reader: &mut dyn ReadSeek,
        sink: &mut dyn ImportSink,
    ) -> Option<Result<ImportResult, ImportError>> {
        self.resolve(file_name)
            .map(|importer| importer.import(reader, sink))
    }

    /// File-picker patterns for every registered importer.
    pub fn patterns(&self) -> Vec<String> {
        let mut patterns: Vec<String> = self
            .importers
            .iter()
            .flat_map(|importer| importer.extensions())
            .map(|extension| format!("*{extension}"))
            .collect();

        patterns.sort();
        patterns.dedup();
        patterns
    }
}
