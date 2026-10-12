//! The date half of the number-format interpreter: reading a section such as `yyyy-mm-dd hh:mm` into
//! the parts to emit, and rendering a serial through them.

use crate::serial_dates::{CivilDateTime, day_name, month_name, try_to_datetime};

/// One part of a date format section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DateTokenKind {
    /// Quoted text, an escaped character, or a separator.
    Literal,

    /// `yyyy`.
    YearFourDigit,

    /// `yy`.
    YearTwoDigit,

    /// `m` or `mm`, which means a month or a minute depending on what stands beside it.
    MonthOrMinute,

    /// `mmm`.
    MonthShortName,

    /// `mmmm`.
    MonthName,

    /// `d` or `dd`.
    DayNumber,

    /// `ddd`.
    DayShortName,

    /// `dddd`.
    DayName,

    /// `h` or `hh`.
    Hour,

    /// `s` or `ss`.
    Second,

    /// The digits after a point: a fraction of a second.
    FractionalSecond,

    /// `AM/PM`.
    AmPm,

    /// `A/P`.
    ApMarker,
}

/// One part of a date format section, with the text it carries when it is a literal.
#[derive(Clone, Debug)]
pub(super) struct DateToken {
    kind: DateTokenKind,
    padded: bool,
    literal: String,
    digits: usize,
}

impl DateToken {
    fn new(kind: DateTokenKind, padded: bool, literal: &str) -> Self {
        Self {
            kind,
            padded,
            literal: literal.to_string(),
            digits: 0,
        }
    }
}

/// Reads a `yyyy-mm-dd hh:mm` style section into the tokens to emit, in order.
///
/// `None` for anything this interpreter will not approximate: an unterminated quote or bracket, or
/// elapsed-time directives such as `[h]` or `[mm]`, which change what the digits mean.
pub(super) fn scan_date_tokens(section: &str) -> Option<Vec<DateToken>> {
    let chars: Vec<char> = section.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        match ch {
            '"' => {
                let offset = chars[index + 1..]
                    .iter()
                    .position(|candidate| *candidate == '"')?;
                let literal: String = chars[index + 1..index + 1 + offset].iter().collect();
                tokens.push(DateToken::new(DateTokenKind::Literal, false, &literal));
                index += offset + 1;
            }

            '\\' | '_' | '*' => {
                let next = *chars.get(index + 1)?;

                if ch != '*' {
                    // "_x" reserves the width of x; one space stands in for it.
                    let literal = if ch == '_' {
                        " ".to_string()
                    } else {
                        next.to_string()
                    };
                    tokens.push(DateToken::new(DateTokenKind::Literal, false, &literal));
                }

                index += 1;
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

                // "[h]", "[mm]" and friends count elapsed time, which changes what the digits mean.
                if matches!(directive.as_str(), "h" | "hh" | "m" | "mm" | "s" | "ss") {
                    return None;
                }

                index += offset + 1;
            }

            _ => {
                if let Some((kind, length)) = read_meridiem(&chars, index) {
                    tokens.push(DateToken::new(kind, false, ""));
                    index += length;
                    continue;
                }

                if ch == '.' {
                    // A point followed by digits is a fraction of a second, as in "hh:mm:ss.000".
                    let mut digits = 0;

                    while matches!(chars.get(index + 1 + digits), Some('0')) {
                        digits += 1;
                    }

                    if digits > 0 {
                        // The point is kept as its own literal so "hh:mm:ss.000" keeps its separator.
                        tokens.push(DateToken::new(DateTokenKind::Literal, false, "."));
                        tokens.push(DateToken {
                            kind: DateTokenKind::FractionalSecond,
                            padded: false,
                            literal: String::new(),
                            digits,
                        });
                        index += digits + 1;
                        continue;
                    }
                }

                if !ch.is_alphabetic() {
                    tokens.push(DateToken::new(DateTokenKind::Literal, false, &ch.to_string()));
                    index += 1;
                    continue;
                }

                let mut run = 1;

                while index + run < chars.len() && chars[index + run].eq_ignore_ascii_case(&ch) {
                    run += 1;
                }

                tokens.push(map_date_token(ch, run)?);
                index += run;
                continue;
            }
        }

        index += 1;
    }

    if tokens.is_empty() { None } else { Some(tokens) }
}

/// Maps one run of repeated letters, e.g. `mmmm`, to the part it stands for.
fn map_date_token(ch: char, run: usize) -> Option<DateToken> {
    let padded = run >= 2;

    match ch.to_ascii_lowercase() {
        'y' => {
            if run > 4 {
                return None;
            }

            let kind = if run == 2 {
                DateTokenKind::YearTwoDigit
            } else {
                DateTokenKind::YearFourDigit
            };

            Some(DateToken::new(kind, padded, ""))
        }

        'm' => match run {
            1 | 2 => Some(DateToken::new(DateTokenKind::MonthOrMinute, padded, "")),
            3 => Some(DateToken::new(DateTokenKind::MonthShortName, false, "")),
            4 => Some(DateToken::new(DateTokenKind::MonthName, false, "")),
            _ => None,
        },

        'd' => match run {
            1 | 2 => Some(DateToken::new(DateTokenKind::DayNumber, padded, "")),
            3 => Some(DateToken::new(DateTokenKind::DayShortName, false, "")),
            4 => Some(DateToken::new(DateTokenKind::DayName, false, "")),
            _ => None,
        },

        'h' if run <= 2 => Some(DateToken::new(DateTokenKind::Hour, padded, "")),
        's' if run <= 2 => Some(DateToken::new(DateTokenKind::Second, padded, "")),

        _ => None,
    }
}

