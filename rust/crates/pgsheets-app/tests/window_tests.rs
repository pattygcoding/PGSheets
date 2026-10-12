//! The window's own wiring, driven without a display.
//!
//! `grid_input_tests.rs` covers the widget; this covers what the *window* does with it — the one editor,
//! the clipboard round trip, undo, and the unsaved-changes marker. Everything here is reached by keyboard
//! or by event, because the grid's screen position inside the window's layout is not something a test
//! should have to know.
//!
//! Nothing here touches a file dialog: `Save` with no path, `Open`, and the unsaved-changes question all
//! open a native dialog, which would block a headless test forever.

use eframe::egui::{self, Event, Key, Modifiers, Pos2};
use pgsheets_app::app::{Command, PgSheetsApp};
use pgsheets_core::CellRange;

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 700.0;

/// The window under test.
struct Harness {
    context: egui::Context,
    app: PgSheetsApp,

    /// How many shapes the last frame painted, so a test can tell "nothing was drawn" from "it was drawn
    /// somewhere else".
    shapes: usize,
}

impl Harness {
    fn new() -> Self {
        Self {
            context: egui::Context::default(),
            app: PgSheetsApp::default(),
            shapes: 0,
        }
    }

    /// Runs one frame of the whole window with these events.
    fn frame(&mut self, events: Vec<Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(WIDTH, HEIGHT))),
            events,
            ..Default::default()
        };

        let app = &mut self.app;

        self.context.begin_pass(input);

        let mut ui = egui::Ui::new(
            self.context.clone(),
            egui::Id::new("window-under-test"),
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(WIDTH, HEIGHT))),
        );

        app.frame(&mut ui);

        let output = self.context.end_pass();

        self.shapes = output.shapes.len();

        output.drop_without_applying_deltas();
    }

    /// A frame with no input, which is what lets a focus request take effect.
    fn idle(&mut self) {
        self.frame(Vec::new());
    }

    fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_string())]);
    }

    /// A key press and its release, in two frames, the way a platform delivers them.
    ///
    /// The release matters: without it egui marks a second press of the same key as auto-repeat and
    /// ignores it, because the key is still down.
    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.frame(vec![
                Event::ModifiersChanged(modifiers),
                Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                },
            ]);
        }
    }

    fn command(&mut self, key: Key) {
        self.key(
            key,
            Modifiers {
                command: true,
                ctrl: true,
                ..Modifiers::default()
            },
        );
    }

    fn paste(&mut self, text: &str) {
        self.frame(vec![Event::Paste(text.to_string())]);
    }

    /// Runs a menu or keyboard command, which is how the menu-only ones are reached without a pointer.
    fn run(&mut self, command: Command) {
        self.app.run(command);
    }

    /// Clears the sample sheet and puts the cursor back at A1.
    ///
    /// The window starts with a sample table now, so a test that wants a blank sheet has to clear it —
    /// and `Select All` leaves the cursor on the *last* cell of the range, which is where the C# window
    /// left it too.
    fn blank_sheet(&mut self) {
        self.command(Key::A);
        self.key(Key::Delete, Modifiers::default());
        self.go_to_start();
    }

    /// Home goes to the start of the row and the arrows clamp at the top, so this reaches A1.
    fn go_to_start(&mut self) {
        self.key(Key::Home, Modifiers::default());

        for _ in 0..32 {
            self.key(Key::ArrowUp, Modifiers::default());
        }
    }

    /// Shift held, for the tests that extend a selection.
    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Modifiers::default()
        }
    }

    /// Types a value into the active cell and commits it with Enter.
    fn enter(&mut self, value: &str) {
        self.type_text(value);
        self.key(Key::Enter, Modifiers::default());
    }

    /// Types a value into the active cell and commits it with Tab, which moves along the row.
    fn enter_and_tab(&mut self, value: &str) {
        self.type_text(value);
        self.key(Key::Tab, Modifiers::default());
    }

    fn raw(&self, row: i32, column: i32) -> String {
        self.app.workbook().raw_input(row, column).to_string()
    }
}

