//! Interpreting raw input as a typed value.

use pgsheets_core::{CellValue, parse_literal};

#[test]
fn empty_input_is_empty() {
    assert!(parse_literal("").is_empty());
}

#[test]
fn numeric_input_is_parsed_with_the_invariant_culture() {
    for (raw, expected) in [
        ("1200", 1200.0),
        ("-3.5", -3.5),
        ("1e3", 1000.0),
        ("1,234", 1234.0),
    ] {
        let value = parse_literal(raw);

        assert_eq!(value.as_number(), Some(expected), "parsing '{raw}'");
        assert!(!value.is_error());
    }
}

#[test]
fn booleans_are_case_insensitive() {
    for raw in ["TRUE", "true", "True"] {
        assert_eq!(parse_literal(raw), CellValue::boolean(true), "parsing '{raw}'");
    }

    assert_eq!(parse_literal("false"), CellValue::boolean(false));
}

#[test]
fn everything_else_is_text() {
    assert_eq!(parse_literal("North"), CellValue::text("North"));
}

#[test]
fn a_stray_formula_is_preserved_as_text() {
    // Should never reach here through the engine, but must not throw or lose data.
    assert_eq!(parse_literal("=1+1"), CellValue::text("=1+1"));
}
