//! Hard limits of the virtual sheet, matching Excel so an imported workbook can never exceed them.

/// Maximum number of rows: 1-based 1,048,576.
pub const MAX_ROWS: i32 = 1_048_576;

/// Maximum number of columns: A..XFD.
pub const MAX_COLUMNS: i32 = 16_384;
