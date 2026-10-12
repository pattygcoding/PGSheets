//! A test double for the formula engine.
//!
//! The workbook's invariants — sparsity, one mutation point, undo grouping, deferred recalculation —
//! are about *when* values are written, not about how a formula is evaluated. Standing in an engine
//! that understands literals and nothing else lets those be tested without a parser, which is the
//! same trick the C# suite's interfaces exist for. Formula behaviour has its own suite.

use pgsheets_core::store::CellStore;
use pgsheets_core::{CellAddress, CellValue, FormulaEngine, parse_literal};

/// Evaluates literals and leaves formulas empty.
#[derive(Default)]
pub struct LiteralEngine;

impl FormulaEngine for LiteralEngine {
    fn recalculate(&mut self, store: &mut dyn CellStore, changed: CellAddress) {
        let raw = store.raw_input(changed.row, changed.column).to_string();
        store.set_computed(changed.row, changed.column, evaluate(&raw));
    }

    fn rebuild(&mut self, store: &mut dyn CellStore) {
        for address in store.populated_cells() {
            let raw = store.raw_input(address.row, address.column).to_string();
            store.set_computed(address.row, address.column, evaluate(&raw));
        }
    }

    fn reset(&mut self) {}

    fn on_cell_removed(&mut self, _cell: CellAddress) {}

    fn precedents(&self, _cell: CellAddress) -> Vec<CellAddress> {
        Vec::new()
    }
}

fn evaluate(raw: &str) -> CellValue {
    if raw.starts_with('=') {
        CellValue::Empty
    } else {
        parse_literal(raw)
    }
}
