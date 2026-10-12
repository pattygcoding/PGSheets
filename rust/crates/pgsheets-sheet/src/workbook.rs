//! The sparse workbook. Cells are stored only when they have content, so an "empty" grid costs
//! nothing. Raw input and computed output are kept side by side: editing writes the raw input and
//! the formula engine writes the result back.

use std::collections::HashMap;

use pgsheets_core::cell_data::CellData;
use pgsheets_core::store::{CellStore, ExportSource, ImportSink};
use pgsheets_core::undo::{CellChange, UndoStack, UndoSuspension};
use pgsheets_core::{CellAddress, CellStyle, CellValue, FormulaEngine, MAX_COLUMNS, MAX_ROWS};
use pgsheets_formulas::rewriter;

/// A listener told that the sheet changed.
///
/// This is the Rust spelling of the C# `Changed` event. A host uses it to repaint or to mark the
/// window dirty; a test uses it to count notifications.
type ChangeListener = Box<dyn FnMut()>;

/// One cell as a structural edit sees it: its raw input, the number format and the style that go with
/// it.
struct CellContent {
    raw: String,
    format: Option<String>,
    style: CellStyle,
}

/// The sparse workbook.
pub struct Workbook {
    cells: HashMap<CellAddress, CellData>,
    engine: Box<dyn FormulaEngine>,
    undo: UndoStack,

    bulk_depth: u32,
    pending_rebuild: bool,

    listeners: Vec<ChangeListener>,

    /// Bumped on every change that a host would see, so a host can notice one without a listener.
    revision: u64,
}

impl Workbook {
    /// Creates an empty workbook around an engine.
    pub fn new(engine: Box<dyn FormulaEngine>) -> Self {
        Self {
            cells: HashMap::new(),
            engine,
            undo: UndoStack::new(),
            bulk_depth: 0,
            pending_rebuild: false,
            listeners: Vec::new(),
            revision: 0,
        }
    }

    /// Registers a listener that is told whenever content or computed values change.
    pub fn on_changed(&mut self, listener: impl FnMut() + 'static) {
        self.listeners.push(Box::new(listener));
    }

    /// Bumped on every change a host would see.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Number of populated cells, which is not the same as the grid's extent.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// True when the sheet holds nothing at all.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The stored cell record, or `None` when the cell is not populated.
    pub fn cell(&self, row: i32, column: i32) -> Option<&CellData> {
        self.cells.get(&CellAddress::new(row, column))
    }

    /// Text to render for a cell; empty when the cell is absent.
    pub fn display_value(&self, row: i32, column: i32) -> &str {
        self.cell(row, column).map(CellData::display_value).unwrap_or("")
    }

    /// The raw input of a cell; empty when the cell is absent.
    pub fn raw_input(&self, row: i32, column: i32) -> &str {
        self.cell(row, column).map(CellData::raw_input).unwrap_or("")
    }

    /// The computed value of a cell.
    pub fn value(&self, row: i32, column: i32) -> CellValue {
        self.cell(row, column)
            .map(|cell| cell.value().clone())
            .unwrap_or(CellValue::Empty)
    }

    /// The number format code of a cell, or `None` when it is General.
    ///
    /// A cell can hold a format with no value: formatting reaches an empty cell just as it does in a
    /// spreadsheet.
    pub fn number_format(&self, row: i32, column: i32) -> Option<&str> {
        self.cell(row, column).and_then(CellData::number_format)
    }

    /// The styling of a cell, or `None` when it has none.
    ///
    /// `None` rather than a default style, so a caller can tell "nothing has been formatted here" from
    /// "this cell was deliberately given the default look".
    pub fn cell_style(&self, row: i32, column: i32) -> Option<CellStyle> {
        self.cell(row, column)
            .map(CellData::style)
            .filter(|style| !style.is_default())
    }

    /// Every populated cell, in no particular order.
    pub fn populated_cells(&self) -> Vec<CellAddress> {
        self.cells.keys().copied().collect()
    }

    /// True when there is something to undo.
    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    /// True when there is something to redo.
    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// Number of undo steps held.
    pub fn undo_depth(&self) -> usize {
        self.undo.undo_depth()
    }

