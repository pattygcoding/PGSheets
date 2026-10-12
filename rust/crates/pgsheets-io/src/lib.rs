//! Reader and writer of the SpreadsheetML (xlsx) container and of delimited text.
//!
//! Both formats are written from scratch: the OOXML parts, content types, relationships, style table
//! and cell serialisation are all here, and no third-party spreadsheet package is involved. The two
//! crates in use provide only the container (`zip`) and the XML tokenizer (`quick-xml`), neither of
//! which `std` has.

pub mod capped_warnings;
pub mod export;
pub mod import;
pub mod ooxml;
pub mod xml;
pub mod zip;

pub use export::{ExportOptions, ExportResult, SheetExporter, SpreadsheetExporter};
pub use import::{ImportError, ImportResult, ImporterRegistry, SpreadsheetImporter};
