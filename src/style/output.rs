//! Clean, minimal output formatting.
//!
//! Design principles (inspired by ripgrep, bat, eza):
//! - Data is primary, chrome is secondary
//! - Consistent column alignment
//! - Simple separators over heavy boxes
//! - Color conveys meaning, not decoration

use super::theme;
use std::fmt::Write;

/// Standard column widths for consistent alignment.
pub mod cols {
    pub const PROJECT: usize = 16;
    pub const LABEL: usize = 12;
    pub const TIME: usize = 10;
    pub const SESSION: usize = 8;
    pub const COUNT: usize = 6;
}

/// A minimal section header with underline.
pub fn header(title: &str) -> String {
    let styled = theme::style_header(title);
    let line = theme::dim(&"─".repeat(title.len()));
    format!("{styled}\n{line}")
}

/// A subtle section header (just bold, no line).
pub fn section(title: &str) -> String {
    theme::bold(title)
}

/// Format a row with consistent column alignment.
/// Takes pairs of (value, width) for each column.
pub fn row(columns: &[(&str, usize)]) -> String {
    columns
        .iter()
        .map(|(val, width)| format!("{val:<width$}"))
        .collect::<Vec<_>>()
        .join("  ")
}

/// A dimmed separator line.
pub fn separator(width: usize) -> String {
    theme::dim(&"─".repeat(width))
}

/// Format a key-value pair on one line.
pub fn field(key: &str, value: &str) -> String {
    format!("{} {}", theme::dim(&format!("{key}:")), value)
}

/// Indent text by n spaces.
pub fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A clean list item with bullet.
pub fn item(text: &str) -> String {
    format!("  {} {text}", theme::dim("·"))
}

/// Format a count with label.
pub fn count(n: usize, singular: &str, plural: &str) -> String {
    let label = if n == 1 { singular } else { plural };
    format!("{} {label}", theme::bold(&n.to_string()))
}

/// Build a simple table output.
pub struct Table {
    headers: Vec<(String, usize)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[(&str, usize)]) -> Self {
        Self {
            headers: headers
                .iter()
                .map(|(h, w)| ((*h).to_string(), *w))
                .collect(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, values: &[&str]) {
        self.rows
            .push(values.iter().map(|s| (*s).to_string()).collect());
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        // Header row
        let header_row: String = self
            .headers
            .iter()
            .map(|(h, w)| {
                let styled = theme::dim(h);
                format!("{styled:<w$}")
            })
            .collect::<Vec<_>>()
            .join("  ");
        let _ = writeln!(out, "{header_row}");

        // Separator
        let sep_width: usize = self.headers.iter().map(|(_, w)| w + 2).sum::<usize>() - 2;
        let _ = writeln!(out, "{}", separator(sep_width));

        // Data rows
        for row in &self.rows {
            let row_str: String = row
                .iter()
                .zip(&self.headers)
                .map(|(val, (_, w))| format!("{val:<w$}"))
                .collect::<Vec<_>>()
                .join("  ");
            let _ = writeln!(out, "{row_str}");
        }

        out
    }
}

/// A minimal progress/proportion display.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return theme::dim(&"·".repeat(width));
    }

    let ratio = (current as f64 / total as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    format!(
        "{}{}",
        theme::success(&"━".repeat(filled)),
        theme::dim(&"─".repeat(empty))
    )
}

/// Format bytes as human-readable size.
pub fn size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a relative time string.
pub fn relative_time(timestamp: Option<&str>) -> String {
    use chrono::{DateTime, Utc};

    let Some(ts) = timestamp else {
        return "unknown".to_string();
    };

    let Ok(dt) = ts.parse::<DateTime<Utc>>() else {
        return "unknown".to_string();
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    let secs = duration.num_seconds();
    if secs < 0 {
        return "future".to_string();
    }

    let mins = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else if days < 30 {
        format!("{}w ago", days / 7)
    } else {
        format!("{}mo ago", days / 30)
    }
}
