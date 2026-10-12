//! Painting and input for the grid: the one control that draws every visible cell.
//!
//! This is the egui counterpart of the C# control's `Render`, `Headers`, `Viewport` and `Input`
//! partials. The invariant is the same one: nothing per cell exists as a widget, the whole sheet is
//! painted in one pass, and only the cells inside the viewport are ever visited.
//!
//! egui lays text out through its own cache, so the bounded text cache the C# control kept by hand is
//! the font store's job here. What is still ours is the clipping: a cell whose text overflows its
//! column is drawn inside a clip rectangle, so only the cells that overflow pay for one.

use eframe::egui::text::{Galley, LayoutJob, TextFormat};
use eframe::egui::{self, Color32, FontId, Key, Rect, Sense, Stroke, StrokeKind, Vec2};
use pgsheets_core::{CellRange, CellValue, HorizontalAlign};
use pgsheets_sheet::Workbook;

use super::geometry::{Area, CellRect, GridLayout};
use super::scrollbar::{self, Scrollbar};

/// Padding between a cell's border and its text.
const CELL_PADDING: f32 = 4.0;

/// How far the second pass of a bold cell is shifted. There is no bold face among the bundled fonts, so
/// "bold" is the text drawn twice, a hair apart.
const BOLD_OFFSET: f32 = 0.7;

/// The scroll-bar track, and the thumb that moves along it.
const TRACK_FILL: Color32 = Color32::from_rgb(0xF1, 0xF1, 0xF1);
const THUMB_FILL: Color32 = Color32::from_rgb(0xC1, 0xC1, 0xC1);

fn grid_line() -> Stroke {
    Stroke::new(1.0, Color32::from_rgb(0xE3, 0xE3, 0xE3))
}

fn selection_line() -> Stroke {
    Stroke::new(2.0, Color32::from_rgb(0x1A, 0x73, 0xE8))
}

/// A blue wash, premultiplied: `rgba(0x1A, 0x73, 0xE8, 0x28)`.
const SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(0x04, 0x12, 0x24, 0x28);
const HEADER_FILL: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
const HEADER_ACTIVE_FILL: Color32 = Color32::from_rgb(0xD7, 0xE7, 0xFB);
const CELL_TEXT: Color32 = Color32::from_rgb(0x20, 0x21, 0x24);
const ERROR_TEXT: Color32 = Color32::from_rgb(0xC5, 0x22, 0x1F);
const HEADER_TEXT: Color32 = Color32::from_rgb(0x5F, 0x63, 0x68);

/// What the grid wants the host to do after a frame.
///
/// The grid never edits the workbook itself: it asks, and the window decides — the same division the
/// C# control had through its events, and the reason the grid is exercised without one.
#[derive(Clone, Debug, PartialEq)]
pub enum GridAction {
    /// Open the in-place editor for a cell. `seed` is the text to start with, or `None` to edit what
    /// the cell already holds.
    Edit {
        /// Zero-based row.
        row: i32,

        /// Zero-based column.
        column: i32,

        /// Text the editor starts with.
        seed: Option<String>,
    },

    /// The active cell or the selection changed.
    SelectionChanged,

    /// Select everything the sheet uses. The grid cannot measure that itself: it has no workbook.
    SelectUsedRange,

    /// Empty every populated cell in the selection.
    Clear,
}

/// The grid: its layout, and the little state a frame needs.
pub struct GridView {
    layout: GridLayout,

    /// True while the grid should hold the keyboard. The window decides, because only it knows whether
    /// the in-place editor or the formula bar has it instead.
    keyboard_owner: bool,
}

impl Default for GridView {
    /// A grid that owns the keyboard, which is what a single-widget host wants; the window turns it off
    /// while one of its text fields is being typed into.
    fn default() -> Self {
        Self {
            layout: GridLayout::default(),
            keyboard_owner: true,
        }
    }
}

/// The id the grid's interactive area uses.
///
/// It is stable rather than auto-generated so that the host can hand the keyboard back to the grid —
/// which it has to after it closes the in-place editor, or the first key after a commit is swallowed
/// while egui still believes the text box has focus.
pub fn focus_id(ui: &egui::Ui) -> egui::Id {
    ui.id().with("pgsheets-grid")
}

impl GridView {
    /// The layout, for a host that needs to ask where a cell is.
    pub fn layout(&self) -> &GridLayout {
        &self.layout
    }

