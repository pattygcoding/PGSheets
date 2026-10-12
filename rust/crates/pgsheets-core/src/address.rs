//! A1 addressing: the zero-based coordinate, its column letters, and the parser that reads both.

/// A zero-based cell coordinate. Row 0 / column 0 maps to the A1 notation `A1`.
///
/// A copyable value type with no heap representation, so it is usable as a dictionary key without
/// allocating. Negative coordinates are representable: they are what arithmetic on references
/// produces before it is clamped, and refusing them here would only move the check elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellAddress {
    /// Zero-based row index.
    pub row: i32,

    /// Zero-based column index.
    pub column: i32,
}

impl CellAddress {
    /// Creates an address from a zero-based row and column.
    pub const fn new(row: i32, column: i32) -> Self {
        Self { row, column }
    }

    /// Converts a zero-based column index to spreadsheet letters: 0 gives `A`, 25 gives `Z`, 26
    /// gives `AA`.
    ///
    /// # Panics
    ///
    /// Panics for a negative column, which has no letter form: that is a programming error rather than
    /// input to be rejected.
    pub fn column_name(column: i32) -> String {
        assert!(column >= 0, "a column index cannot be negative: {column}");

        // Eight characters covers the whole sheet: XFD is the last column.
        let mut buffer = [0u8; 8];
        let mut index = buffer.len();
        let mut value = column;

        loop {
            index -= 1;
            buffer[index] = b'A' + (value % 26) as u8;
            value = value / 26 - 1;

            if value < 0 {
                break;
            }
        }

        String::from_utf8_lossy(&buffer[index..]).into_owned()
    }

    /// Formats a zero-based row and column as A1 notation.
    ///
    /// # Panics
    ///
    /// Panics for a negative row or column, which has no A1 form.
    pub fn to_a1(row: i32, column: i32) -> String {
        assert!(row >= 0, "a row index cannot be negative: {row}");

        let mut text = Self::column_name(column);
        text.push_str(&(row + 1).to_string());
        text
    }

    /// Parses A1 notation such as `A1` or `B7`, discarding any `$` anchors.
    pub fn try_parse_a1(text: &str) -> Option<(i32, i32)> {
        Self::try_parse_a1_anchored(text).map(|parsed| (parsed.row, parsed.column))
    }

    /// Parses A1 notation and reports which parts were anchored with `$`.
    ///
    /// The anchors matter when a formula is copied, so the parser keeps them rather than discarding
    /// them. `None` means the text was not a reference at all.
    pub fn try_parse_a1_anchored(text: &str) -> Option<ParsedAddress> {
        if text.is_empty() {
            return None;
        }

        let bytes = text.as_bytes();
        let mut index = 0;
        let mut absolute_column = false;

        if bytes[index] == b'$' {
            absolute_column = true;
            index += 1;
        }

        let mut column_number = 0i32;
        let mut letter_count = 0;

        while index < bytes.len() {
            let byte = bytes[index];

            if byte.is_ascii_uppercase() {
                column_number = column_number * 26 + (byte - b'A' + 1) as i32;
            } else if byte.is_ascii_lowercase() {
                column_number = column_number * 26 + (byte - b'a' + 1) as i32;
            } else {
                break;
            }

            letter_count += 1;
            index += 1;
        }

        // Three letters is the whole sheet; a fourth could never address a real column.
        if letter_count == 0 || letter_count > 3 {
            return None;
        }

        let mut absolute_row = false;

        if index < bytes.len() && bytes[index] == b'$' {
            absolute_row = true;
            index += 1;
        }

        let mut row_number = 0i32;
        let mut digit_count = 0;

        while index < bytes.len() {
            let byte = bytes[index];

            if !byte.is_ascii_digit() {
                return None;
            }

            row_number = row_number * 10 + (byte - b'0') as i32;
            digit_count += 1;
            index += 1;
        }

        // Rows are 1-based in the notation, so row 0 is not addressable.
        if digit_count == 0 || row_number < 1 {
            return None;
        }

        Some(ParsedAddress {
            row: row_number - 1,
            column: column_number - 1,
            absolute_row,
            absolute_column,
        })
    }
}

impl core::fmt::Display for CellAddress {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&CellAddress::to_a1(self.row, self.column))
    }
}

/// The result of parsing A1 notation, including which parts were anchored with `$`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedAddress {
    /// Zero-based row.
    pub row: i32,

    /// Zero-based column.
    pub column: i32,

    /// True when the row carried a `$`.
    pub absolute_row: bool,

    /// True when the column carried a `$`.
    pub absolute_column: bool,
}
