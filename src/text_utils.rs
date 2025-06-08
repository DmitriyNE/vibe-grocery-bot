use tracing::trace;

/// Clean a single text line from a user message.
///
/// Returns `None` if the line should be ignored (for example it is the
/// archived list separator or becomes empty after trimming). Otherwise returns
/// the cleaned line without leading status emojis or whitespace.
pub fn parse_item_line(line: &str) -> Option<String> {
    trace!(?line, "Parsing item line");
    if line.trim() == "--- Archived List ---" {
        trace!("Ignoring archived list separator");
        return None;
    }

    let cleaned = line
        .trim_start_matches(['☑', '✅', '⬜', '🛒', '•', '🗑', '\u{fe0f}'])
        .trim();

    if cleaned.starts_with("Removed via voice request") {
        trace!("Ignoring removal header");
        return None;
    }

    if cleaned.is_empty() {
        trace!("Line empty after cleaning");
        None
    } else {
        let result = cleaned.to_string();
        trace!(?result, "Parsed line");
        Some(result)
    }
}

pub fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Normalize an item string for matching operations.
///
/// This removes any leading quantity digits and whitespace and
/// lowercases the rest so lookups are more tolerant to voice command
/// variations.
pub fn normalize_for_match(text: &str) -> String {
    let trimmed = text
        .trim_start_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
        .trim();
    let result = trimmed.to_lowercase();
    trace!(original = %text, normalized = %result, "normalized for match");
    result
}
