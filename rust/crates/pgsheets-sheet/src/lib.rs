//! The sparse workbook: one mutation point, undo history and a paste that follows formula references.
//!
//! This crate sits between the formula engine and the application. It is the only place that writes
//! cell values, which is why every change to a sheet — including an undo — goes through
//! [`Workbook::set_raw_input`] and its siblings.

pub mod sheet_paste;
pub mod workbook;

pub use sheet_paste::{apply as paste_into, shift_for};
pub use workbook::Workbook;
