//! The test a `COUNTIF`, `SUMIF` or `AVERAGEIF` applies to each cell.
//!
//! A criterion arrives as a value — a number, or text such as `">=10"` or `"<>closed"` — so it is
//! interpreted once and then tested against every cell rather than being re-read each time.

use pgsheets_core::CellValue;

use super::helpers::compare_ignore_case;
use super::wildcards::wildcard_match;

/// The comparison a criterion applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CriteriaOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// A criterion, read once.
pub(super) struct Criteria {
    operator: CriteriaOperator,
    number: f64,
    is_numeric: bool,
    text: String,
}

impl Criteria {
    /// Reads a criterion: an optional comparison prefix, then a number or some text.
    pub(super) fn parse(value: &CellValue) -> Self {
        let CellValue::Text(text) = value else {
            // A bare number or boolean is an equality test.
            return match value.as_number() {
                Some(number) => Self {
                    operator: CriteriaOperator::Equal,
                    number,
                    is_numeric: true,
                    text: String::new(),
                },
                None => Self {
                    operator: CriteriaOperator::Equal,
                    number: 0.0,
                    is_numeric: false,
                    text: value.as_text().to_string(),
                },
            };
        };

        let (operator, operand) = split_operator(text);

        // The operand is read with the same leniency every other numeric argument gets.
        match CellValue::text(operand).as_number() {
            Some(number) => Self {
                operator,
                number,
                is_numeric: true,
                text: String::new(),
            },
            None => Self {
                operator,
                number: 0.0,
                is_numeric: false,
                text: operand.to_string(),
            },
        }
    }

    /// True when a cell satisfies the criterion.
    pub(super) fn matches(&self, value: &CellValue) -> bool {
        if value.is_error() {
            return false;
        }

        if self.is_numeric {
            // An empty cell counts as zero here, which is what a spreadsheet does.
            return compare(
                self.operator,
                match value.as_number() {
                    Some(number) => number.partial_cmp(&self.number),
                    None => None,
                },
            );
        }

        let text = value.as_text();

        if matches!(
            self.operator,
            CriteriaOperator::Equal | CriteriaOperator::NotEqual
        ) {
            let equal = wildcard_match(&text, &self.text);

            return if self.operator == CriteriaOperator::Equal {
                equal
            } else {
                !equal
            };
        }

        compare(self.operator, Some(compare_ignore_case(&text, &self.text)))
    }
}

/// Splits a leading comparison out of a criterion.
///
/// Two-character operators are tested first, because their one-character prefixes would otherwise
/// match and leave the other character in the operand.
fn split_operator(text: &str) -> (CriteriaOperator, &str) {
    for (prefix, operator) in [
        ("<>", CriteriaOperator::NotEqual),
        (">=", CriteriaOperator::GreaterThanOrEqual),
        ("<=", CriteriaOperator::LessThanOrEqual),
        ("=", CriteriaOperator::Equal),
        (">", CriteriaOperator::GreaterThan),
        ("<", CriteriaOperator::LessThan),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return (operator, rest);
        }
    }

    (CriteriaOperator::Equal, text)
}

/// Applies a comparison, treating "unordered" — only possible when a value has no numeric reading at
/// all — as a failure of every operator.
fn compare(operator: CriteriaOperator, ordering: Option<core::cmp::Ordering>) -> bool {
    use core::cmp::Ordering;

    let Some(ordering) = ordering else {
        return false;
    };

    match operator {
        CriteriaOperator::Equal => ordering == Ordering::Equal,
        CriteriaOperator::NotEqual => ordering != Ordering::Equal,
        CriteriaOperator::LessThan => ordering == Ordering::Less,
        CriteriaOperator::LessThanOrEqual => ordering != Ordering::Greater,
        CriteriaOperator::GreaterThan => ordering == Ordering::Greater,
        CriteriaOperator::GreaterThanOrEqual => ordering != Ordering::Less,
    }
}
