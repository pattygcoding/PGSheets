//! The engine over a store: what an edit propagates to, how a cycle is reported, and that every
//! function in the library can be called without the engine breaking.

use std::collections::HashMap;

use pgsheets_core::store::CellStore;
use pgsheets_core::{CellAddress, CellValue, parse_literal};
use pgsheets_formulas::{Engine, FunctionLibrary};

/// A store as the engine sees it: raw input in, computed values out, sparsity respected.
#[derive(Default)]
struct Store {
    cells: HashMap<CellAddress, CellValue>,
    raw: HashMap<CellAddress, String>,
}

impl Store {
    fn set(&mut self, row: i32, column: i32, raw: &str) {
        let address = CellAddress::new(row, column);
        self.raw.insert(address, raw.to_string());

        if raw.is_empty() {
            self.cells.remove(&address);
        } else {
            self.cells.entry(address).or_insert(CellValue::Empty);
        }
    }

    fn display(&self, row: i32, column: i32) -> String {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(CellValue::to_display_string)
            .unwrap_or_default()
    }
}

impl CellStore for Store {
    fn raw_input(&self, row: i32, column: i32) -> &str {
        self.raw
            .get(&CellAddress::new(row, column))
            .map(String::as_str)
            .unwrap_or("")
    }

    fn value(&self, row: i32, column: i32) -> CellValue {
        self.cells
            .get(&CellAddress::new(row, column))
            .cloned()
            .unwrap_or(CellValue::Empty)
    }

    fn set_computed(&mut self, row: i32, column: i32, value: CellValue) {
        // A cell that does not exist stays that way, which is what keeps a deleted cell deleted.
        if let Some(cell) = self.cells.get_mut(&CellAddress::new(row, column)) {
            *cell = value;
        }
    }

    fn populated_cells(&self) -> Vec<CellAddress> {
        self.cells.keys().copied().collect()
    }
}

/// Puts a raw value in a cell, then recalculates it, as the workbook does on an edit.
fn set(engine: &mut Engine, store: &mut Store, a1: &str, raw: &str) {
    let (row, column) = CellAddress::try_parse_a1(a1).unwrap();
    let address = CellAddress::new(row, column);

    store.set(row, column, raw);

    if raw.is_empty() {
        // The cell is gone, but whatever referenced it still has to be told the value changed: the
        // recalculation after a delete is what turns `=A1+1` into 1 rather than leaving 6 behind.
        engine.on_cell_removed(address);
    }

    engine.recalculate(store, address);
}

#[test]
fn an_edit_recalculates_what_depends_on_it() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "2");
    set(&mut engine, &mut store, "B1", "=A1*10");
    set(&mut engine, &mut store, "C1", "=B1+A1");

    assert_eq!(store.display(0, 1), "20");
    assert_eq!(store.display(0, 2), "22");

    // Changing the root of the chain propagates all the way down it.
    set(&mut engine, &mut store, "A1", "3");

    assert_eq!(store.display(0, 1), "30");
    assert_eq!(store.display(0, 2), "33");
}

#[test]
fn an_unrelated_edit_leaves_a_formula_alone() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "1");
    set(&mut engine, &mut store, "B1", "5");
    set(&mut engine, &mut store, "D1", "=B1+1");

    set(&mut engine, &mut store, "A1", "2");

    assert_eq!(store.display(0, 3), "6");
}

#[test]
fn a_cycle_is_reported_rather_than_hanging() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "=B1+1");
    set(&mut engine, &mut store, "B1", "=A1+1");

    // Re-entering a cell that is still being evaluated *is* the cycle, and it is reported as such.
    assert_eq!(store.display(0, 0), "#CIRCULAR!");
    assert_eq!(store.display(0, 1), "#CIRCULAR!");
}

#[test]
fn a_self_reference_is_a_cycle_too() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "=A1+1");

    assert_eq!(store.display(0, 0), "#CIRCULAR!");
}

