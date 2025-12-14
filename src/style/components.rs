//! Reusable styled UI components.
//!
//! Provides both "heavy" (boxed) and "light" (minimal) output styles.
//! Prefer light styles for cleaner output.

use super::{ctx, theme};
use std::borrow::Cow;
use std::fmt::Write;

/// Box drawing characters with unicode/ascii variants.
pub struct BoxChars;

impl BoxChars {
    fn chars() -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        if ctx().unicode {
            ("╭", "╮", "╰", "╯", "─", "│")
        } else {
            ("+", "+", "+", "+", "-", "|")
        }
    }

    pub fn top_left() -> &'static str {
        Self::chars().0
    }
    pub fn top_right() -> &'static str {
        Self::chars().1
    }
    pub fn bottom_left() -> &'static str {
        Self::chars().2
    }
    pub fn bottom_right() -> &'static str {
        Self::chars().3
    }
    pub fn horizontal() -> &'static str {
        Self::chars().4
    }
    pub fn vertical() -> &'static str {
        Self::chars().5
    }
}

/// Create a horizontal divider.
pub fn divider(width: usize) -> String {
    theme::dim(&BoxChars::horizontal().repeat(width))
}

/// Create a styled header box (heavy style).
pub fn header_box(title: &str) -> String {
    let width = super::term_width().min(70);
    let inner_width = width.saturating_sub(4);

    let title_display = if title.len() > inner_width {
        format!("{}...", &title[..inner_width.saturating_sub(3)])
    } else {
        title.to_string()
    };
    let padding = inner_width.saturating_sub(title_display.len());
    let h_line = BoxChars::horizontal().repeat(width.saturating_sub(2));

    format!(
        "{}{}{}\n{}  {}{}  {}\n{}{}{}",
        theme::dim(BoxChars::top_left()),
        theme::dim(&h_line),
        theme::dim(BoxChars::top_right()),
        theme::dim(BoxChars::vertical()),
        theme::style_header(&title_display),
        " ".repeat(padding),
        theme::dim(BoxChars::vertical()),
        theme::dim(BoxChars::bottom_left()),
        theme::dim(&h_line),
        theme::dim(BoxChars::bottom_right()),
    )
}

/// Create a content box with wrapped text (heavy style).
pub fn content_box(content: &str, max_width: usize) -> String {
    let width = max_width.min(super::term_width().saturating_sub(4));
    let inner_width = width.saturating_sub(4);

    let wrapped: Vec<Cow<'_, str>> = textwrap::wrap(content, inner_width);
    let h_line = BoxChars::horizontal().repeat(width.saturating_sub(2));

    let mut output = String::new();
    let _ = writeln!(
        output,
        "  {}{}{}",
        theme::dim(BoxChars::top_left()),
        theme::dim(&h_line),
        theme::dim(BoxChars::top_right())
    );

    if wrapped.is_empty() {
        let _ = writeln!(
            output,
            "  {} {} {}",
            theme::dim(BoxChars::vertical()),
            " ".repeat(inner_width),
            theme::dim(BoxChars::vertical())
        );
    } else {
        for line in &wrapped {
            let visible_len = console::measure_text_width(line);
            let padding = inner_width.saturating_sub(visible_len);
            let _ = writeln!(
                output,
                "  {} {}{} {}",
                theme::dim(BoxChars::vertical()),
                line,
                " ".repeat(padding),
                theme::dim(BoxChars::vertical())
            );
        }
    }

    let _ = write!(
        output,
        "  {}{}{}",
        theme::dim(BoxChars::bottom_left()),
        theme::dim(&h_line),
        theme::dim(BoxChars::bottom_right())
    );

    output
}

/// Create a progress bar visualization.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn progress_bar(current: usize, total: usize, width: usize) -> String {
    let (filled_char, empty_char) = if ctx().unicode {
        ("━", "─")
    } else {
        ("#", "-")
    };

    if total == 0 {
        return theme::dim(&empty_char.repeat(width));
    }

    let ratio = (current as f64 / total as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    if super::should_style() {
        format!(
            "{}{}",
            theme::success(&filled_char.repeat(filled)),
            theme::dim(&empty_char.repeat(empty))
        )
    } else {
        format!("{}{}", filled_char.repeat(filled), empty_char.repeat(empty))
    }
}

/// Create a score visualization bar.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn score_bar(score: f64, width: usize) -> String {
    let (filled_char, empty_char) = if ctx().unicode {
        ("━", "─")
    } else {
        ("#", "-")
    };

    let filled = (score.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", filled_char.repeat(filled), empty_char.repeat(empty))
}

/// Format a label with consistent left padding.
pub fn padded_label(label: &str, width: usize) -> String {
    if label.len() >= width {
        label[..width].to_string()
    } else {
        format!("{label:<width$}")
    }
}

/// Format a right-aligned value.
pub fn right_align(value: &str, width: usize) -> String {
    format!("{value:>width$}")
}
