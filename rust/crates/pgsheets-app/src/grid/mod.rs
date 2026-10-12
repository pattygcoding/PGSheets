//! The grid: its layout arithmetic, its painting and its input.
//!
//! The geometry is a type of its own with no window in sight, so the parts worth testing are testable;
//! what is left is the egui glue around it.

pub mod geometry;
pub mod scrollbar;
mod view;

pub use geometry::{Area, CellRect, GridLayout, Metrics};
pub use scrollbar::Scrollbar;
pub use view::{GridAction, GridView, focus_id};