#[test]
fn deleting_a_precedent_recalculates_its_dependants() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "5");
    set(&mut engine, &mut store, "B1", "=A1+1");

    assert_eq!(store.display(0, 1), "6");

    set(&mut engine, &mut store, "A1", "");

    assert_eq!(store.display(0, 1), "1");
}

#[test]
fn the_precedents_of_a_formula_are_the_cells_it_reads() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "1");
    set(&mut engine, &mut store, "B1", "1");
    set(&mut engine, &mut store, "C1", "=A1+B1+SUM(A1:B1)");

    let mut precedents = engine.precedents(CellAddress::new(0, 2));
    precedents.sort_unstable();
    precedents.dedup();

    assert_eq!(
        precedents,
        vec![CellAddress::new(0, 0), CellAddress::new(0, 1)],
        "a range contributes each of its cells, and the duplicates collapse"
    );

    assert!(engine.precedents(CellAddress::new(0, 0)).is_empty());
}

#[test]
fn only_the_branch_a_condition_selects_is_evaluated() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "=IF(FALSE,1/0,\"safe\")");
    set(&mut engine, &mut store, "A2", "=IF(TRUE,1/0,\"safe\")");
    set(&mut engine, &mut store, "A3", "=IFERROR(1/0,\"caught\")");

    assert_eq!(store.display(0, 0), "safe");
    assert_eq!(store.display(1, 0), "#DIV/0!");
    assert_eq!(store.display(2, 0), "caught");
}

#[test]
fn a_literal_becomes_a_typed_value() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "1200");

    assert_eq!(store.display(0, 0), "1200");
    assert_eq!(store.value(0, 0), CellValue::number(1200.0));
    assert_eq!(parse_literal("1200"), CellValue::number(1200.0));
}

#[test]
fn an_unknown_function_surfaces_as_a_name_error() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "=NOPE(1)");

    assert_eq!(store.display(0, 0), "#NAME?");
}

#[test]
fn a_rebuild_recomputes_the_whole_sheet() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    store.set(0, 0, "2");
    store.set(0, 1, "=A1*3");

    engine.rebuild(&mut store);

    assert_eq!(store.display(0, 1), "6");
}

#[test]
fn a_function_name_is_matched_whatever_its_case() {
    let mut engine = Engine::new();
    let mut store = Store::default();

    set(&mut engine, &mut store, "A1", "=sum(1,2)");

    assert_eq!(store.display(0, 0), "3");
}

/// The sweep the C# suite performs over its own library: every registered function has to answer
/// rather than panic, whatever it is handed. A missing argument check shows up here rather than as a
/// crash in front of a user.
#[test]
fn every_function_survives_being_misused() {
    let library = FunctionLibrary::standard();
    let mut names: Vec<&str> = library.names().collect();
    names.sort_unstable();

    assert!(names.len() >= 83, "the library shrank: {} functions", names.len());

    for name in names {
        for formula in [
            format!("={name}()"),
            format!("={name}(1,2,3,4,5,6,7,8,9,10)"),
            format!("={name}(\"text\")"),
            format!("={name}(A1:B2)"),
            format!("={name}(1/0)"),
        ] {
            let mut engine = Engine::new();
            let mut store = Store::default();

            set(&mut engine, &mut store, "A1", "1");
            set(&mut engine, &mut store, "A2", "2");
            set(&mut engine, &mut store, "B1", "3");
            set(&mut engine, &mut store, "B2", "4");
            set(&mut engine, &mut store, "C1", &formula);

            let display = store.display(0, 2);

            if display.is_empty() {
                // A text-joining function with nothing to join answers with empty text, which is a
                // value rather than a failure. Nothing else may answer with nothing at all.
                assert!(
                    name.starts_with("CONCAT"),
                    "{formula} produced nothing, which only a text join may do"
                );
            }

            assert_ne!(
                display, "#NAME?",
                "{name} is registered but the engine could not resolve it"
            );
        }
    }
}