#[test]
fn the_grid_gets_a_real_rectangle_in_the_middle_of_the_window() {
    let mut harness = Harness::new();
    harness.idle();

    let size = harness.app.grid().layout().size();

    // The bars take a strip each and the grid gets what is left — and it has to be a *real* rectangle.
    // An empty or negative one paints nothing at all, which is precisely the bug this test exists for:
    // the window opened with the menu, the formula bar, the status bar and no sheet in between.
    assert!(
        size.width > f64::from(WIDTH) * 0.9 && size.width <= f64::from(WIDTH),
        "the grid's width was {} of {WIDTH}",
        size.width
    );
    assert!(
        size.height > f64::from(HEIGHT) * 0.8 && size.height < f64::from(HEIGHT),
        "the grid's height was {} of {HEIGHT}",
        size.height
    );

    // The first cell is on screen rather than scrolled out of it.
    let first = harness.app.grid().layout().cell_bounds(0, 0);

    assert!(first.x > 0.0 && first.y >= 0.0, "{first:?}");
    assert!(first.y + first.height < f64::from(HEIGHT), "{first:?}");

    // And the frame actually painted something beyond the bars.
    assert!(
        harness.shapes > 40,
        "the frame painted only {} shapes",
        harness.shapes
    );
}

#[test]
fn a_new_window_starts_with_the_sample_sheet_and_no_history() {
    let mut harness = Harness::new();
    harness.idle();

    // The sample table the C# window seeded, and it is not an edit: nothing to undo, nothing unsaved.
    assert_eq!(harness.raw(0, 0), "Region");
    assert_eq!(harness.raw(1, 3), "=B2+C2");
    assert_eq!(harness.app.workbook().display_value(1, 3), "2650");
    assert_eq!(harness.app.workbook().display_value(5, 1), "3780");

    assert!(!harness.app.is_dirty(), "nobody has edited anything yet");
    assert_eq!(
        harness.app.workbook().undo_depth(),
        0,
        "seeding the sample is not an edit to undo"
    );
    assert_eq!(harness.app.status(), "Ready");
    assert_eq!(harness.app.selection_text(), "", "one cell aggregates nothing");
}

#[test]
fn typing_a_value_and_pressing_enter_writes_it_and_moves_down() {
    let mut harness = Harness::new();
    harness.idle();
    harness.blank_sheet();

    // The first character opens the editor, the second lands in it.
    harness.enter("42");

    assert_eq!(harness.raw(0, 0), "42");
    assert_eq!(harness.raw(1, 0), "");
    assert!(harness.app.is_dirty());
    assert_eq!(
        harness.app.selection(),
        CellRange::single(1, 0),
        "Enter moves to the cell below"
    );
}

#[test]
fn tab_moves_along_the_row() {
    let mut harness = Harness::new();
    harness.idle();

    harness.enter_and_tab("1");
    harness.enter_and_tab("2");

    assert_eq!(harness.raw(0, 0), "1");
    assert_eq!(harness.raw(0, 1), "2");
    assert_eq!(harness.app.selection(), CellRange::single(0, 2));
}

#[test]
fn escape_throws_the_edit_away() {
    let mut harness = Harness::new();
    harness.idle();

    harness.type_text("9");
    harness.key(Key::Escape, Modifiers::default());

    assert_eq!(harness.raw(0, 0), "Region", "the cell is as it was");
    assert!(
        !harness.app.is_dirty(),
        "an abandoned edit never reached the document"
    );
}

#[test]
fn copy_puts_the_selection_on_the_clipboard_as_delimited_text() {
    let mut harness = Harness::new();
    harness.idle();

    // Clear the sample first, so the selection is exactly the two cells this test fills in.
    harness.blank_sheet();

    harness.enter_and_tab("1");
    harness.enter_and_tab("2");

    harness.command(Key::A);
    assert_eq!(
        harness.app.selection(),
        CellRange::new(0, 0, 0, 1),
        "select all covers the two cells"
    );

    harness.command(Key::C);

    assert_eq!(
        harness.app.clipboard_text().trim_end(),
        "1\t2",
        "tab-delimited raw input, which is what a paste reads"
    );
    assert_eq!(harness.app.status(), "Copied A1:B1");
}