    /// The layout, for a host that needs to move the selection.
    pub fn layout_mut(&mut self) -> &mut GridLayout {
        &mut self.layout
    }

    /// Says whether the grid should hold the keyboard.
    ///
    /// Doing this every frame rather than reacting to a click is not politeness: egui consumes
    /// unmodified arrows and Tab for its own widget-focus navigation, so an upward arrow press moves the
    /// focus to whatever focusable widget is above the grid. A grid that did not take the keyboard back
    /// would lose the key that moved it away, and the one after it would be typed into that other widget.
    pub fn set_keyboard_owner(&mut self, owner: bool) {
        self.keyboard_owner = owner;
    }

    /// A cell's rectangle in screen coordinates, which is what positions the one editor.
    pub fn cell_rect(&self, origin: egui::Pos2, row: i32, column: i32) -> Rect {
        to_screen(origin, self.layout.cell_bounds(row, column))
    }

    /// Draws the grid, handles input, and reports what the host should do about it.
    pub fn show(&mut self, ui: &mut egui::Ui, workbook: &Workbook) -> Option<GridAction> {
        let rect = ui.available_rect_before_wrap();

        // The scroll bars take a strip off the right and the bottom, and the cells are laid out in what is
        // left — which is what keeps a cell's rectangle, the hit test and the editor all in the same frame
        // of reference.
        let body = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x - scrollbar::WIDTH, rect.max.y - scrollbar::WIDTH),
        );

        let origin = body.min;

        self.layout.set_size(Area {
            width: f64::from(body.width()),
            height: f64::from(body.height()),
        });

        let response = ui.interact(body, focus_id(ui), Sense::click_and_drag());
        let painter = ui.painter_at(body);

        // The grid takes the keyboard whenever the window says it should have it, every frame, because
        // egui hands it to the next widget on an unmodified arrow or Tab.
        if self.keyboard_owner && !response.has_focus() {
            response.request_focus();
        }

        painter.rect_filled(body, 0.0, Color32::WHITE);

        self.paint_cells(&painter, body, origin, workbook);
        self.paint_grid_lines(&painter, body, origin);
        self.paint_selection(&painter, origin);
        self.paint_column_header(&painter, origin);
        self.paint_row_header(&painter, origin);

        self.paint_scrollbars(ui, rect, body);

        self.handle_pointer(ui, &response, origin)
            .or_else(|| self.handle_keys(ui))
    }

    /// The two scroll bars, in the strips the cells were laid out beside.
    ///
    /// Dragging a thumb scrolls the view, and clicking a track pages it by one viewport — the two things
    /// a scroll bar is for. The wheel and the keyboard keep working on the cells as before, because the
    /// bars only claim the strips they are drawn in.
    fn paint_scrollbars(&mut self, ui: &egui::Ui, rect: Rect, body: Rect) {
        let extent = self.layout.extent();
        let viewport = self.layout.viewport();
        let (offset_x, offset_y) = self.layout.scroll_offset();

        let horizontal_track = Rect::from_min_max(
            egui::pos2(rect.min.x, body.max.y),
            egui::pos2(body.max.x, rect.max.y),
        );

        let vertical_track = Rect::from_min_max(
            egui::pos2(body.max.x, rect.min.y),
            egui::pos2(rect.max.x, body.max.y),
        );

        let horizontal = Scrollbar {
            extent: extent.width,
            viewport: viewport.width,
            offset: offset_x,
        };

        let vertical = Scrollbar {
            extent: extent.height,
            viewport: viewport.height,
            offset: offset_y,
        };

        if let Some(offset) = draw_bar(ui, horizontal_track, horizontal, false, "grid-scroll-x") {
            self.layout.set_scroll_offset(offset, offset_y);
        }

        if let Some(offset) = draw_bar(ui, vertical_track, vertical, true, "grid-scroll-y") {
            self.layout.set_scroll_offset(offset_x, offset);
        }

        // The corner where the two meet, so it does not read as part of the sheet.
        ui.painter_at(rect).rect_filled(
            Rect::from_min_max(egui::pos2(body.max.x, body.max.y), rect.max),
            0.0,
            TRACK_FILL,
        );
    }

    /// Arrows, Tab, Enter, Home/End, the page keys, Delete, F2, and typing to start an edit.
    ///
    /// One action per frame: a keystroke that moved the selection does not also start an edit, which is
    /// what stops a fast typist's characters from landing in the wrong cell.
    fn handle_keys(&mut self, ui: &egui::Ui) -> Option<GridAction> {
        if !self.keyboard_owner {
            return None;
        }

        let shift = ui.input(|i| i.modifiers.shift);
        let active = self.layout.active_cell();

        // A printable character opens the editor seeded with it: you start typing and the cell becomes
        // something you are editing, which is what a spreadsheet does.
        for event in ui.input(|i| i.events.clone()) {
            let egui::Event::Text(text) = event else {
                continue;
            };

            if text.chars().next().is_some_and(|c| !c.is_control()) {
                return Some(GridAction::Edit {
                    row: active.row,
                    column: active.column,
                    seed: Some(text),
                });
            }
        }

        let pressed = |key: Key| ui.input(|i| i.key_pressed(key));

        if pressed(Key::F2) {
            return Some(GridAction::Edit {
                row: active.row,
                column: active.column,
                seed: None,
            });
        }

        if pressed(Key::Delete) || pressed(Key::Backspace) {
            return Some(GridAction::Clear);
        }

        let metrics = self.layout.metrics();
        let page = ((self.layout.viewport().height / metrics.row_height) as i32).max(1);

        let (row_delta, column_delta) = if pressed(Key::ArrowUp) {
            (-1, 0)
        } else if pressed(Key::ArrowDown) {
            (1, 0)
        } else if pressed(Key::ArrowLeft) {
            (0, -1)
        } else if pressed(Key::ArrowRight) {
            (0, 1)
        } else if pressed(Key::Tab) {
            (0, if shift { -1 } else { 1 })
        } else if pressed(Key::Enter) {
            (if shift { -1 } else { 1 }, 0)
        } else if pressed(Key::PageDown) {
            (page, 0)
        } else if pressed(Key::PageUp) {
            (-page, 0)
        } else if pressed(Key::Home) {
            (0, -active.column)
        } else if pressed(Key::End) {
            (0, self.layout.column_count() - 1 - active.column)
        } else {
            return None;
        };

        // Shift turns a move into an extension, which is what keeps the anchor where it was.
        let changed = if shift {
            self.layout.extend_active_by(row_delta, column_delta)
        } else {
            self.layout.move_active_by(row_delta, column_delta)
        };

        changed.then_some(GridAction::SelectionChanged)
    }

    /// Every populated cell inside the viewport, and nothing else.
    ///
    /// A cell that has never been formatted keeps the fast route — one `layout_no_wrap` and a draw — so
    /// the styling machinery is only paid for by the cells that asked for it.
    fn paint_cells(&self, painter: &egui::Painter, rect: Rect, origin: egui::Pos2, workbook: &Workbook) {
        let metrics = self.layout.metrics();
        let font = FontId::proportional(metrics.font_size);
        let padding = f64::from(CELL_PADDING);

        let (first_row, last_row) = self.layout.visible_rows();
        let (first_column, last_column) = self.layout.visible_columns();

        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let Some(cell) = workbook.cell(row, column) else {
                    continue;
                };

                let style = cell.style();
                let bounds = self.layout.cell_bounds(row, column);
                let cell_rect = to_screen(origin, bounds);

                // A fill is painted even when the cell holds nothing: a cell can carry a fill and no
                // value, and that is exactly what "colour the column, then fill it in" leaves behind.
                if let Some([red, green, blue]) = style.fill {
                    painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(red, green, blue));
                }

                let text = cell.display_value();

                if text.is_empty() {
                    continue;
                }

                let colour = if cell.value().is_error() {
                    ERROR_TEXT
                } else {
                    style
                        .text_color
                        .map(|[red, green, blue]| Color32::from_rgb(red, green, blue))
                        .unwrap_or(CELL_TEXT)
                };

                let align = effective_align(style.align, cell.value());
                let available = bounds.width - padding * 2.0;

                let galley = if style.is_default() {
                    painter.layout_no_wrap(text.to_owned(), font.clone(), colour)
                } else {
                    let mut job = LayoutJob::default();

                    if style.wrap {
                        // Wrapping to the column, not to the row: this app has no variable row heights,
                        // so a wrapped cell shows what fits and clips the rest.
                        job.wrap.max_width = available.max(1.0) as f32;
                    }

                    job.append(
                        text,
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: colour,
                            italics: style.italic,
                            underline: underline_stroke(style.underline, colour),
                            ..Default::default()
                        },
                    );

                    painter.layout_job(job)
                };

                let text_width = galley.size().x;

                let text_x = match align {
                    HorizontalAlign::Right => cell_rect.max.x - CELL_PADDING - text_width,
                    HorizontalAlign::Center => cell_rect.center().x - text_width / 2.0,
                    _ => cell_rect.min.x + CELL_PADDING,
                };

                // Wrapped text starts at the top and grows down; a single line sits in the middle of the
                // row, as it always has.
                let text_y = if style.wrap {
                    cell_rect.min.y + CELL_PADDING / 2.0
                } else {
                    cell_rect.center().y - galley.size().y / 2.0
                };

                let position = egui::pos2(text_x, text_y);

                let overflows =
                    style.wrap || f64::from(text_width) > available || galley.size().y > cell_rect.height();

                if overflows {
                    // Only cells that actually overflow pay for a clip.
                    let clipped = painter.with_clip_rect(cell_rect.intersect(rect));
                    draw_galley(&clipped, position, &galley, colour, style.bold);
                    continue;
                }

                draw_galley(painter, position, &galley, colour, style.bold);
            }
        }
    }

    /// The gridlines of the visible window.
    fn paint_grid_lines(&self, painter: &egui::Painter, rect: Rect, origin: egui::Pos2) {
        let metrics = self.layout.metrics();
        let (first_row, last_row) = self.layout.visible_rows();
        let (first_column, last_column) = self.layout.visible_columns();

        let top = rect.min.y + metrics.header_height as f32;
        let left = rect.min.x + metrics.header_width as f32;

        for column in first_column..=last_column + 1 {
            let x = origin.x + self.layout.column_x(column) as f32;

            if x < left {
                continue;
            }

            painter.line_segment([egui::pos2(x, top), egui::pos2(x, rect.max.y)], grid_line());
        }

        for row in first_row..=last_row + 1 {
            let y = origin.y + self.layout.row_y(row) as f32;

            if y < top {
                continue;
            }

            painter.line_segment([egui::pos2(left, y), egui::pos2(rect.max.x, y)], grid_line());
        }
    }

    /// Shades the selected block and outlines it, with the active cell outlined inside.
    fn paint_selection(&self, painter: &egui::Painter, origin: egui::Pos2) {
        let range = self.layout.selection();
        let metrics = self.layout.metrics();

        let rect = Rect::from_min_size(
            egui::pos2(
                origin.x + self.layout.column_x(range.column1) as f32,
                origin.y + self.layout.row_y(range.row1) as f32,
            ),
            Vec2::new(
                range.column_count() as f32 * metrics.column_width as f32,
                range.row_count() as f32 * metrics.row_height as f32,
            ),
        );

        painter.rect_filled(rect, 0.0, SELECTION_FILL);
        painter.rect_stroke(rect, 0.0, selection_line(), StrokeKind::Inside);

        // A big selection would otherwise hide which cell keyboard input will land in.
        if !range.is_single_cell() {
            let active = self.layout.active_cell();
            let cell = to_screen(origin, self.layout.cell_bounds(active.row, active.column));

            painter.rect_stroke(cell, 0.0, selection_line(), StrokeKind::Inside);
        }
    }

    /// Sticky column header: A, B, C... with the selected columns highlighted.
    fn paint_column_header(&self, painter: &egui::Painter, origin: egui::Pos2) {
        let metrics = self.layout.metrics();
        let range = self.layout.selection();
        let strip = Rect::from_min_size(
            egui::pos2(origin.x + metrics.header_width as f32, origin.y),
            Vec2::new(self.layout.viewport().width as f32, metrics.header_height as f32),
        );

        if strip.width() <= 0.0 {
            return;
        }

        painter.rect_filled(strip, 0.0, HEADER_FILL);

        let font = FontId::proportional(metrics.font_size - 1.0);
        let (first_column, last_column) = self.layout.visible_columns();

        for column in first_column..=last_column {
            let bounds = self.layout.cell_bounds(0, column);
            let cell = Rect::from_min_size(
                egui::pos2(origin.x + bounds.x as f32, strip.min.y),
                Vec2::new(bounds.width as f32, strip.height()),
            );

            if cell.max.x <= strip.min.x {
                continue;
            }

            if column >= range.column1 && column <= range.column2 {
                painter.rect_filled(cell, 0.0, HEADER_ACTIVE_FILL);
            }

            let label = pgsheets_core::CellAddress::column_name(column);
            let galley = painter.layout_no_wrap(label, font.clone(), HEADER_TEXT);

            painter.galley(cell.center() - galley.size() / 2.0, galley, HEADER_TEXT);
            painter.line_segment([cell.left_top(), cell.left_bottom()], grid_line());
        }

        painter.line_segment([strip.left_bottom(), strip.right_bottom()], grid_line());
    }

    /// Sticky row header: 1, 2, 3... with the selected rows highlighted.
    fn paint_row_header(&self, painter: &egui::Painter, origin: egui::Pos2) {
        let metrics = self.layout.metrics();
        let range = self.layout.selection();
        let strip = Rect::from_min_size(
            egui::pos2(origin.x, origin.y + metrics.header_height as f32),
            Vec2::new(metrics.header_width as f32, self.layout.viewport().height as f32),
        );

        if strip.height() <= 0.0 {
            return;
        }

        painter.rect_filled(strip, 0.0, HEADER_FILL);

        let font = FontId::proportional(metrics.font_size - 1.0);
        let (first_row, last_row) = self.layout.visible_rows();

        for row in first_row..=last_row {
            let bounds = self.layout.cell_bounds(row, 0);
            let cell = Rect::from_min_size(
                egui::pos2(strip.min.x, origin.y + bounds.y as f32),
                Vec2::new(strip.width(), bounds.height as f32),
            );

            if cell.max.y <= strip.min.y {
                continue;
            }

            if row >= range.row1 && row <= range.row2 {
                painter.rect_filled(cell, 0.0, HEADER_ACTIVE_FILL);
            }

            let galley = painter.layout_no_wrap((row + 1).to_string(), font.clone(), HEADER_TEXT);

            painter.galley(cell.center() - galley.size() / 2.0, galley, HEADER_TEXT);
            painter.line_segment([cell.left_top(), cell.right_top()], grid_line());
        }

        painter.line_segment([strip.right_top(), strip.right_bottom()], grid_line());
    }

    /// Pointer input: the wheel scrolls, a press selects, a drag extends, a double-click edits, and the
    /// header strips select whole columns and rows.
    fn handle_pointer(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        origin_of_grid: egui::Pos2,
    ) -> Option<GridAction> {
        if response.hovered() {
            let delta = ui.input(|i| i.smooth_scroll_delta);
            let (x, y) = self.layout.scroll_offset();

            self.layout
                .set_scroll_offset(x - f64::from(delta.x), y - f64::from(delta.y));
        }

        let position = response.interact_pointer_pos()?;

        let x = f64::from(position.x - origin_of_grid.x);
        let y = f64::from(position.y - origin_of_grid.y);
        let metrics = self.layout.metrics();

        // Column header strip: the whole column.
        if y < metrics.header_height && x >= metrics.header_width {
            let cell = self.layout.try_hit_test(x, metrics.header_height + 1.0)?;

            if !response.clicked() {
                return None;
            }

            let last_row = self.layout.row_count() - 1;
            self.layout
                .select_range(CellRange::new(0, cell.column, last_row, cell.column));

            return Some(GridAction::SelectionChanged);
        }

        // Row header strip: the whole row.
        if x < metrics.header_width && y >= metrics.header_height {
            let cell = self.layout.try_hit_test(metrics.header_width + 1.0, y)?;

            if !response.clicked() {
                return None;
            }

            let last_column = self.layout.column_count() - 1;
            self.layout
                .select_range(CellRange::new(cell.row, 0, cell.row, last_column));

            return Some(GridAction::SelectionChanged);
        }

        let cell = self.layout.try_hit_test(x, y)?;
        let shift = ui.input(|i| i.modifiers.shift);

        // Where the press landed. A drag is only recognised once the pointer has moved, so by then the
        // press point is somewhere else — and it is the press that decides where a selection grows from.
        let pressed = ui.input(|i| i.pointer.press_origin()).and_then(|origin| {
            self.layout.try_hit_test(
                f64::from(origin.x - origin_of_grid.x),
                f64::from(origin.y - origin_of_grid.y),
            )
        });

        if response.double_clicked() {
            self.layout.set_active_cell(cell.row, cell.column);

            return Some(GridAction::Edit {
                row: cell.row,
                column: cell.column,
                seed: None,
            });
        }

        if response.drag_started() {
            // Anchor on the press, then grow to wherever the pointer has reached.
            if let Some(pressed) = pressed {
                self.layout.set_active_cell(pressed.row, pressed.column);
            }

            return self
                .layout
                .extend_selection_to(cell.row, cell.column)
                .then_some(GridAction::SelectionChanged);
        }

        if response.clicked() {
            // Shift extends what is there instead of starting a new selection.
            let changed = if shift {
                self.layout.extend_selection_to(cell.row, cell.column)
            } else {
                self.layout.set_active_cell(cell.row, cell.column)
            };

            return changed.then_some(GridAction::SelectionChanged);
        }

        if response.dragged() {
            // Dragging extends the selection rather than moving it.
            return self
                .layout
                .extend_selection_to(cell.row, cell.column)
                .then_some(GridAction::SelectionChanged);
        }

        None
    }
}

