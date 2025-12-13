//! Color themes for terminal output.

use owo_colors::{OwoColorize, Style};
use std::sync::OnceLock;

/// Cached theme instance.
static THEME: OnceLock<Theme> = OnceLock::new();

/// Semantic colors for different content types.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub user: Style,
    pub assistant: Style,
    pub tool_use: Style,
    pub tool_result: Style,
    pub error: Style,
    pub dim: Style,
    pub bold: Style,
    pub project: Style,
    pub session: Style,
    pub time: Style,
    pub score: Style,
    pub header: Style,
    pub success: Style,
    pub warning: Style,
}

impl Theme {
    /// The default dark theme.
    fn dark() -> Self {
        Self {
            user: Style::new().cyan(),
            assistant: Style::new().magenta(),
            tool_use: Style::new().yellow(),
            tool_result: Style::new().green(),
            error: Style::new().red().bold(),
            dim: Style::new().dimmed(),
            bold: Style::new().bold(),
            project: Style::new().white().bold(),
            session: Style::new().cyan().dimmed(),
            time: Style::new().dimmed(),
            score: Style::new().blue(),
            header: Style::new().bold().cyan(),
            success: Style::new().green().bold(),
            warning: Style::new().yellow(),
        }
    }

    /// A plain theme with no colors (for accessibility or non-TTY).
    fn plain() -> Self {
        Self {
            user: Style::new(),
            assistant: Style::new(),
            tool_use: Style::new(),
            tool_result: Style::new(),
            error: Style::new().bold(),
            dim: Style::new(),
            bold: Style::new().bold(),
            project: Style::new().bold(),
            session: Style::new(),
            time: Style::new(),
            score: Style::new(),
            header: Style::new().bold(),
            success: Style::new().bold(),
            warning: Style::new().bold(),
        }
    }
}

/// Get the cached theme based on style context.
fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        if super::should_style() {
            Theme::dark()
        } else {
            Theme::plain()
        }
    })
}

// Helper macro to reduce boilerplate for style functions
macro_rules! style_fn {
    ($name:ident, $field:ident) => {
        pub fn $name(text: &str) -> String {
            text.style(theme().$field).to_string()
        }
    };
}

style_fn!(dim, dim);
style_fn!(bold, bold);
style_fn!(success, success);
style_fn!(warning, warning);
style_fn!(style_error, error);
style_fn!(style_time, time);
style_fn!(style_session, session);
style_fn!(style_project, project);
style_fn!(style_header, header);
style_fn!(style_tool, tool_use);

/// Style a string based on chunk kind.
pub fn style_chunk_kind(kind: &str, text: &str) -> String {
    let t = theme();
    let style = match kind {
        "message" => t.assistant,
        "tool_use" => t.tool_use,
        "tool_result" => t.tool_result,
        _ => t.dim,
    };
    text.style(style).to_string()
}

/// Style a string based on role.
pub fn style_role(role: &str, text: &str) -> String {
    let t = theme();
    let style = match role {
        "user" => t.user,
        "assistant" => t.assistant,
        _ => t.dim,
    };
    text.style(style).to_string()
}

/// Style a score value with visual bar.
pub fn style_score(score: f64) -> String {
    let bar = score_bar(score);
    format!("{:.2} {}", score, bar.style(theme().score))
}

/// Create a visual score bar.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn score_bar(score: f64) -> String {
    let filled = (score.clamp(0.0, 1.0) * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}
