//! Shared utility functions.

/// Truncates text to approximately `max_len` characters, breaking at word boundary.
///
/// Normalizes whitespace and ensures clean truncation at word boundaries
/// when possible. Returns the truncated string with "..." appended if truncated.
///
/// # Examples
///
/// ```
/// use glhf::utils::truncate_text;
///
/// assert_eq!(truncate_text("hello", 10), "hello");
/// assert_eq!(truncate_text("hello world this is long", 15), "hello world...");
/// ```
pub fn truncate_text(content: &str, max_len: usize) -> String {
    // Normalize whitespace
    let words: Vec<&str> = content.split_whitespace().collect();
    let normalized = words.join(" ");

    let char_count = normalized.chars().count();
    if char_count <= max_len {
        return normalized;
    }

    // Build up result word by word until we exceed max_len
    let mut result = String::new();
    for word in words {
        let new_len = if result.is_empty() {
            word.chars().count()
        } else {
            result.chars().count() + 1 + word.chars().count()
        };

        if new_len > max_len {
            break;
        }

        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(word);
    }

    if result.is_empty() {
        // Single word too long - just take first max_len chars
        format!(
            "{}...",
            normalized.chars().take(max_len).collect::<String>()
        )
    } else {
        format!("{result}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_unchanged() {
        assert_eq!(truncate_text("hello", 10), "hello");
    }

    #[test]
    fn test_truncates_at_word_boundary() {
        let result = truncate_text("hello world this is a test", 15);
        assert_eq!(result, "hello world...");
    }

    #[test]
    fn test_normalizes_whitespace() {
        let result = truncate_text("hello    world\n\ntest", 100);
        assert_eq!(result, "hello world test");
    }

    #[test]
    fn test_single_long_word() {
        let result = truncate_text("superlongwordthatexceedslimit", 10);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 13); // 10 chars + "..."
    }
}
