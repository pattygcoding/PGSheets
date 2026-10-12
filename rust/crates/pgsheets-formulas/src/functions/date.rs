//! Functions that read a date.
//!
//! A date is accepted either as a serial number, which is how a spreadsheet stores one, or as the ISO
//! text this app's importer produces for a date-formatted cell — so these work on a real workbook
//! today and keep working once a number format is what turns a serial back into something that reads
//! like a date.

use pgsheets_core::serial_dates::{CivilDateTime, try_to_datetime};
use pgsheets_core::{CellError, CellValue};

use super::helpers::try_integer;
use crate::ast::{AstNode, FormulaContext};

/// Which part of a date to report.
#[derive(Clone, Copy)]
enum DatePart {
    Year,
    Month,
    Day,
}

pub(super) fn year(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    date_part_of(arguments, context, DatePart::Year)
}

pub(super) fn month(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    date_part_of(arguments, context, DatePart::Month)
}

pub(super) fn day(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    date_part_of(arguments, context, DatePart::Day)
}

fn date_part_of(arguments: &[AstNode], context: &mut dyn FormulaContext, part: DatePart) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let date = match try_date(&arguments[0], context) {
        Ok(date) => date,
        Err(error) => return error,
    };

    let value = match part {
        DatePart::Year => date.year,
        DatePart::Month => date.month as i32,
        DatePart::Day => date.day as i32,
    };

    CellValue::number(f64::from(value))
}

/// `WEEKDAY(date, [type])` — 1 to 7, with the type choosing where the week starts.
pub(super) fn week_day(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let date = match try_date(&arguments[0], context) {
        Ok(date) => date,
        Err(error) => return error,
    };

    let day_type = if arguments.len() == 2 {
        match try_integer(arguments, 1, context) {
            Ok(day_type) => day_type,
            Err(error) => return error,
        }
    } else {
        1
    };

    // The weekday counts up from Sunday, which is what WEEKDAY's default type reports.
    let from_sunday = date.weekday as i32;

    match day_type {
        1 => CellValue::number(f64::from(from_sunday + 1)),
        2 => CellValue::number(f64::from(((from_sunday + 6) % 7) + 1)),
        3 => CellValue::number(f64::from((from_sunday + 6) % 7)),
        _ => CellValue::error(CellError::Number),
    }
}

/// Reads an argument as a date, from either a serial number or ISO text.
fn try_date(node: &AstNode, context: &mut dyn FormulaContext) -> Result<CivilDateTime, CellValue> {
    let value = node.evaluate(context);

    if value.is_error() {
        return Err(value);
    }

    if let CellValue::Text(text) = &value {
        // A date-formatted cell from a real workbook arrives as text, so it has to be readable.
        return parse_iso_datetime(text).ok_or_else(|| CellValue::error(CellError::Value));
    }

    let Some(serial) = value.as_number() else {
        return Err(CellValue::error(CellError::Value));
    };

    try_to_datetime(serial).ok_or_else(|| CellValue::error(CellError::Number))
}

/// Reads the shapes a date can arrive in as text: `yyyy-MM-dd`, optionally followed by a space or a
/// `T` and `HH:mm:ss`, and optionally by a trailing `Z`.
///
/// Deliberately exact rather than a general parse, so a cell holding "2024" is not silently read as a
/// year.
fn parse_iso_datetime(text: &str) -> Option<CivilDateTime> {
    let text = text.trim();
    let bytes = text.as_bytes();

    if bytes.len() < 10 {
        return None;
    }

    let year = digits(&bytes[0..4])? as i32;

    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }

    let month = digits(&bytes[5..7])?;
    let day = digits(&bytes[8..10])?;

    // A date that does not exist has to be refused here, because the arithmetic below would happily
    // roll it into the next month.
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }

    let date = CivilDateTime::date(year, month, day);

    if bytes.len() == 10 {
        return Some(date);
    }

    if !matches!(bytes[10], b' ' | b'T') {
        return None;
    }

    // A trailing 'Z' is accepted and ignored: it names a zone, not a different clock reading.
    if bytes.len() == 20 && bytes[19] == b'Z' {
        // handled below by the length check
    } else if bytes.len() != 19 {
        return None;
    }

    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }

    let hour = digits(&bytes[11..13])?;
    let minute = digits(&bytes[14..16])?;
    let second = digits(&bytes[17..19])?;

    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    Some(date.at(hour, minute, second, 0))
}

/// Reads a run of ASCII digits.
fn digits(bytes: &[u8]) -> Option<u32> {
    let mut value = 0;

    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }

        value = value * 10 + u32::from(byte - b'0');
    }

    Some(value)
}

/// The number of days in a month, with the Gregorian leap-year rule.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}
