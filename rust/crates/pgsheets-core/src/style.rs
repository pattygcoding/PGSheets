//! The cell formatting this program understands: the handful of things a cell can say about how its
//! contents look, beyond the number format that decides what a value *means*.
//!
//! Deliberately small and `Copy`. A style travels through the same mutation point as a value and a
//! number format, so it has to be cheap to snapshot for undo and comparable by value — which is why it
//! is a flat record of flags and an optional colour rather than anything that could grow a tree.

/// Where a cell's contents sit horizontally.
///
/// [`HorizontalAlign::General`] is the spreadsheet default rather than a position of its own: numbers
/// go right, text goes left and booleans are centred, which is what Excel and Sheets do when a cell is
/// not told otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    /// The value decides: numbers right, text left, booleans centred.
    #[default]
    General,

    /// Against the left edge of the cell.
    Left,

    /// Centred.
    Center,

    /// Against the right edge of the cell.
    Right,
}

/// The outlook of a cell: the text attributes and the alignment it was given.
///
/// The default encompasses "no formatting at all", which is what makes an unstyled cell cost nothing
/// to compare and lets the render path keep its fast path for everything the user has not touched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellStyle {
    /// Heavier strokes. There is no bold face among the bundled fonts, so this is drawn heavier rather
    /// than swapped for another font.
    pub bold: bool,

    /// Slanted text.
    pub italic: bool,

    /// A line under the text.
    pub underline: bool,

    /// Horizontal position of the contents.
    pub align: HorizontalAlign,

    /// Text colour as red, green and blue, or `None` for the grid's default. Kept as plain bytes so
    /// this crate stays free of any UI type.
    pub text_color: Option<[u8; 3]>,

    /// The cell's background fill as red, green and blue, or `None` for no fill. Like the text colour
    /// it is plain bytes, so the model keeps no UI type.
    pub fill: Option<[u8; 3]>,

    /// Wrap the text to the column width rather than letting it overflow.
    pub wrap: bool,
}

impl CellStyle {
    /// The default style, spelled out so callers do not need [`Default`] in scope.
    pub const DEFAULT: Self = Self {
        bold: false,
        italic: false,
        underline: false,
        align: HorizontalAlign::General,
        text_color: None,
        fill: None,
        wrap: false,
    };

    /// True when this style would render exactly as an unstyled cell does.
    ///
    /// The render path takes its cheap route when this holds, so an untouched sheet pays nothing for
    /// styling existing.
    pub fn is_default(&self) -> bool {
        *self == Self::DEFAULT
    }

    /// A copy with bold set.
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// A copy with italic set.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// A copy with the underline set.
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// A copy with the alignment set.
    pub fn with_align(mut self, align: HorizontalAlign) -> Self {
        self.align = align;
        self
    }

    /// A copy with the text colour set, or cleared with `None`.
    pub fn with_text_color(mut self, text_color: Option<[u8; 3]>) -> Self {
        self.text_color = text_color;
        self
    }

    /// A copy with the background fill set, or cleared with `None`.
    pub fn with_fill(mut self, fill: Option<[u8; 3]>) -> Self {
        self.fill = fill;
        self
    }

    /// A copy with wrapping set.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}
