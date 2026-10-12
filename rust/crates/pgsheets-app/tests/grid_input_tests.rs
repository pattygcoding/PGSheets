//! The grid driven by synthetic input, in a windowless egui context.
//!
//! `GridView::show` is an ordinary egui widget, so a real `Context` can run it with hand-made events and
//! no display at all. That is what lets the parts that only exist at run time — hit-testing under the
//! pointer, focus, drag-select, type-to-edit — be tested rather than assumed.

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use pgsheets_app::grid::{GridAction, GridView};
use pgsheets_sheet::Workbook;

/// The screen the tests pretend to draw on. The grid gets all of it, so a test point is a layout point.
const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;

/// The centre of a cell, given the default metrics: 100-wide columns, 24-high rows, 52/24 headers.
fn cell(row: f32, column: f32) -> Pos2 {
    Pos2::new(52.0 + column * 100.0 + 10.0, 24.0 + row * 24.0 + 10.0)
}

fn moved(position: Pos2) -> Event {
    Event::PointerMoved(position)
}

fn press(position: Pos2, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    }
}

fn release(position: Pos2) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    }
}

fn shift_down() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}

/// A grid, a workbook and a context to run them in.
struct Harness {
    context: egui::Context,
    grid: GridView,
    workbook: Workbook,

    /// Where the grid's own top-left corner landed, observed rather than assumed: the root `Ui` is not
    /// obliged to hand over the whole screen at exactly its origin.
    origin: Pos2,
}

impl Harness {
    /// A grid over a workbook holding one cell of text, at A1.
    fn new() -> Self {
        let mut workbook = Workbook::new(Box::new(pgsheets_formulas::Engine::new()));

        workbook.set_raw_input(0, 0, "hello");

        Self {
            context: egui::Context::default(),
            grid: GridView::default(),
            workbook,
            origin: Pos2::ZERO,
        }
    }

    /// A layout point, translated into the screen point the platform would report for it.
    fn screen(&self, position: Pos2) -> Pos2 {
        position + self.origin.to_vec2()
    }

    /// Runs one frame with these events and reports what the grid asked for.
    fn frame(&mut self, events: Vec<Event>) -> Option<GridAction> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(WIDTH, HEIGHT))),
            events,
            ..Default::default()
        };

        let grid = &mut self.grid;
        let workbook = &self.workbook;

        // A pass is begun and ended by hand: there is no window, but egui does not need one to run a
        // widget.
        self.context.begin_pass(input);

        let mut ui = egui::Ui::new(
            self.context.clone(),
            egui::Id::new("grid-under-test"),
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(WIDTH, HEIGHT))),
        );

        self.origin = ui.available_rect_before_wrap().min;

        let action = grid.show(&mut ui, workbook);

        self.context.end_pass().drop_without_applying_deltas();

        action
    }

    /// A press and a release at the same point: a click, the way the platform would deliver it.
    fn click(&mut self, position: Pos2) -> Option<GridAction> {
        let screen = self.screen(position);

        // The platform hovers before it clicks, and egui needs that first pass to know where the
        // pointer is.
        self.frame(vec![moved(screen)]);
        self.frame(vec![moved(screen), press(screen, Modifiers::default())]);

        self.frame(vec![moved(screen), release(screen)])
    }

    /// A click with modifiers held, which a real backend announces with a modifier event.
    fn click_with(&mut self, position: Pos2, modifiers: Modifiers) -> Option<GridAction> {
        let screen = self.screen(position);

        self.frame(vec![moved(screen)]);
        self.frame(vec![
            Event::ModifiersChanged(modifiers),
            moved(screen),
            press(screen, modifiers),
        ]);

        self.frame(vec![
            Event::ModifiersChanged(modifiers),
            moved(screen),
            release(screen),
        ])
    }

    /// A press, a move with the button still down, and a release: a drag.
    fn drag(&mut self, from: Pos2, to: Pos2) -> Option<GridAction> {
        let start = self.screen(from);
        let end = self.screen(to);

        // As with a click, the pointer has to be known before the button goes down.
        self.frame(vec![moved(start)]);
        self.frame(vec![moved(start), press(start, Modifiers::default())]);

        let extended = self.frame(vec![moved(end)]);

        self.frame(vec![moved(end), release(end)]);

        extended
    }

    fn type_text(&mut self, text: &str) -> Option<GridAction> {
        self.frame(vec![Event::Text(text.to_string())])
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) -> Option<GridAction> {
        // Modifiers are announced separately: egui tracks them from this event alone, even though a key
        // event also carries them. The release is sent too, or a second press of the same key counts as
        // auto-repeat and is ignored.
        let action = self.frame(vec![
            Event::ModifiersChanged(modifiers),
            Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            },
        ]);

        self.frame(vec![
            Event::ModifiersChanged(modifiers),
            Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            },
        ]);

        action
    }

    fn selection(&self) -> pgsheets_core::CellRange {
        self.grid.layout().selection()
    }

    fn active(&self) -> pgsheets_core::CellAddress {
        self.grid.layout().active_cell()
    }
}