/// Draws a laid-out cell, twice when the cell is bold.
///
/// There is no bold face among the bundled fonts, so bold is the same text drawn a second time a hair
/// to the right, which thickens every stroke.
fn draw_galley(
    painter: &egui::Painter,
    position: egui::Pos2,
    galley: &std::sync::Arc<Galley>,
    colour: Color32,
    bold: bool,
) {
    painter.galley(position, galley.clone(), colour);

    if bold {
        painter.galley(position + Vec2::new(BOLD_OFFSET, 0.0), galley.clone(), colour);
    }
}

/// The underline a cell asks for, or none.
fn underline_stroke(underline: bool, colour: Color32) -> Stroke {
    if underline {
        Stroke::new(1.0, colour)
    } else {
        Stroke::NONE
    }
}

/// Where a cell's text sits: the alignment it was given, or — for General — the spreadsheet default of
/// numbers right, booleans centred and everything else left.
fn effective_align(align: HorizontalAlign, value: &CellValue) -> HorizontalAlign {
    match align {
        HorizontalAlign::General => match value {
            CellValue::Number(_) => HorizontalAlign::Right,
            CellValue::Boolean(_) => HorizontalAlign::Center,
            _ => HorizontalAlign::Left,
        },
        explicit => explicit,
    }
}

