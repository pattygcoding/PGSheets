//! The two scroll bars, drawn and driven by the grid itself.
//!
//! The implementation this replaced had two native scroll bars wired to the grid's extent and viewport,
//! so the grid owned neither the painting nor the drag. egui's own `ScrollArea` cannot take their place
//! here: it would take over the wheel and the pointer, and the grid's virtualised painting does its own
//! clipping. So the arithmetic is a plain type — which is what makes the thumb positions testable — and
//! the drawing is a few rectangles.

/// How wide a scroll bar is, in points. The cells are laid out in what is left.
pub const WIDTH: f32 = 12.0;

/// The shortest a thumb may get, so that it can always be grabbed.
const MIN_THUMB: f32 = 24.0;

/// The numbers one scroll bar is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scrollbar {
    /// Total content size along the bar, in points.
    pub extent: f64,

    /// Visible size along the bar.
    pub viewport: f64,

    /// How far the content is scrolled, in points.
    pub offset: f64,
}

impl Scrollbar {
    /// How far the content can be scrolled: zero when it all fits.
    pub fn max_offset(&self) -> f64 {
        (self.extent - self.viewport).max(0.0)
    }

    /// True when there is nothing to scroll, so the bar is drawn but does nothing.
    pub fn is_settled(&self) -> bool {
        self.max_offset() <= 0.0
    }

    /// The thumb's length along a track of this size.
    pub fn thumb_length(&self, track: f32) -> f32 {
        if self.extent <= 0.0 {
            return track;
        }

        let fraction = (self.viewport / self.extent).clamp(0.0, 1.0);

        (track * fraction as f32).clamp(MIN_THUMB.min(track), track)
    }

    /// Where the thumb starts along the track.
    pub fn thumb_start(&self, track: f32) -> f32 {
        if self.is_settled() {
            return 0.0;
        }

        let travel = (track - self.thumb_length(track)).max(0.0);

        (f64::from(travel) * (self.offset / self.max_offset())) as f32
    }

    /// The offset a thumb dragged to this position means.
    pub fn offset_at(&self, track: f32, start: f32) -> f64 {
        let travel = (track - self.thumb_length(track)).max(0.0);

        if travel <= 0.0 {
            return 0.0;
        }

        let fraction = f64::from(start.clamp(0.0, travel)) / f64::from(travel);

        fraction * self.max_offset()
    }

    /// The offset one click on the track means: a viewport's worth, towards the pointer.
    pub fn page(&self, towards_end: bool) -> f64 {
        let moved = if towards_end {
            self.offset + self.viewport
        } else {
            self.offset - self.viewport
        };

        moved.clamp(0.0, self.max_offset())
    }
}
