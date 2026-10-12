use crate::numeric::DecimalDigits;

use super::dates::{DateToken, render_date, scan_date_tokens};
use super::is_date_pattern;

/// A format section read into the parts that matter when it is rendered.
pub(super) struct Pattern {
    /// Set when the section is a date pattern; then the rest of the fields are unused.
    date_tokens: Option<Vec<DateToken>>,

    prefix: String,
    suffix: String,
    min_integer_digits: usize,
    min_fraction_digits: usize,
    max_fraction_digits: usize,
    grouping: bool,
    percent: bool,

    /// Hundreds to divide by, from a comma that follows the placeholders: `#,##0,` is thousands.
    scale: u32,

    /// A section that is nothing but literal text, e.g. `"zero"`.
    literal_only: bool,
}

impl Pattern {
    /// Reads a section, or `None` when it is something this interpreter will not approximate.
    pub(super) fn parse(section: &str) -> Option<Self> {
        if is_date_pattern(section) {
            return scan_date_tokens(section).map(Self::date);
        }

        scan_number(section)
    }

    /// Renders a value, or `None` so the caller falls back to the plain rendering.
    pub(super) fn render(&self, value: f64, section_count: usize) -> Option<String> {
        match &self.date_tokens {
            Some(tokens) => render_date(value, tokens),
            None => self.render_number(value, section_count),
        }
    }

    fn date(tokens: Vec<DateToken>) -> Self {
        Self {
            date_tokens: Some(tokens),
            prefix: String::new(),
            suffix: String::new(),
            min_integer_digits: 0,
            min_fraction_digits: 0,
            max_fraction_digits: 0,
            grouping: false,
            percent: false,
            scale: 0,
            literal_only: false,
        }
    }
}

