//! The PGSheets desktop application: a virtualised, custom-drawn grid in one window.
//!
//! Every UI type lives in `pgsheets-app`; the model, the formulas and the file formats are separate
//! crates that cannot reach any of it.
//!
//! The crate is a library with a thin binary on top, which is what lets the grid's arithmetic — the
//! part that is worth testing and needs no window — be tested from `tests/`.

pub mod app;
pub mod files;
pub mod grid;

use eframe::egui;

/// Opens the window.
///
/// This is the whole of the binary: everything else lives in the library so that the parts worth
/// testing are reachable from `tests/`.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PGSheets")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PGSheets",
        options,
        Box::new(|_context| Ok(Box::new(app::PgSheetsApp::default()))),
    )
}
