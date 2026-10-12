//! The typed cell value: its display form, its coercions and its equality.

use pgsheets_core::{CellError, CellValue};

#[test]
fn empty_is_the_default() {
    let value = CellValue::default();

    assert!(value.is_empty());
    assert!(!value.is_error());
    assert_eq!(value.to_display_string(), String::new());
}

#[test]
fn text_of_empty_collapses_to_empty() {
    assert!(CellValue::text("").is_empty());
}

#[test]
fn errors_display_their_excel_code() {
    for (error, expected) in [
        (CellError::DivideByZero, "#DIV/0!"),
        (CellError::Value, "#VALUE!"),
        (CellError::Reference, "#REF!"),
        (CellError::Name, "#NAME?"),
        (CellError::Number, "#NUM!"),
        (CellError::Circular, "#CIRCULAR!"),
        (CellError::NotAvailable, "#N/A"),
    ] {
        let value = CellValue::error(error);

        assert!(value.is_error());
        assert_eq!(value.to_display_string(), expected);
        assert_eq!(value.error_code(), Some(error));
    }
}

#[test]
fn booleans_display_uppercase() {
    assert_eq!(CellValue::boolean(true).to_display_string(), "TRUE");
    assert_eq!(CellValue::boolean(false).to_display_string(), "FALSE");
}

#[test]
fn numbers_avoid_floating_point_noise() {
    assert_eq!(CellValue::number(0.1 + 0.2).to_display_string(), "0.3");
}

#[test]
fn numbers_format_with_the_invariant_culture() {
    assert_eq!(CellValue::number(1234.5).to_display_string(), "1234.5");
    assert_eq!(CellValue::number(-0.0).to_display_string(), "-0");
    assert_eq!(CellValue::number(1e15).to_display_string(), "1E+15");
}

#[test]
fn numeric_coercion_handles_empty_and_booleans() {
    assert_eq!(CellValue::Empty.as_number(), Some(0.0));
    assert_eq!(CellValue::boolean(true).as_number(), Some(1.0));
    assert_eq!(CellValue::boolean(false).as_number(), Some(0.0));
}

#[test]
fn numeric_coercion_reads_numeric_text() {
    assert_eq!(CellValue::text("42.5").as_number(), Some(42.5));
    assert_eq!(CellValue::text("North").as_number(), None);

    // Group separators are accepted, as they are by the original implementation.
    assert_eq!(CellValue::text("1,234").as_number(), Some(1234.0));
}

#[test]
fn numeric_coercion_rejects_errors() {
    assert_eq!(CellValue::error(CellError::Value).as_number(), None);
}

#[test]
fn the_zero_fallback_is_what_it_says() {
    assert_eq!(CellValue::text("North").as_number_or_zero(), 0.0);
    assert_eq!(CellValue::number(7.0).as_number_or_zero(), 7.0);
}

#[test]
fn booleans_are_interpreted_as_conditions() {
    assert!(CellValue::boolean(true).as_boolean());
    assert!(!CellValue::boolean(false).as_boolean());
    assert!(CellValue::number(1.0).as_boolean());
    assert!(!CellValue::number(0.0).as_boolean());
    assert!(CellValue::text("TRUE").as_boolean());
    assert!(!CellValue::Empty.as_boolean());
}

#[test]
fn equality_is_structural() {
    assert_eq!(CellValue::number(3.0), CellValue::number(3.0));
    assert_ne!(CellValue::number(3.0), CellValue::number(4.0));
    assert_eq!(CellValue::text("a"), CellValue::text("a"));
    assert_ne!(CellValue::text("a"), CellValue::text("b"));
}
