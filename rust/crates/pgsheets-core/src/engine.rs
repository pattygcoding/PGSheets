//! The formula calculation contract.

use crate::address::CellAddress;
use crate::store::CellStore;

/// The engine contract, kept intentionally small so an alternative engine — or a different
/// evaluation strategy — can be dropped in behind the same surface.
///
/// The store is passed to each call rather than being attached once. That is one deviation from the
/// C# shape, and it is the reason the Rust model needs no shared mutable state: the engine owns only
/// its caches and its dependency graph.
pub trait FormulaEngine {
    /// Recomputation entry point for an edit: recalculates the changed cell and every cell that
    /// depends on it.
    fn recalculate(&mut self, store: &mut dyn CellStore, changed: CellAddress);

    /// Recomputes the entire workbook. Used once after a bulk import.
    fn rebuild(&mut self, store: &mut dyn CellStore);

    /// Drops all cached state: parse trees and the dependency graph.
    fn reset(&mut self);

    /// Notifies the engine that a cell was deleted, so its edges can be detached.
    fn on_cell_removed(&mut self, cell: CellAddress);

    /// The cells a formula directly reads; empty for literals and unknown cells.
    fn precedents(&self, cell: CellAddress) -> Vec<CellAddress>;
}
