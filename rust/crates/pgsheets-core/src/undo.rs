//! Records cell edits so they can be replayed backwards and forwards.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::style::CellStyle;

/// Most steps kept. Older ones fall off the end: undo history is a convenience, not a document.
const MAX_STEPS: usize = 200;

/// One cell's change within a step: its input, its number format and its style, before and after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellChange {
    /// Zero-based row.
    pub row: i32,

    /// Zero-based column.
    pub column: i32,

    /// The raw input before the change.
    pub before: String,

    /// The raw input after the change.
    pub after: String,

    /// The number format before the change.
    pub before_format: Option<String>,

    /// The number format after the change.
    pub after_format: Option<String>,

    /// The style before the change.
    pub before_style: CellStyle,

    /// The style after the change.
    pub after_style: CellStyle,
}

impl CellChange {
    /// Creates a change record for a value and its number format, with no change of style.
    ///
    /// Styles go on afterwards with [`CellChange::with_style`], because most edits do not touch them.
    pub fn new(
        row: i32,
        column: i32,
        before: impl Into<String>,
        after: impl Into<String>,
        before_format: Option<String>,
        after_format: Option<String>,
    ) -> Self {
        Self {
            row,
            column,
            before: before.into(),
            after: after.into(),
            before_format,
            after_format,
            before_style: CellStyle::DEFAULT,
            after_style: CellStyle::DEFAULT,
        }
    }

    /// The same change, with the style it also carried, before and after.
    pub fn with_style(mut self, before_style: CellStyle, after_style: CellStyle) -> Self {
        self.before_style = before_style;
        self.after_style = after_style;
        self
    }
}

/// Records cell edits so they can be replayed backwards and forwards.
///
/// Edits are grouped: everything between [`UndoStack::begin_group`] and [`UndoStack::end_group`]
/// becomes one undo step, which is what makes clearing or pasting a whole range undo in a single
/// action. Inside a group each cell is recorded once, keeping its first "before" and its last
/// "after".
///
/// Unlike the C# original, the stack does not hold the callback that applies a change: it hands the
/// changes back to the caller instead. A Rust field cannot borrow its owner mutably, and inverting
/// the control is what keeps the model free of shared mutable state.
pub struct UndoStack {
    undo: Vec<Vec<CellChange>>,
    redo: Vec<Vec<CellChange>>,
    group: Option<Vec<CellChange>>,
    group_index: HashMap<(i32, i32), usize>,
    group_depth: u32,

    /// Suspension is shared rather than borrowed, so the caller can keep editing the model while the
    /// scope is open — which is the whole point of suspending.
    suppressed: Rc<Cell<u32>>,

    /// Bumped whenever the availability of undo or redo changes, for hosts that need to know.
    revision: u64,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    /// Creates an empty history.
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            group: None,
            group_index: HashMap::new(),
            group_depth: 0,
            suppressed: Rc::new(Cell::new(0)),
            revision: 0,
        }
    }

    /// Bumped whenever the availability of undo or redo changes.
    ///
    /// This is the Rust spelling of the C# `Changed` event: a host that wants to know whether the
    /// Undo menu item should be enabled compares this against the value it saw last.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True when there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True when there is something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of undo steps held.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Opens a group; nested calls share the outermost one.
    pub fn begin_group(&mut self) {
        self.group_depth += 1;
    }

    /// Closes a group, committing it as one step. A group with no net change is dropped.
    pub fn end_group(&mut self) {
        if self.group_depth == 0 {
            return;
        }

        self.group_depth -= 1;

        if self.group_depth > 0 {
            return;
        }

        let group = self.group.take();
        self.group_index.clear();

        if let Some(changes) = group {
            self.commit(changes);
        }
    }

    /// Notes one cell change: its raw input, its number format and its style.
    ///
    /// No-ops when none of the three actually changed, so applying the same format twice is not an
    /// undo step.
    pub fn record(&mut self, change: CellChange) {
        let raw_unchanged = change.before == change.after;
        let format_unchanged = change.before_format == change.after_format;
        let style_unchanged = change.before_style == change.after_style;

        if self.is_suspended() || (raw_unchanged && format_unchanged && style_unchanged) {
            return;
        }

        if self.group_depth == 0 {
            self.commit(vec![change]);
            return;
        }

        let group = self.group.get_or_insert_with(Vec::new);

        if let Some(index) = self.group_index.get(&(change.row, change.column)).copied() {
            // The same cell twice in one step: keep the first "before" and the last "after".
            group[index].after = change.after;
            group[index].after_format = change.after_format;
            group[index].after_style = change.after_style;
            return;
        }

        self.group_index.insert((change.row, change.column), group.len());
        group.push(change);
    }

    /// The changes to replay backwards, in the order they have to be applied, or `None` when there is
    /// nothing to undo. The step moves to the redo stack either way; the caller applies the changes.
    pub fn undo(&mut self) -> Option<Vec<CellChange>> {
        let step = self.undo.pop()?;

        // Reverse order, so a cell touched twice inside one step ends on its earliest value.
        let reversed = step.iter().rev().cloned().collect();

        self.redo.push(step);
        self.revision += 1;
        Some(reversed)
    }

    /// The changes to replay forwards, in the order they have to be applied, or `None` when there is
    /// nothing to redo. The caller applies the changes.
    pub fn redo(&mut self) -> Option<Vec<CellChange>> {
        let step = self.redo.pop()?;
        self.undo.push(step.clone());
        self.revision += 1;
        Some(step)
    }

    /// Drops all history, e.g. when the document is replaced wholesale.
    pub fn clear(&mut self) {
        let had_history = !self.undo.is_empty() || !self.redo.is_empty();

        self.undo.clear();
        self.redo.clear();
        self.group = None;
        self.group_index.clear();

        if had_history {
            self.revision += 1;
        }
    }

    /// Suspends recording until the returned scope is dropped.
    ///
    /// Used by work that replaces the document rather than editing it, where there is nothing
    /// meaningful to undo back to.
    pub fn suspend(&self) -> UndoSuspension {
        self.suppressed.set(self.suppressed.get() + 1);

        UndoSuspension {
            suppressed: Rc::clone(&self.suppressed),
        }
    }

    /// True while recording is suspended.
    pub fn is_suspended(&self) -> bool {
        self.suppressed.get() > 0
    }

    fn commit(&mut self, step: Vec<CellChange>) {
        self.undo.push(step);
        self.redo.clear();

        while self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }

        self.revision += 1;
    }
}

/// Keeps undo recording suspended for as long as it is alive.
pub struct UndoSuspension {
    suppressed: Rc<Cell<u32>>,
}

impl Drop for UndoSuspension {
    fn drop(&mut self) {
        self.suppressed.set(self.suppressed.get().saturating_sub(1));
    }
}