    /// Replays the most recent step backwards. False when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(changes) = self.undo.undo() else {
            return false;
        };

        self.begin_bulk_update();

        for change in changes {
            self.apply_cell(
                change.row,
                change.column,
                &change.before,
                change.before_format,
                change.before_style,
            );
        }

        self.end_bulk_update();
        true
    }

    /// Replays the most recently undone step. False when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(changes) = self.undo.redo() else {
            return false;
        };

        self.begin_bulk_update();

        for change in changes {
            self.apply_cell(
                change.row,
                change.column,
                &change.after,
                change.after_format,
                change.after_style,
            );
        }

        self.end_bulk_update();
        true
    }

    /// Suspends undo recording until the returned scope is dropped.
    ///
    /// Used when the document is replaced wholesale — opening a file, starting a new one — where
    /// there is nothing meaningful to undo back to.
    pub fn suspend_undo_history(&self) -> UndoSuspension {
        self.undo.suspend()
    }

    /// Stores raw input for a cell, keeping the number format and style it already has, recalculates, and
    /// records an undo step.
    ///
    /// An empty string removes the cell, unless a format or a style is attached to it.
    pub fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str) {
        // The previous state has to be read *before* the mutation: applying a value writes through the
        // very record a caller holds, so reading it afterwards would record "no change".
        let previous = self.raw_input(row, column).to_string();

        if previous == raw_input {
            return;
        }

        let format = self.number_format(row, column).map(str::to_string);
        let style = self.cell(row, column).map(CellData::style).unwrap_or_default();

        // Typing a value does not change the way that value is displayed, nor how it looks.
        self.apply_cell(row, column, raw_input, format.clone(), style);
        self.undo.record(
            CellChange::new(row, column, previous, raw_input, format.clone(), format)
                .with_style(style, style),
        );
    }

    /// Sets the number format of a cell and records one undo step.
    ///
    /// A blank format means General; clearing the format of a cell that holds no value and no style
    /// removes the cell entirely.
    pub fn set_number_format(&mut self, row: i32, column: i32, number_format: Option<&str>) {
        let format = number_format
            .filter(|code| !code.trim().is_empty())
            .map(str::to_string);

        let Some(cell) = self.cell(row, column) else {
            if format.is_none() {
                return;
            }

            // A format on its own is still worth keeping: it is what a value typed later shows as.
            self.apply_cell(row, column, "", format.clone(), CellStyle::DEFAULT);
            self.undo
                .record(CellChange::new(row, column, "", "", None, format));
            return;
        };

        if cell.number_format() == format.as_deref() {
            return;
        }

        // Read the previous state before mutating, for the same reason as above.
        let raw = cell.raw_input().to_string();
        let style = cell.style();
        let previous_format = cell.number_format().map(str::to_string);

        self.apply_cell(row, column, &raw, format.clone(), style);
        self.undo.record(
            CellChange::new(row, column, &raw, &raw, previous_format, format).with_style(style, style),
        );
    }

    /// Sets the styling of a cell and records one undo step.
    ///
    /// A style with no value and no number format removes the cell; a style applied to an empty cell
    /// keeps it, because formatting is remembered for whatever is typed next.
    pub fn set_cell_style(&mut self, row: i32, column: i32, style: CellStyle) {
        let style = if style.is_default() {
            CellStyle::DEFAULT
        } else {
            style
        };

        let Some(cell) = self.cell(row, column) else {
            if style.is_default() {
                return;
            }

            // A style on its own is still worth keeping: it is what a value typed later looks like.
            self.apply_cell(row, column, "", None, style);
            self.undo.record(
                CellChange::new(row, column, "", "", None, None).with_style(CellStyle::DEFAULT, style),
            );
            return;
        };

        if cell.style() == style {
            return;
        }

        let raw = cell.raw_input().to_string();
        let format = cell.number_format().map(str::to_string);
        let previous_style = cell.style();

        self.apply_cell(row, column, &raw, format.clone(), style);
        self.undo.record(
            CellChange::new(row, column, &raw, &raw, format.clone(), format)
                .with_style(previous_style, style),
        );
    }

    /// Applies a raw value, its number format and its style without touching the undo history.
    ///
    /// This is the single mutation point, shared by the setters above and by undo/redo replay.
    fn apply_cell(
        &mut self,
        row: i32,
        column: i32,
        raw_input: &str,
        number_format: Option<String>,
        style: CellStyle,
    ) {
        let address = CellAddress::new(row, column);
        let format = number_format.filter(|code| !code.is_empty());
        let style = if style.is_default() {
            CellStyle::DEFAULT
        } else {
            style
        };

        if raw_input.is_empty() && format.is_none() && style.is_default() {
            // Nothing to show and nothing to remember: the cell goes, keeping the sheet sparse.
            self.cells.remove(&address);
            self.engine.on_cell_removed(address);
        } else if let Some(cell) = self.cells.get_mut(&address) {
            cell.set_raw_input(raw_input);
            cell.set_number_format(format);
            cell.set_style(style);
        } else {
            let mut cell = CellData::new(raw_input);
            cell.set_number_format(format);
            cell.set_style(style);
            self.cells.insert(address, cell);
        }

        self.recalculate_or_defer(address);
        self.raise_changed();
    }

    /// Suppresses per-cell recalculation and change notifications.
    ///
    /// Used when importing many rows, so the engine runs a single rebuild at the end instead of once
    /// per cell. It also groups the whole bulk edit into one undo step.
    pub fn begin_bulk_update(&mut self) {
        if self.bulk_depth == 0 {
            self.undo.begin_group();
        }

        self.bulk_depth += 1;
    }

    /// Ends a bulk update, rebuilding once if anything is out of date.
    pub fn end_bulk_update(&mut self) {
        if self.bulk_depth == 0 {
            return;
        }

        self.bulk_depth -= 1;

        if self.bulk_depth > 0 {
            return;
        }

        if self.pending_rebuild {
            self.pending_rebuild = false;
            let Self { cells, engine, .. } = self;
            engine.rebuild(&mut CellsView { cells });
        }

        self.undo.end_group();
        self.raise_changed();
    }

    /// Inserts blank rows at `at`, pushing existing cells down and adjusting every formula.
    ///
    /// Returns false, changing nothing, when that would push data off the sheet.
    pub fn insert_rows(&mut self, at: i32, count: i32) -> bool {
        self.insert_lines(at, count, true)
    }

    /// Removes rows, pulling later cells up and rewriting references that pointed at them.
    pub fn delete_rows(&mut self, at: i32, count: i32) -> bool {
        self.delete_lines(at, count, true)
    }

    /// Inserts blank columns at `at`.
    pub fn insert_columns(&mut self, at: i32, count: i32) -> bool {
        self.insert_lines(at, count, false)
    }

    /// Removes columns, rewriting references that pointed at them.
    pub fn delete_columns(&mut self, at: i32, count: i32) -> bool {
        self.delete_lines(at, count, false)
    }

    fn insert_lines(&mut self, at: i32, count: i32, rows: bool) -> bool {
        if count <= 0 {
            return true;
        }

        let limit = if rows { MAX_ROWS } else { MAX_COLUMNS };
        let last_index = limit - 1;

        let at = at.clamp(0, last_index);
        let count = count.min(limit - at);

        for address in self.cells.keys() {
            let position = if rows { address.row } else { address.column };

            if position >= at && position + count > last_index {
                // Refuse rather than silently push data off the edge of the sheet.
                return false;
            }
        }

        let mut replacement = HashMap::new();

        for (address, cell) in &self.cells {
            let position = if rows { address.row } else { address.column };
            let moved = if position >= at {
                position + count
            } else {
                position
            };
            let raw = rewrite_for_line_change(cell.raw_input(), at, count, true, rows);

            let key = if rows {
                CellAddress::new(moved, address.column)
            } else {
                CellAddress::new(address.row, moved)
            };

            replacement.insert(
                key,
                CellContent {
                    raw,
                    format: cell.number_format().map(str::to_string),
                    style: cell.style(),
                },
            );
        }

        self.replace_contents(replacement);
        true
    }

    fn delete_lines(&mut self, at: i32, count: i32, rows: bool) -> bool {
        if count <= 0 {
            return true;
        }

        let limit = if rows { MAX_ROWS } else { MAX_COLUMNS };

        let at = at.clamp(0, limit - 1);
        let count = count.min(limit - at);

        let mut replacement = HashMap::new();

        for (address, cell) in &self.cells {
            let position = if rows { address.row } else { address.column };

            // Cells inside the removed band are gone; everything after it moves up.
            if position >= at && position < at + count {
                continue;
            }

            let moved = if position >= at + count {
                position - count
            } else {
                position
            };

            let raw = rewrite_for_line_change(cell.raw_input(), at, count, false, rows);

            let key = if rows {
                CellAddress::new(moved, address.column)
            } else {
                CellAddress::new(address.row, moved)
            };

            replacement.insert(
                key,
                CellContent {
                    raw,
                    format: cell.number_format().map(str::to_string),
                    style: cell.style(),
                },
            );
        }

        self.replace_contents(replacement);
        true
    }

    /// Replaces the whole sheet with a transformed copy as a single undo step.
    ///
    /// Cells that no longer exist are cleared and the rest written — value, format and style together;
    /// every key is visited once, so the undo stack records one coherent before/after per cell and an
    /// undo puts the sheet back exactly as it was.
    fn replace_contents(&mut self, replacement: HashMap<CellAddress, CellContent>) {
        self.begin_bulk_update();

        // Snapshot the keys first: the setters add and remove entries as they go.
        let existing: Vec<CellAddress> = self.cells.keys().copied().collect();

        for address in existing {
            if replacement.contains_key(&address) {
                continue;
            }

            // Clearing the format and the style as well as the value is what removes a formatted empty
            // cell.
            self.clear_cell(address.row, address.column);
        }

        for (address, content) in replacement {
            self.set_raw_input(address.row, address.column, &content.raw);
            self.set_number_format(address.row, address.column, content.format.as_deref());
            self.set_cell_style(address.row, address.column, content.style);
        }

        self.end_bulk_update();
    }

    /// Removes a cell entirely — value, number format and style — recording one undo step.
    fn clear_cell(&mut self, row: i32, column: i32) {
        let Some(cell) = self.cell(row, column) else {
            return;
        };

        let raw = cell.raw_input().to_string();
        let format = cell.number_format().map(str::to_string);
        let style = cell.style();

        self.apply_cell(row, column, "", None, CellStyle::DEFAULT);
        self.undo.record(
            CellChange::new(row, column, raw, "", format, None).with_style(style, CellStyle::DEFAULT),
        );
    }

    /// Removes every cell and the undo history that referred to them.
    pub fn clear(&mut self) {
        if self.cells.is_empty() {
            self.undo.clear();
            return;
        }

        self.cells.clear();
        self.engine.reset();
        self.undo.clear();
        self.raise_changed();
    }

    /// Drops the undo history without touching the document.
    ///
    /// Used when a document is *built* rather than edited — the sample sheet a new window starts with —
    /// where there is nothing meaningful to undo back to. The C# window suspended undo *recording* while
    /// it seeded; dropping what was recorded afterwards reaches the same place with one call.
    pub fn forget_history(&mut self) {
        self.undo.clear();
    }

    /// Recalculates a cell and everything that depends on it, or defers until the bulk update ends.
    fn recalculate_or_defer(&mut self, address: CellAddress) {
        if self.bulk_depth > 0 {
            self.pending_rebuild = true;
            return;
        }

        // Split borrows: the engine and the cell map are separate fields, so the engine can be given
        // the map without a second borrow of the workbook.
        let Self { cells, engine, .. } = self;
        engine.recalculate(&mut CellsView { cells }, address);
    }

    /// Tells every listener, unless a bulk update is in progress.
    fn raise_changed(&mut self) {
        if self.bulk_depth > 0 {
            return;
        }

        self.revision += 1;

        for listener in &mut self.listeners {
            listener();
        }
    }
}

