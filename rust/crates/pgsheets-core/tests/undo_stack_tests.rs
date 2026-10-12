//! The undo history, exercised over a plain dictionary so these tests need no spreadsheet.
//!
//! The C# stack replays changes through a callback. This one hands them back instead, so the harness
//! below applies them — which is exactly what the workbook does in the application.

use std::collections::HashMap;

use pgsheets_core::{CellChange, UndoStack};

/// A tiny stand-in for a sheet: values and number formats, keyed by coordinate.
struct Target {
    cells: HashMap<(i32, i32), String>,
    formats: HashMap<(i32, i32), Option<String>>,
    stack: UndoStack,
}

impl Target {
    fn new() -> Self {
        Self {
            cells: HashMap::new(),
            formats: HashMap::new(),
            stack: UndoStack::new(),
        }
    }

    fn get(&self, row: i32, column: i32) -> &str {
        self.cells.get(&(row, column)).map(String::as_str).unwrap_or("")
    }

    fn get_format(&self, row: i32, column: i32) -> Option<String> {
        self.formats.get(&(row, column)).cloned().flatten()
    }

    /// Simulates a user edit: apply the value, then record it.
    fn edit(&mut self, row: i32, column: i32, value: &str) {
        let before = self.get(row, column).to_string();
        let format = self.get_format(row, column);

        self.set(row, column, value, format.clone());
        self.stack.record(CellChange::new(
            row,
            column,
            before,
            value,
            format.clone(),
            format,
        ));
    }

    /// Simulates applying a number format, which leaves the value alone.
    fn format(&mut self, row: i32, column: i32, number_format: Option<&str>) {
        let value = self.get(row, column).to_string();
        let before = self.get_format(row, column);
        let after = number_format.map(str::to_string);

        self.set(row, column, &value, after.clone());
        self.stack
            .record(CellChange::new(row, column, &value, &value, before, after));
    }

    fn undo(&mut self) -> bool {
        let Some(changes) = self.stack.undo() else {
            return false;
        };

        for change in changes {
            self.set(change.row, change.column, &change.before, change.before_format);
        }

        true
    }

    fn redo(&mut self) -> bool {
        let Some(changes) = self.stack.redo() else {
            return false;
        };

        for change in changes {
            self.set(change.row, change.column, &change.after, change.after_format);
        }

        true
    }

    fn set(&mut self, row: i32, column: i32, value: &str, number_format: Option<String>) {
        self.cells.insert((row, column), value.to_string());
        self.formats.insert((row, column), number_format);
    }
}

#[test]
fn starts_empty() {
    let target = Target::new();

    assert!(!target.stack.can_undo());
    assert!(!target.stack.can_redo());
    assert_eq!(target.stack.undo_depth(), 0);
}

#[test]
fn undo_restores_the_previous_value() {
    let mut target = Target::new();
    target.edit(0, 0, "first");
    target.edit(0, 0, "second");

    assert!(target.undo());
    assert_eq!(target.get(0, 0), "first");

    assert!(target.undo());
    assert_eq!(target.get(0, 0), "");

    assert!(!target.undo());
}

#[test]
fn redo_reapplies_the_value() {
    let mut target = Target::new();
    target.edit(0, 0, "value");
    target.undo();

    assert!(target.redo());
    assert_eq!(target.get(0, 0), "value");
    assert!(!target.redo());
}

#[test]
fn a_new_edit_discards_the_redo_history() {
    let mut target = Target::new();
    target.edit(0, 0, "first");
    target.undo();

    assert!(target.stack.can_redo());

    target.edit(0, 0, "second");

    assert!(!target.stack.can_redo());
    assert_eq!(target.get(0, 0), "second");
}

#[test]
fn a_group_undoes_as_one_step() {
    let mut target = Target::new();

    target.stack.begin_group();
    target.edit(0, 0, "a");
    target.edit(1, 0, "b");
    target.edit(2, 0, "c");
    target.stack.end_group();

    assert_eq!(target.stack.undo_depth(), 1);

    target.undo();

    assert_eq!(target.get(0, 0), "");
    assert_eq!(target.get(1, 0), "");
    assert_eq!(target.get(2, 0), "");
}

