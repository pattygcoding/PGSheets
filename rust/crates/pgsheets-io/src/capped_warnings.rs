//! Warning bookkeeping shared by the import and export results.
//!
//! Warnings are capped so a pathological file — thousands of unsupported functions, say — cannot
//! exhaust memory, while a final entry still tells the reader that the list was truncated.

/// Message appended once the cap is reached.
pub const SUPPRESSED_MESSAGE: &str = "Further warnings suppressed.";

/// Most warnings a result keeps before it starts summarising instead.
const MAX_WARNINGS: usize = 100;

/// Adds a warning, or the truncation notice once the cap is reached.
pub fn add_warning(warnings: &mut Vec<String>, message: impl Into<String>) {
    match warnings.len() {
        len if len < MAX_WARNINGS => warnings.push(message.into()),
        len if len == MAX_WARNINGS => warnings.push(SUPPRESSED_MESSAGE.to_string()),
        _ => {}
    }
}