/// The cell map as the formula engine sees it.
///
/// A separate type rather than an implementation on [`Workbook`] itself: the engine holds this
/// mutably at the same moment the workbook holds the engine, and only two disjoint fields can be
/// borrowed that way.
struct CellsView<'a> {
    cells: &'a mut HashMap<CellAddress, CellData>,
}

impl CellStore for CellsView<'_> {
    fn raw_input(&self, row: i32, column: i32) -> &str {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(CellData::raw_input)
            .unwrap_or("")
    }

    fn value(&self, row: i32, column: i32) -> CellValue {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(|cell| cell.value().clone())
            .unwrap_or(CellValue::Empty)
    }

    /// Cells that do not exist are ignored, so that removing a cell is not undone by the
    /// recalculation that follows it.
    fn set_computed(&mut self, row: i32, column: i32, value: CellValue) {
        if let Some(cell) = self.cells.get_mut(&CellAddress::new(row, column)) {
            cell.set_computed(value);
        }
    }

    fn populated_cells(&self) -> Vec<CellAddress> {
        self.cells.keys().copied().collect()
    }
}

impl ImportSink for Workbook {
    fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str) {
        Workbook::set_raw_input(self, row, column, raw_input);
    }

    fn set_number_format(&mut self, row: i32, column: i32, number_format: Option<&str>) {
        Workbook::set_number_format(self, row, column, number_format);
    }

    fn set_cell_style(&mut self, row: i32, column: i32, style: Option<&CellStyle>) {
        Workbook::set_cell_style(self, row, column, style.copied().unwrap_or_default());
    }
}

impl ExportSource for Workbook {
    fn populated_cells(&self) -> Vec<CellAddress> {
        Workbook::populated_cells(self)
    }

    fn raw_input(&self, row: i32, column: i32) -> &str {
        Workbook::raw_input(self, row, column)
    }

    fn value(&self, row: i32, column: i32) -> CellValue {
        Workbook::value(self, row, column)
    }

    fn number_format(&self, row: i32, column: i32) -> Option<&str> {
        Workbook::number_format(self, row, column)
    }

    fn cell_style(&self, row: i32, column: i32) -> Option<CellStyle> {
        Workbook::cell_style(self, row, column)
    }
}

/// Adjusts a formula for a row or column being inserted or removed.
///
/// A value that is not a formula is returned untouched, and so is a formula the parser cannot read:
/// guessing at a formula is worse than leaving it exactly as it was.
fn rewrite_for_line_change(raw: &str, at: i32, count: i32, inserted: bool, rows: bool) -> String {
    if !raw.starts_with('=') {
        return raw.to_string();
    }

    if rows {
        rewriter::adjust_for_rows(raw, at, count, inserted)
    } else {
        rewriter::adjust_for_columns(raw, at, count, inserted)
    }
}