#[test]
fn pasting_delimited_text_fills_the_selection_and_undoes_in_one_step() {
    let mut harness = Harness::new();
    harness.idle();
    harness.blank_sheet();

    harness.enter("0");
    harness.key(Key::ArrowUp, Modifiers::default());

    let depth_before = harness.app.workbook().undo_depth();

    harness.paste("7\t8\r\n9\t10");

    assert_eq!(harness.raw(0, 0), "7");
    assert_eq!(harness.raw(0, 1), "8");
    assert_eq!(harness.raw(1, 0), "9");
    assert_eq!(harness.raw(1, 1), "10");
    assert_eq!(
        harness.app.workbook().undo_depth(),
        depth_before + 1,
        "a paste is a single undo step"
    );

    harness.command(Key::Z);

    assert_eq!(harness.raw(0, 0), "0", "undo restores what the paste replaced");
    assert_eq!(harness.raw(1, 1), "");
}

#[test]
fn delete_clears_the_selection_as_one_step() {
    let mut harness = Harness::new();
    harness.idle();

    harness.enter_and_tab("a");
    harness.enter_and_tab("b");

    harness.command(Key::A);
    harness.key(Key::Delete, Modifiers::default());

    assert_eq!(harness.app.workbook().cell_count(), 0);

    harness.command(Key::Z);

    assert_eq!(harness.raw(0, 0), "a");
    assert_eq!(harness.raw(0, 1), "b");
}

#[test]
fn the_document_stays_unsaved_after_undoing_back_to_where_it_started() {
    let mut harness = Harness::new();
    harness.idle();

    assert!(!harness.app.is_dirty());

    harness.enter("1");
    assert!(harness.app.is_dirty());

    harness.command(Key::Z);

    // The cell is as it was before the edit, and the document has moved since it was last saved, so it is
    // still unsaved — the same thing Excel does.
    assert_eq!(harness.raw(0, 0), "Region");
    assert!(harness.app.is_dirty());

    harness.command(Key::Y);
    assert_eq!(harness.raw(0, 0), "1");
}

#[test]
fn the_aggregate_of_the_selection_reads_like_a_spreadsheet() {
    let mut harness = Harness::new();
    harness.idle();
    harness.blank_sheet();

    harness.enter_and_tab("1");
    harness.enter_and_tab("2");
    harness.command(Key::A);

    assert_eq!(harness.app.selection(), CellRange::new(0, 0, 0, 1));
    assert_eq!(
        harness.app.selection_text(),
        "Count: 2    Sum: 3    Average: 1.5",
        "the wording and spacing are the C# window's"
    );
}

#[test]
fn a_single_cell_has_no_aggregate_but_the_sample_sheet_does() {
    let mut harness = Harness::new();
    harness.idle();

    // One cell aggregates nothing at all.
    assert_eq!(harness.app.selection_text(), "");

    // The sample sheet's numbers: six quarters, three row totals and three column totals.
    harness.command(Key::A);

    let text = harness.app.selection_text().to_string();

    assert!(text.starts_with("Count: 12    Sum: "), "{text}");
    assert!(text.contains("    Average: "), "{text}");
}

#[test]
fn a_number_format_applies_to_the_populated_cells_in_the_selection() {
    let mut harness = Harness::new();
    harness.idle();

    harness.command(Key::A);
    harness.run(Command::NumberFormat(Some("0.00")));

    let workbook = harness.app.workbook();

    assert_eq!(workbook.number_format(1, 1), Some("0.00"));
    assert_eq!(workbook.display_value(1, 1), "1200.00");

    let status = harness.app.status().to_string();

    assert!(
        status.starts_with("Applied \"0.00\" to 21 cell(s) in A1:D8"),
        "{status}"
    );

    // The whole selection is one undo step, so one undo takes every format back off.
    harness.command(Key::Z);

    assert_eq!(harness.app.workbook().number_format(1, 1), None);
    assert_eq!(harness.app.workbook().display_value(1, 1), "1200");

    // And General clears it again.
    harness.run(Command::NumberFormat(None));

    let status = harness.app.status().to_string();

    assert!(
        status.starts_with("Cleared the number format on 21 cell(s) in A1:D8"),
        "{status}"
    );
}

#[test]
fn structural_edits_follow_the_selection_and_undo_in_one_step() {
    let mut harness = Harness::new();
    harness.idle();

    // Two rows at the top: the header row and the first region.
    harness.key(Key::ArrowDown, Harness::shift());
    harness.run(Command::InsertRows);

    assert_eq!(harness.raw(2, 0), "Region", "the header moved down two rows");
    assert_eq!(harness.raw(3, 3), "=B4+C4", "and the formula followed it");
    assert_eq!(harness.app.status(), "Inserted 2 row(s)");

    harness.command(Key::Z);

    assert_eq!(harness.raw(0, 0), "Region");
    assert_eq!(harness.raw(1, 3), "=B2+C2");
}