#[test]
fn a_group_records_each_cell_once() {
    let mut target = Target::new();

    target.stack.begin_group();
    target.edit(0, 0, "a");
    target.edit(0, 0, "b");
    target.edit(0, 0, "c");
    target.stack.end_group();

    target.undo();

    // One step, and it returns to the value from before the group, not to "b".
    assert_eq!(target.get(0, 0), "");
    assert!(!target.stack.can_undo());
}

#[test]
fn repeating_a_cell_inside_a_group_replays_the_last_value() {
    let mut target = Target::new();

    target.stack.begin_group();
    target.edit(0, 0, "a");
    target.edit(0, 0, "b");
    target.stack.end_group();

    target.undo();
    assert_eq!(target.get(0, 0), "");

    target.redo();
    assert_eq!(target.get(0, 0), "b");
}

#[test]
fn nested_groups_still_produce_one_step() {
    let mut target = Target::new();

    target.stack.begin_group();
    target.stack.begin_group();
    target.edit(0, 0, "a");
    target.stack.end_group();
    target.edit(1, 0, "b");
    target.stack.end_group();

    assert_eq!(target.stack.undo_depth(), 1);
}

#[test]
fn an_empty_group_is_not_a_step() {
    let mut target = Target::new();

    target.stack.begin_group();
    target.stack.end_group();

    assert!(!target.stack.can_undo());
}

#[test]
fn an_unbalanced_end_group_is_harmless() {
    let mut target = Target::new();

    target.stack.end_group();

    assert!(!target.stack.can_undo());
}

#[test]
fn identical_values_are_not_recorded() {
    let mut target = Target::new();

    target
        .stack
        .record(CellChange::new(0, 0, "same", "same", None, None));

    assert!(!target.stack.can_undo());
}

#[test]
fn a_number_format_change_is_its_own_step() {
    let mut target = Target::new();
    target.edit(0, 0, "1234");

    target.format(0, 0, Some("0.00"));

    assert!(target.stack.can_undo());
    assert_eq!(target.get_format(0, 0), Some("0.00".to_string()));

    // Undoing the format leaves the value untouched.
    target.undo();

    assert_eq!(target.get_format(0, 0), None);
    assert_eq!(target.get(0, 0), "1234");

    target.redo();

    assert_eq!(target.get_format(0, 0), Some("0.00".to_string()));
    assert_eq!(target.get(0, 0), "1234");
}

#[test]
fn applying_the_same_format_twice_is_not_a_step() {
    let mut target = Target::new();
    target.edit(0, 0, "1");
    target.format(0, 0, Some("0.00"));

    let depth = target.stack.undo_depth();

    target.format(0, 0, Some("0.00"));

    assert_eq!(target.stack.undo_depth(), depth);
}

#[test]
fn suspend_stops_recording_until_the_scope_ends() {
    let mut target = Target::new();

    {
        let _suspension = target.stack.suspend();
        target.edit(0, 0, "ignored");
    }

    assert!(!target.stack.can_undo());
    assert_eq!(target.get(0, 0), "ignored");

    target.edit(1, 0, "recorded");
    assert!(target.stack.can_undo());
}

#[test]
fn nested_suspensions_keep_recording_off_until_the_last_one_ends() {
    let mut target = Target::new();

    {
        let _outer = target.stack.suspend();

        {
            let _inner = target.stack.suspend();
            target.edit(0, 0, "ignored");
        }

        assert!(target.stack.is_suspended());
        target.edit(1, 0, "still ignored");
    }

    assert!(!target.stack.is_suspended());
    assert!(!target.stack.can_undo());
}

#[test]
fn clear_drops_the_history() {
    let mut target = Target::new();
    target.edit(0, 0, "value");

    target.stack.clear();

    assert!(!target.stack.can_undo());
    assert!(!target.stack.can_redo());
}

#[test]
fn the_revision_advances_once_per_step() {
    let mut target = Target::new();
    let start = target.stack.revision();

    target.edit(0, 0, "value");
    assert_eq!(target.stack.revision(), start + 1);

    target.undo();
    assert_eq!(target.stack.revision(), start + 2);
}

#[test]
fn the_history_is_bounded() {
    let mut target = Target::new();

    for index in 0..250 {
        target.edit(0, 0, &index.to_string());
    }

    assert_eq!(target.stack.undo_depth(), 200);
}
