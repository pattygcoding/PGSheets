//! The workbook: addressing, values, the sparse model, undo history, number formats, serial dates
//! and clipboard blocks.
//!
//! This crate holds no UI types and no file-format code, so everything in it is reachable from a
//! headless test. The two contracts that keep it that way are [`CellStore`] (what the formula engine
//! may read and write) and [`ExportSource`] (the read-only view a writer gets).

pub mod address;
pub mod bounds;
pub mod cell_buffer;
pub mod cell_data;
pub mod engine;
pub mod literals;
pub mod number_format;
pub mod numeric;
pub mod range;
pub mod reference;
pub mod serial_dates;
pub mod store;
pub mod undo;
pub mod value;

pub use address::{CellAddress, ParsedAddress};
pub use bounds::{MAX_COLUMNS, MAX_ROWS};
pub use cell_buffer::CellBuffer;
pub use cell_data::CellData;
pub use engine::FormulaEngine;
pub use literals::parse_literal;
pub use range::CellRange;
pub use reference::CellReference;
pub use serial_dates::CivilDateTime;
pub use store::{CellStore, ExportSource, ImportSink};
pub use undo::{CellChange, UndoStack, UndoSuspension};
pub use value::{CellError, CellValue};