#[test]
fn a_click_selects_the_cell_under_the_pointer() {
    let mut harness = Harness::new();

    harness.click(cell(3.0, 2.0));

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(3, 2));
    assert_eq!(harness.selection(), pgsheets_core::CellRange::single(3, 2));
}

#[test]
fn a_click_over_the_headers_hits_nothing() {
    let mut harness = Harness::new();

    // The corner where the two strips meet belongs to neither, so the selection does not move.
    harness.click(Pos2::new(10.0, 10.0));

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(0, 0));
    assert_eq!(harness.selection(), pgsheets_core::CellRange::single(0, 0));
}

#[test]
fn a_click_in_the_column_header_selects_the_whole_column() {
    let mut harness = Harness::new();

    // Column B's header: x inside the column, y inside the header strip.
    let action = harness.click(Pos2::new(52.0 + 100.0 + 10.0, 10.0));

    assert_eq!(action, Some(GridAction::SelectionChanged));

    let selection = harness.selection();

    assert_eq!(selection.column1, 1);
    assert_eq!(selection.column2, 1);
    assert_eq!(selection.row1, 0);
    assert!(selection.row2 > 1000, "the whole virtual column");
}

#[test]
fn a_click_in_the_row_header_selects_the_whole_row() {
    let mut harness = Harness::new();

    harness.click(Pos2::new(10.0, 24.0 + 48.0 + 5.0));

    let selection = harness.selection();

    assert_eq!(selection.row1, 2);
    assert_eq!(selection.row2, 2);
    assert_eq!(selection.column1, 0);
    assert!(selection.column2 > 1000, "the whole virtual row");
}

#[test]
fn dragging_extends_the_selection_from_where_the_press_landed() {
    let mut harness = Harness::new();

    // The change happens while the button is down, so the release frame reports nothing.
    harness.drag(cell(1.0, 1.0), cell(4.0, 3.0));

    assert_eq!(harness.selection(), pgsheets_core::CellRange::new(1, 1, 4, 3));
    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(4, 3));
}

#[test]
fn a_shift_click_extends_instead_of_moving() {
    let mut harness = Harness::new();
    harness.click(cell(2.0, 2.0));

    harness.click_with(cell(5.0, 4.0), shift_down());

    assert_eq!(harness.selection(), pgsheets_core::CellRange::new(2, 2, 5, 4));
    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(5, 4));
}

