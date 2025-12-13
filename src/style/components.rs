//! Reusable styled UI components.

use super::{ctx, theme};
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement,
    Table,
};
use std::fmt::Write;

/// Box drawing characters for UI components.
pub struct Box;

impl Box {
    /// Top-left corner.
    pub fn tl() -> &'static str {
        if ctx().unicode {
            "╭"
        } else {
            "+"
        }
    }

    /// Top-right corner.
    pub fn tr() -> &'static str {
        if ctx().unicode {
            "╮"
        } else {
            "+"
        }
    }

    /// Bottom-left corner.
    pub fn bl() -> &'static str {
        if ctx().unicode {
            "╰"
        } else {
            "+"
        }
    }

    /// Bottom-right corner.
    pub fn br() -> &'static str {
        if ctx().unicode {
            "╯"
        } else {
            "+"
        }
    }

    /// Horizontal line.
    pub fn h() -> &'static str {
        if ctx().unicode {
            "─"
        } else {
            "-"
        }
    }

    /// Vertical line.
    pub fn v() -> &'static str {
        if ctx().unicode {
            "│"
        } else {
            "|"
        }
    }

    /// Light horizontal (for dividers).
    pub fn h_light() -> &'static str {
        if ctx().unicode {
            "─"
        } else {
            "-"
        }
    }
}

/// Create a horizontal divider.
pub fn divider(width: usize) -> String {
    theme::dim(&Box::h().repeat(width))
}

/// Create a styled header box.
pub fn header_box(title: &str) -> String {
    let width = super::term_width().min(70);
    let inner_width = width - 4; // Account for corners and padding
    let title_display = if title.len() > inner_width {
        format!("{}...", &title[..inner_width - 3])
    } else {
        title.to_string()
    };
    let padding = inner_width.saturating_sub(title_display.len());

    format!(
        "{}{}{}\n{}  {}{}  {}\n{}{}{}",
        theme::dim(Box::tl()),
        theme::dim(&Box::h().repeat(width - 2)),
        theme::dim(Box::tr()),
        theme::dim(Box::v()),
        theme::style_header(&title_display),
        " ".repeat(padding),
        theme::dim(Box::v()),
        theme::dim(Box::bl()),
        theme::dim(&Box::h().repeat(width - 2)),
        theme::dim(Box::br()),
    )
}

/// Create a simple header line (less visual noise).
pub fn header_line(title: &str) -> String {
    let styled_title = theme::style_header(title);
    format!("\n{styled_title}\n{}\n", divider(title.len()))
}

/// Create a content box.
pub fn content_box(content: &str, max_width: usize) -> String {
    let width = max_width.min(super::term_width() - 4);
    let lines = wrap_text(content, width - 4);

    let mut output = String::new();
    let _ = writeln!(
        output,
        "  {}{}{}",
        theme::dim(Box::tl()),
        theme::dim(&Box::h().repeat(width - 2)),
        theme::dim(Box::tr())
    );

    for line in lines {
        let padding = width - 4 - visible_width(&line);
        let _ = writeln!(
            output,
            "  {} {}{} {}",
            theme::dim(Box::v()),
            line,
            " ".repeat(padding),
            theme::dim(Box::v())
        );
    }

    let _ = write!(
        output,
        "  {}{}{}",
        theme::dim(Box::bl()),
        theme::dim(&Box::h().repeat(width - 2)),
        theme::dim(Box::br())
    );

    output
}

/// Create a result card.
pub fn result_card(header: &str, content: &str, max_width: usize) -> String {
    format!("  {header}\n{}\n", content_box(content, max_width - 2))
}

/// Create a styled table.
pub fn styled_table() -> Table {
    let mut table = Table::new();
    if ctx().unicode {
        table.load_preset(UTF8_FULL);
        table.apply_modifier(UTF8_ROUND_CORNERS);
    }
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Create a cell with a specific color.
pub fn colored_cell(text: &str, color: Color) -> Cell {
    Cell::new(text).fg(color)
}

/// Create a bold cell.
pub fn bold_cell(text: &str) -> Cell {
    Cell::new(text).add_attribute(Attribute::Bold)
}

/// Create a dimmed cell.
pub fn dim_cell(text: &str) -> Cell {
    Cell::new(text).add_attribute(Attribute::Dim)
}

/// Wrap text to a maximum width.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Get the visible width of a string (accounting for ANSI codes).
fn visible_width(s: &str) -> usize {
    console::measure_text_width(s)
}

/// Create a progress bar visualization.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "░".repeat(width);
    }

    let ratio = (current as f64 / total as f64).min(1.0);
    let filled = (ratio * width as f64).round().max(0.0) as usize;
    let empty = width.saturating_sub(filled);

    if super::should_style() {
        format!(
            "{}{}",
            theme::success(&"█".repeat(filled)),
            theme::dim(&"░".repeat(empty))
        )
    } else {
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

/// Create a score visualization bar.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn score_bar(score: f64, width: usize) -> String {
    let filled = (score.max(0.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar_char = if ctx().unicode { "█" } else { "#" };
    let empty_char = if ctx().unicode { "░" } else { "." };

    format!("{}{}", bar_char.repeat(filled), empty_char.repeat(empty))
}

/// Format a label with consistent padding.
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

/// Create a key-value line.
pub fn key_value(key: &str, value: &str) -> String {
    format!("  {}: {value}", theme::dim(key))
}

/// Create a bullet point.
pub fn bullet_point(text: &str) -> String {
    format!("  {} {text}", if ctx().unicode { "•" } else { "*" })
}

/// Create a numbered item.
pub fn numbered_item(num: usize, text: &str) -> String {
    format!("  [{}] {text}", theme::dim(&num.to_string()))
}
