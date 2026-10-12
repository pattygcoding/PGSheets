//! Spreadsheet wildcards: `*` for any run of characters, `?` for exactly one, and `~` to escape the
//! next character.
//!
//! Two-pointer backtracking rather than a regular expression, so a `COUNTIF` over a large range
//! compiles nothing and allocates nothing per cell.
//!
//! A wildcard matches one *character* here. The original walked UTF-16 code units, which can only
//! differ for text outside the basic plane, where `?` would match half of a surrogate pair.

/// Matches text against a wildcard pattern.
pub(super) fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();

    let mut text_index = 0;
    let mut pattern_index = 0;
    let mut star = None;
    let mut star_text = 0;

    while text_index < text.len() {
        // Try to consume one pattern token against one character.
        if let Some(next) = token_matches(&pattern, pattern_index, text[text_index]) {
            pattern_index = next;
            text_index += 1;
            continue;
        }

        if pattern.get(pattern_index) == Some(&'*') {
            // Remember the star and try matching the rest with nothing under it first.
            star = Some(pattern_index);
            star_text = text_index;
            pattern_index += 1;
            continue;
        }

        let Some(star) = star else {
            return false;
        };

        // Let the last star swallow one more character and retry from just after it.
        star_text += 1;
        text_index = star_text;
        pattern_index = star + 1;
    }

    // Trailing stars match nothing at all.
    while pattern.get(pattern_index) == Some(&'*') {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

/// Consumes the pattern token at `position` if it matches `ch`, returning where the pattern continues.
///
/// Stars are the caller's business, because they match a variable number of characters.
fn token_matches(pattern: &[char], position: usize, ch: char) -> Option<usize> {
    let token = *pattern.get(position)?;

    if token == '~' && position + 1 < pattern.len() {
        // An escaped character matches literally, and consumes both pattern characters.
        return same_character(ch, pattern[position + 1]).then_some(position + 2);
    }

    if token == '*' {
        return None;
    }

    if token == '?' || same_character(ch, token) {
        return Some(position + 1);
    }

    None
}

/// Case-insensitive character comparison, without allocating.
fn same_character(left: char, right: char) -> bool {
    left.to_uppercase().eq(right.to_uppercase())
}