/// Reads the placeholders, literals and modifiers of a numeric section.
fn scan_number(section: &str) -> Option<Pattern> {
    let chars: Vec<char> = section.chars().collect();

    let mut prefix = String::new();
    let mut suffix = String::new();
    // Whether a literal being read belongs before or after the placeholders.
    let mut is_prefix = true;

    let mut min_integer = 0usize;
    let mut min_fraction = 0usize;
    let mut max_fraction = 0usize;
    let mut placeholders = 0usize;
    let mut grouping = false;
    let mut percent = false;
    let mut scale = 0u32;
    let mut seen_point = false;
    let mut seen_digits = false;

    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        match ch {
            '"' => {
                let offset = chars[index + 1..]
                    .iter()
                    .position(|candidate| *candidate == '"')?;
                let literal: String = chars[index + 1..index + 1 + offset].iter().collect();

                if is_prefix {
                    prefix.push_str(&literal);
                } else {
                    suffix.push_str(&literal);
                }

                index += offset + 2;
                continue;
            }

            '\\' => {
                index += 1;
                let escaped = *chars.get(index)?;

                if is_prefix {
                    prefix.push(escaped);
                } else {
                    suffix.push(escaped);
                }
            }

            '_' => {
                // "_x" reserves the width of x; one space is close enough.
                index += 1;
                chars.get(index)?;

                if is_prefix {
                    prefix.push(' ');
                } else {
                    suffix.push(' ');
                }
            }

            '*' => {
                // "*x" repeats x to fill the cell; the fill itself is not reproduced.
                index += 1;
                chars.get(index)?;
            }

            '[' => {
                let offset = chars[index + 1..]
                    .iter()
                    .position(|candidate| *candidate == ']')?;
                let directive: String = chars[index + 1..index + 1 + offset]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_lowercase();

                // Elapsed time changes what the digits mean, so it is not approximated.
                if matches!(directive.as_str(), "h" | "hh" | "m" | "mm" | "s" | "ss") {
                    return None;
                }

                index += offset + 2;
                continue;
            }

            '0' | '#' | '?' => {
                is_prefix = false;
                placeholders += 1;
                seen_digits = true;

                if seen_point {
                    max_fraction += 1;

                    if ch == '0' {
                        min_fraction += 1;
                    }
                } else if ch == '0' {
                    min_integer += 1;
                }
            }

            '.' => {
                if seen_point {
                    suffix.push('.');
                }

                seen_point = true;
            }

            ',' => {
                if !seen_digits {
                    prefix.push(',');
                } else if is_placeholder(chars.get(index + 1).copied()) {
                    // A comma between placeholders groups them; a trailing one scales by a thousand.
                    grouping = true;
                } else {
                    scale += 1;
                }
            }

            '%' => {
                percent = true;

                if is_prefix {
                    prefix.push('%');
                } else {
                    suffix.push('%');
                }
            }

            _ => {
                if ch.is_alphabetic() {
                    // A letter here is scientific notation, "General", or a token this interpreter
                    // does not know. None of them can be approximated safely.
                    return None;
                }

                // An unquoted '/' next to a placeholder is a fraction ("# ?/?"), which would
                // otherwise be laid out as nonsense. A quoted one never reaches here.
                if ch == '/' && seen_digits && is_placeholder(chars.get(index + 1).copied()) {
                    return None;
                }

                if is_prefix {
                    prefix.push(ch);
                } else {
                    suffix.push(ch);
                }
            }
        }

        index += 1;
    }

    if placeholders == 0 {
        // A section with no placeholders can still be pure literal text, which is how a workbook
        // writes a word for a value: 0.00;0.00;"zero". A bare word, "General" or "@" cannot describe
        // a number at all, so those fall back to the plain rendering.
        let has_literal = section.contains('"') || section.contains('\\');

        if !has_literal || section.contains('@') {
            return None;
        }

        return Some(Pattern {
            date_tokens: None,
            prefix,
            suffix,
            min_integer_digits: 0,
            min_fraction_digits: 0,
            max_fraction_digits: 0,
            grouping: false,
            percent: false,
            scale: 0,
            literal_only: true,
        });
    }

    Some(Pattern {
        date_tokens: None,
        prefix,
        suffix,
        min_integer_digits: min_integer,
        min_fraction_digits: min_fraction,
        max_fraction_digits: max_fraction,
        grouping,
        percent,
        scale,
        literal_only: false,
    })
}

fn is_placeholder(ch: Option<char>) -> bool {
    matches!(ch, Some('0' | '#' | '?'))
}

impl Pattern {
    /// Lays the digits out according to the placeholders.
    fn render_number(&self, value: f64, section_count: usize) -> Option<String> {
        if self.literal_only {
            // The section is a word rather than a number layout: `0.00;0.00;"zero"`.
            return Some(format!("{}{}", self.prefix, self.suffix));
        }

        // Beyond this a `decimal` could not hold the value either, so it is refused rather than
        // rendered as something a reader would not recognise.
        if !value.is_finite() || value.abs() > 7.9e28 {
            return None;
        }

        let mut magnitude = DecimalDigits::from_f64(value.abs())?;

        if self.percent {
            magnitude.scale_by_power_of_ten(2);
        }

        for _ in 0..self.scale {
            magnitude.scale_by_power_of_ten(-3);
        }

        // Rounding happens on the decimal digits, which is what makes "0.00" land on the value a
        // reader expects rather than on the binary fraction a hair either side of it.
        magnitude.round_to_places(self.max_fraction_digits as u32);

        let mut text = String::new();

        // One section has to supply its own sign. With two or more, the negative section carries
        // whatever the workbook wants shown — parentheses, for instance.
        if value < 0.0 && section_count == 1 {
            text.push('-');
        }

        text.push_str(&self.prefix);
        text.push_str(&magnitude.to_fixed(
            self.min_integer_digits,
            self.min_fraction_digits,
            self.max_fraction_digits,
            self.grouping,
        ));
        text.push_str(&self.suffix);

        Some(text)
    }
}