/// Recognises `AM/PM` or `A/P` at the current position.
fn read_meridiem(chars: &[char], index: usize) -> Option<(DateTokenKind, usize)> {
    for (candidate, kind) in [("AM/PM", DateTokenKind::AmPm), ("A/P", DateTokenKind::ApMarker)] {
        let letters: Vec<char> = candidate.chars().collect();

        if index + letters.len() > chars.len() {
            continue;
        }

        let matched = chars[index..index + letters.len()]
            .iter()
            .zip(&letters)
            .all(|(found, expected)| found.eq_ignore_ascii_case(expected));

        if matched {
            return Some((kind, letters.len()));
        }
    }

    None
}

/// Renders a serial number as a date.
///
/// `None` when the value is not a date at all — anything before the start of the date system, or past
/// the end of the calendar — so the caller falls back to the plain rendering.
pub(super) fn render_date(value: f64, tokens: &[DateToken]) -> Option<String> {
    let date = try_to_datetime(value)?;

    // An AM/PM marker is what makes the hours count a twelve-hour clock.
    let twelve_hour = tokens
        .iter()
        .any(|token| matches!(token.kind, DateTokenKind::AmPm | DateTokenKind::ApMarker));

    let mut text = String::with_capacity(tokens.len() * 2);

    for index in 0..tokens.len() {
        append_date_token(&mut text, tokens, index, date, value, twelve_hour);
    }

    Some(text)
}

/// Emits one token into the rendered text.
fn append_date_token(
    text: &mut String,
    tokens: &[DateToken],
    index: usize,
    date: CivilDateTime,
    serial: f64,
    twelve_hour: bool,
) {
    let token = &tokens[index];

    match token.kind {
        DateTokenKind::Literal => text.push_str(&token.literal),

        DateTokenKind::YearFourDigit => text.push_str(&format!("{:04}", date.year)),

        DateTokenKind::YearTwoDigit => text.push_str(&format!("{:02}", date.year % 100)),

        DateTokenKind::MonthName => text.push_str(month_name(date.month)),

        DateTokenKind::MonthShortName => text.push_str(&abbreviate(month_name(date.month))),

        DateTokenKind::MonthOrMinute => {
            let value = if is_minute(tokens, index) {
                date.minute
            } else {
                date.month
            };

            text.push_str(&pad(value, token.padded));
        }

        DateTokenKind::DayNumber => text.push_str(&pad(date.day, token.padded)),

        DateTokenKind::DayName => text.push_str(day_name(date.weekday)),

        DateTokenKind::DayShortName => text.push_str(&abbreviate(day_name(date.weekday))),

        // A twelve-hour clock counts 12, 1, 2, ... rather than 0, 1, 2, ...
        DateTokenKind::Hour => {
            let value = if twelve_hour {
                ((date.hour + 11) % 12) + 1
            } else {
                date.hour
            };

            text.push_str(&pad(value, token.padded));
        }

        DateTokenKind::Second => text.push_str(&pad(date.second, token.padded)),

        DateTokenKind::FractionalSecond => {
            // Read from the fraction of the day rather than from the decoded date: the serial is
            // rounded to the millisecond on the whole value, which can be a millisecond out by here.
            let fraction = serial - serial.floor();
            let whole = (fraction * 86_400.0) % 1.0;
            let milliseconds = ((whole * 1000.0).round() as i64).clamp(0, 999);
            let digits = token.digits.clamp(1, 3);
            let text_of_millis = format!("{milliseconds:03}");

            text.push_str(&text_of_millis[..digits]);
        }

        DateTokenKind::AmPm => text.push_str(if date.hour < 12 { "AM" } else { "PM" }),

        DateTokenKind::ApMarker => text.push(if date.hour < 12 { 'A' } else { 'P' }),
    }
}

/// True when a run of `m`s stands for minutes: a spreadsheet reads it that way next to an hour or
/// before seconds, and as a month everywhere else.
fn is_minute(tokens: &[DateToken], index: usize) -> bool {
    for before in (0..index).rev() {
        match tokens[before].kind {
            DateTokenKind::Literal => continue,
            DateTokenKind::Hour => return true,
            _ => break,
        }
    }

    for token in tokens.iter().skip(index + 1) {
        if token.kind == DateTokenKind::Literal {
            continue;
        }

        return token.kind == DateTokenKind::Second;
    }

    false
}

/// Pads to two digits when the token was written with two or more letters.
fn pad(value: u32, padded: bool) -> String {
    if padded {
        format!("{value:02}")
    } else {
        value.to_string()
    }
}

/// The first three characters of a name, which is what `mmm` and `ddd` mean.
fn abbreviate(name: &str) -> String {
    name.chars().take(3).collect()
}