#[test]
fn a_paste_from_this_application_shifts_relative_references() {
    let mut harness = Harness::new();
    harness.idle();
    harness.blank_sheet();

    harness.enter("1");
    harness.enter("2");

    // B1 holds a reference to the cell above it.
    harness.key(Key::ArrowUp, Modifiers::default());
    harness.key(Key::ArrowUp, Modifiers::default());
    harness.key(Key::ArrowRight, Modifiers::default());
    harness.enter("=A1");

    // Copy B1, move one row down, and paste.
    harness.key(Key::ArrowUp, Modifiers::default());
    harness.command(Key::C);

    let copied = harness.app.clipboard_text().to_string();

    harness.key(Key::ArrowDown, Modifiers::default());
    harness.paste(&copied);

    assert_eq!(harness.raw(1, 1), "=A2", "the reference moved with the block");
    assert_eq!(harness.app.workbook().display_value(1, 1), "2");
}

#[test]
fn a_command_that_would_lose_work_waits_for_an_answer() {
    let mut harness = Harness::new();
    harness.idle();

    // Clean: New goes straight through.
    harness.run(Command::New);
    assert_eq!(harness.app.workbook().cell_count(), 0);
    assert_eq!(harness.app.status(), "New spreadsheet");

    harness.enter("1");
    assert!(harness.app.is_dirty());

    // Dirty: New has to ask first, so nothing has happened yet.
    harness.run(Command::New);

    assert_eq!(harness.raw(0, 0), "1", "the document is untouched");
    assert!(harness.app.is_dirty());

    // And an ordinary command on a dirty document is not held up by the question.
    harness.run(Command::SelectUsedRange);
    assert_eq!(harness.app.selection(), CellRange::single(0, 0));
}

#[test]
fn text_from_another_application_keeps_its_references() {
    let mut harness = Harness::new();
    harness.idle();
    harness.blank_sheet();

    harness.enter("7");

    // Nothing was copied from here, so there is no origin to shift from.
    harness.paste("=A1+1");

    assert_eq!(harness.raw(1, 0), "=A1+1");
}

#[test]
fn a_formula_typed_into_a_cell_is_calculated() {
    let mut harness = Harness::new();
    harness.idle();

    // Enter moves down, so the two numbers land in A1 and A2.
    harness.enter("2");
    harness.enter("3");

    // Back to the top and over to C1, so the formula does not sit on top of its own inputs.
    harness.key(Key::ArrowUp, Modifiers::default());
    harness.key(Key::ArrowUp, Modifiers::default());
    harness.key(Key::ArrowRight, Modifiers::default());
    harness.key(Key::ArrowRight, Modifiers::default());

    harness.enter("=A1+A2");

    let workbook = harness.app.workbook();

    assert_eq!(workbook.raw_input(0, 2), "=A1+A2");
    assert_eq!(workbook.display_value(0, 2), "5");
    assert_eq!(workbook.value(0, 2).as_number(), Some(5.0));
    assert_eq!(harness.app.selection(), CellRange::single(1, 2));
}

#[test]
fn an_open_editor_keeps_the_clipboard_shortcuts_for_itself() {
    let mut harness = Harness::new();
    harness.idle();

    harness.enter_and_tab("1");
    harness.enter_and_tab("2");
    harness.key(Key::ArrowLeft, Modifiers::default());
    harness.key(Key::ArrowLeft, Modifiers::default());

    harness.type_text("hello");
    harness.idle();

    // While the editor is open the window leaves Ctrl+A to the text box, so the selection behind it does
    // not become the used range and nothing is copied.
    harness.command(Key::A);
    harness.command(Key::C);

    assert_eq!(
        harness.app.selection(),
        CellRange::single(0, 0),
        "the selection did not change under the editor"
    );
    assert_eq!(harness.app.clipboard_text(), "", "and nothing was copied");

    // Escape leaves the document as it was, editor and all.
    harness.key(Key::Escape, Modifiers::default());

    assert_eq!(harness.raw(0, 0), "1");
    assert_eq!(harness.raw(0, 1), "2");
}