#[test]
fn the_wheel_scrolls_the_viewport() {
    let mut harness = Harness::new();
    harness.click(cell(0.0, 0.0));

    // A negative delta moves the content up, which is what a wheel spun away from the user means.
    let over_the_grid = harness.screen(cell(0.0, 0.0));

    harness.frame(vec![
        moved(over_the_grid),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::default(),
        },
    ]);

    // Scrolling is smoothed, so the offset eases in over the frames that follow.
    for _ in 0..10 {
        harness.frame(Vec::new());
    }

    let (scroll_x, scrolled_down) = harness.grid.layout().scroll_offset();

    assert_eq!(scroll_x, 0.0);
    assert!(scrolled_down > 0.0, "the wheel scrolled the view down");

    // And the other way brings it back, without ever going above the top.
    harness.frame(vec![
        moved(over_the_grid),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 120.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::default(),
        },
    ]);

    for _ in 0..60 {
        harness.frame(Vec::new());
    }

    let (_, scrolled_back) = harness.grid.layout().scroll_offset();

    assert!(scrolled_back >= 0.0);
    assert!(
        scrolled_back < scrolled_down,
        "the wheel scrolled the view back up: {scrolled_back} of {scrolled_down}"
    );
}

#[test]
fn typing_a_character_starts_an_edit_seeded_with_it() {
    let mut harness = Harness::new();
    harness.click(cell(1.0, 1.0));

    let action = harness.type_text("7");

    assert_eq!(
        action,
        Some(GridAction::Edit {
            row: 1,
            column: 1,
            seed: Some("7".to_string())
        })
    );
}

#[test]
fn f2_starts_an_edit_from_the_cells_own_content() {
    let mut harness = Harness::new();
    harness.click(cell(0.0, 0.0));

    let action = harness.key(Key::F2, Modifiers::default());

    assert_eq!(
        action,
        Some(GridAction::Edit {
            row: 0,
            column: 0,
            seed: None
        })
    );
}

#[test]
fn delete_clears_the_selection() {
    let mut harness = Harness::new();
    harness.click(cell(1.0, 1.0));

    assert_eq!(
        harness.key(Key::Delete, Modifiers::default()),
        Some(GridAction::Clear)
    );
}

#[test]
fn the_arrows_move_the_active_cell_and_shift_extends_it() {
    let mut harness = Harness::new();

    harness.click(cell(2.0, 2.0));
    harness.key(Key::ArrowDown, Modifiers::default());
    harness.key(Key::ArrowRight, Modifiers::default());

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(3, 3));

    harness.key(Key::ArrowRight, shift_down());

    assert_eq!(harness.selection(), pgsheets_core::CellRange::new(3, 3, 3, 4));
}

#[test]
fn tab_and_enter_move_along_the_grid() {
    let mut harness = Harness::new();

    harness.click(cell(0.0, 0.0));
    harness.key(Key::Tab, Modifiers::default());

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(0, 1));

    harness.key(Key::Enter, Modifiers::default());

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(1, 1));

    harness.key(Key::Enter, shift_down());

    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(0, 1));
}

#[test]
fn a_keyboard_move_that_leaves_the_viewport_scrolls_to_follow_it() {
    let mut harness = Harness::new();
    harness.click(cell(0.0, 0.0));

    // Far past the bottom of a 600-point window.
    for _ in 0..100 {
        harness.key(Key::PageDown, Modifiers::default());
    }

    let (_, scroll_y) = harness.grid.layout().scroll_offset();
    let active = harness.active();

    assert!(active.row > 0);
    assert!(scroll_y > 0.0, "the view followed the cursor");
    assert!(
        harness.grid.layout().cell_bounds(active.row, active.column).y < f64::from(HEIGHT),
        "and the active cell is on screen"
    );
}

#[test]
fn the_grid_takes_the_keyboard_when_nothing_else_holds_it() {
    let mut harness = Harness::new();

    // No click: the grid has to claim focus by itself, or arrows would do nothing on a fresh window.
    let action = harness.key(Key::ArrowDown, Modifiers::default());

    assert_eq!(action, Some(GridAction::SelectionChanged));
    assert_eq!(harness.active(), pgsheets_core::CellAddress::new(1, 0));
}
