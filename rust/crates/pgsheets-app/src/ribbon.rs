//! The ribbon: the cell commands in labelled groups, and the colour menus behind the font controls.
//!
//! Everything here draws and reports; nothing touches the document. A control returns what it means and
//! the window runs it, which is the same division the grid keeps with its actions — and the reason the
//! ribbon can be exercised without a workbook.

use eframe::egui::{self, Color32, Rect, Shape, Stroke, StrokeKind};
use pgsheets_core::HorizontalAlign;

use crate::palette::{Palette, STANDARD_COLORS};

/// The caption under a ribbon group: small and grey, the way a spreadsheet labels its groups.
const CAPTION: Color32 = Color32::from_rgb(0x5F, 0x63, 0x68);

/// The colour a text control shows when the cell carries none.
pub const DEFAULT_TEXT: [u8; 3] = [0x20, 0x21, 0x24];

/// The colour a fill control shows when the cell carries none, which is white — an empty cell's paper.
pub const DEFAULT_FILL: [u8; 3] = [0xFF, 0xFF, 0xFF];

/// Draws one ribbon group: a framed row of controls with its name beneath it.
pub fn group(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // The body sits in a row; a group that needs a second line opens its own `vertical`.
                ui.horizontal(|ui| body(ui));
                ui.add_space(1.0);
                ui.label(egui::RichText::new(title).size(10.0).color(CAPTION));
            });
        });
}

/// One horizontal-alignment button, drawn rather than labelled: the four ragged lines a spreadsheet
/// shows, aligned the way the button means.
///
/// Drawn with the painter rather than set as a glyph, because the bundled fonts carry no alignment
/// symbol, and a character a font lacks renders as a blank or a box.
pub fn alignment_button(
    ui: &mut egui::Ui,
    align: HorizontalAlign,
    selected: bool,
    hint: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 20.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        let painter = ui.painter();

        painter.rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
        painter.rect_stroke(rect, visuals.corner_radius, visuals.bg_stroke, StrokeKind::Inside);

        let ink = visuals.fg_stroke.color;
        let inner = rect.shrink2(egui::vec2(8.0, 4.0));
        let step = inner.height() / 4.0;

        // Four lines, long and short alternating, which is the ragged edge the icon reads by.
        for (index, fraction) in [1.0f32, 0.55, 1.0, 0.55].into_iter().enumerate() {
            let y = inner.top() + step * (index as f32 + 0.5);
            let width = inner.width() * fraction;

            let x = match align {
                HorizontalAlign::Right => inner.right() - width,
                HorizontalAlign::Center => inner.center().x - width / 2.0,
                _ => inner.left(),
            };

            painter.line_segment(
                [egui::pos2(x, y), egui::pos2(x + width, y)],
                Stroke::new(1.5, ink),
            );
        }
    }

    response.on_hover_text(hint)
}

/// What picking from a colour menu asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourChoice {
    /// Use this colour, or clear the setting with `None`.
    Apply(Option<[u8; 3]>),

    /// Keep this colour in the palette.
    Save([u8; 3]),

    /// Drop this colour from the palette.
    Forget([u8; 3]),
}

/// A single swatch: a coloured square that reports being clicked.
fn swatch(ui: &mut egui::Ui, colour: [u8; 3], size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        // A border, so a white swatch is still visible against a light menu.
        let edge = if response.hovered() {
            ui.style().visuals.selection.bg_fill
        } else {
            ui.style().visuals.widgets.noninteractive.bg_stroke.color
        };

        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, Color32::from_rgb(colour[0], colour[1], colour[2]));
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, edge), StrokeKind::Inside);
    }

    response
}

