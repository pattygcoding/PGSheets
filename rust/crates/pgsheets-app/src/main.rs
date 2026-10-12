//! The PGSheets desktop application: a virtualised, custom-drawn grid in one window.
//!
//! Every UI type lives in `pgsheets-app`; the model, the formulas and the file formats are separate
//! crates that cannot reach any of it.

fn main() {
    pgsheets_app::run().expect("the window could not be opened");
}