/// Converts a layout rectangle into screen coordinates.
fn to_screen(origin: egui::Pos2, bounds: CellRect) -> Rect {
    Rect::from_min_size(
        egui::pos2(origin.x + bounds.x as f32, origin.y + bounds.y as f32),
        Vec2::new(bounds.width as f32, bounds.height as f32),
    )
}

/// Draws one bar and turns a dragged thumb, or a click on the track, into a new scroll offset.
///
/// Returns `None` when the bar was not touched, so the caller keeps the offset it had.
fn draw_bar(ui: &egui::Ui, track: Rect, bar: Scrollbar, vertical: bool, id: &str) -> Option<f64> {
    let painter = ui.painter_at(track);
    let along = if vertical { track.height() } else { track.width() };

    painter.rect_filled(track, 0.0, TRACK_FILL);

    let length = bar.thumb_length(along);
    let start = bar.thumb_start(along);

    let thumb = if vertical {
        Rect::from_min_size(
            track.min + Vec2::new(0.0, start),
            Vec2::new(track.width(), length),
        )
    } else {
        Rect::from_min_size(
            track.min + Vec2::new(start, 0.0),
            Vec2::new(length, track.height()),
        )
    };

    painter.rect_filled(thumb, 3.0, THUMB_FILL);

    if bar.is_settled() {
        // Nothing to scroll: the bar is shown, but it neither moves nor takes the pointer.
        return None;
    }

    let response = ui.interact(track, ui.id().with(id), Sense::click_and_drag());

    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        let moved = if vertical { delta.y } else { delta.x };
        let travel = (along - length).max(0.0);

        if travel > 0.0 {
            let per_point = bar.max_offset() / f64::from(travel);

            return Some(bar.offset + f64::from(moved) * per_point);
        }

        return None;
    }

    let clicked_off_the_thumb = if vertical {
        response
            .interact_pointer_pos()
            .is_some_and(|position| position.y > thumb.max.y)
    } else {
        response
            .interact_pointer_pos()
            .is_some_and(|position| position.x > thumb.max.x)
    };

    if response.clicked() && response.interact_pointer_pos().is_some() {
        return Some(bar.page(clicked_off_the_thumb));
    }

    None
}