/// A colour button: the current colour in a chip, with a triangle that says a menu is behind it.
fn colour_chip(ui: &mut egui::Ui, colour: [u8; 3], hint: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(36.0, 20.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let painter = ui.painter();

        painter.rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
        painter.rect_stroke(rect, visuals.corner_radius, visuals.bg_stroke, StrokeKind::Inside);

        // The colour itself, and the triangle that says a menu is behind it.
        let chip = Rect::from_min_size(
            rect.min + egui::vec2(4.0, 4.0),
            egui::vec2(rect.width() - 18.0, rect.height() - 8.0),
        );
        painter.rect_filled(chip, 2.0, Color32::from_rgb(colour[0], colour[1], colour[2]));
        painter.rect_stroke(
            chip,
            2.0,
            Stroke::new(1.0, visuals.bg_stroke.color),
            StrokeKind::Inside,
        );

        let centre = egui::pos2(rect.right() - 9.0, rect.center().y);
        painter.add(Shape::convex_polygon(
            vec![
                egui::pos2(centre.x - 3.5, centre.y - 2.0),
                egui::pos2(centre.x + 3.5, centre.y - 2.0),
                egui::pos2(centre.x, centre.y + 2.5),
            ],
            visuals.fg_stroke.color,
            Stroke::NONE,
        ));
    }

    response.on_hover_text(hint)
}

/// Draws a colour control, opens its menu, and reports whatever was picked.
///
/// `clear_label` names what "no colour" means here — *Automatic* for the text colour, *No fill* for the
/// background — because the two differ in what they say rather than in what they do.
pub fn colour_control(
    ui: &mut egui::Ui,
    hint: &str,
    current: Option<[u8; 3]>,
    fallback: [u8; 3],
    palette: &Palette,
    clear_label: &str,
) -> Option<ColourChoice> {
    let response = colour_chip(ui, current.unwrap_or(fallback), hint);
    let mut choice = None;

    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(196.0);

        if ui.button(clear_label).clicked() {
            choice = Some(ColourChoice::Apply(None));
        }

        palette_rows(ui, "Basic colours", &STANDARD_COLORS, &mut choice);

        if !palette.recent().is_empty() {
            palette_rows(ui, "Recently used", palette.recent(), &mut choice);
        }

        if !palette.saved().is_empty() {
            palette_rows(ui, "Saved", palette.saved(), &mut choice);
        }

        if let Some(colour) = current {
            ui.separator();

            let kept = palette.saved().contains(&colour);
            let label = if kept {
                "Remove from saved"
            } else {
                "Save this colour"
            };

            if ui.button(label).clicked() {
                choice = Some(if kept {
                    ColourChoice::Forget(colour)
                } else {
                    ColourChoice::Save(colour)
                });
            }
        }

        // A colour that is not in the grid, behind a header so the menu stays short.
        egui::CollapsingHeader::new("More colours")
            .id_salt("more-colours")
            .show(ui, |ui| {
                more_colours(ui, current.unwrap_or(fallback), &mut choice)
            });
    });

    choice
}

/// A titled grid of swatches, wrapping to the menu's width.
fn palette_rows(ui: &mut egui::Ui, title: &str, colours: &[[u8; 3]], choice: &mut Option<ColourChoice>) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(title).size(10.0).color(CAPTION));

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);

        for colour in colours {
            if swatch(ui, *colour, 18.0).clicked() {
                *choice = Some(ColourChoice::Apply(Some(*colour)));
            }
        }
    });
}

/// The full picker, for a colour that is not one of the swatches.
///
/// The colour is taken with an explicit button rather than applied as the picker moves: applying on
/// every frame of a drag would write an undo step per frame, which is not what anyone wants.
fn more_colours(ui: &mut egui::Ui, current: [u8; 3], choice: &mut Option<ColourChoice>) {
    let id = egui::Id::new("colour-picker");
    let mut working = ui
        .ctx()
        .data_mut(|data| data.get_temp::<[u8; 3]>(id))
        .unwrap_or(current);
    let mut rgba = Color32::from_rgb(working[0], working[1], working[2]);

    ui.scope(|ui| {
        ui.set_max_width(180.0);
        egui::widgets::color_picker::color_picker_color32(
            ui,
            &mut rgba,
            egui::widgets::color_picker::Alpha::Opaque,
        );
    });

    working = [rgba.r(), rgba.g(), rgba.b()];
    ui.ctx().data_mut(|data| data.insert_temp(id, working));

    if ui.button("Use this colour").clicked() {
        *choice = Some(ColourChoice::Apply(Some(working)));
    }
}
