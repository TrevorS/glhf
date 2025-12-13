//! Color themes for terminal output.

use owo_colors::{OwoColorize, Style};

/// Semantic colors for different content types.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// User messages (cyan/teal).
    pub user: Style,
    /// Assistant messages (magenta/pink).
    pub assistant: Style,
    /// Tool use invocations (yellow/gold).
    pub tool_use: Style,
    /// Successful tool results (green).
    pub tool_result: Style,
    /// Error results (red).
    pub error: Style,
    /// Dimmed/secondary text.
    pub dim: Style,
    /// Bold/emphasized text.
    pub bold: Style,
    /// Project names.
    pub project: Style,
    /// Session IDs.
    pub session: Style,
    /// Timestamps.
    pub time: Style,
    /// Search match highlight.
    pub highlight: Style,
    /// Score/relevance.
    pub score: Style,
    /// Border/divider color.
    pub border: Style,
    /// Header/title style.
    pub header: Style,
    /// Success messages.
    pub success: Style,
    /// Warning messages.
    pub warning: Style,
}

impl Theme {
    /// The default dark theme.
    pub fn dark() -> Self {
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
            highlight: Style::new().bold().underline(),
            score: Style::new().blue(),
            border: Style::new().dimmed(),
            header: Style::new().bold().cyan(),
            success: Style::new().green().bold(),
            warning: Style::new().yellow(),
        }
    }

    /// A plain theme with no colors (for accessibility).
    pub fn plain() -> Self {
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
            highlight: Style::new().bold(),
            score: Style::new(),
            border: Style::new(),
            header: Style::new().bold(),
            success: Style::new().bold(),
            warning: Style::new().bold(),
        }
    }
}

/// Get the current theme based on style context.
pub fn current() -> Theme {
    if super::should_style() {
        Theme::dark()
    } else {
        Theme::plain()
    }
}

/// Style a string based on chunk kind.
pub fn style_chunk_kind(kind: &str, text: &str) -> String {
    let theme = current();
    let styled = match kind {
        "message" => text.style(theme.assistant),
        "tool_use" => text.style(theme.tool_use),
        "tool_result" => text.style(theme.tool_result),
        _ => text.style(theme.dim),
    };
    styled.to_string()
}

/// Style a string based on role.
pub fn style_role(role: &str, text: &str) -> String {
    let theme = current();
    let styled = match role {
        "user" => text.style(theme.user),
        "assistant" => text.style(theme.assistant),
        _ => text.style(theme.dim),
    };
    styled.to_string()
}

/// Style a tool name.
pub fn style_tool(text: &str) -> String {
    let theme = current();
    text.style(theme.tool_use).to_string()
}

/// Style an error.
pub fn style_error(text: &str) -> String {
    let theme = current();
    text.style(theme.error).to_string()
}

/// Style a timestamp.
pub fn style_time(text: &str) -> String {
    let theme = current();
    text.style(theme.time).to_string()
}

/// Style a session ID.
pub fn style_session(text: &str) -> String {
    let theme = current();
    text.style(theme.session).to_string()
}

/// Style a project name.
pub fn style_project(text: &str) -> String {
    let theme = current();
    text.style(theme.project).to_string()
}

/// Style a header/title.
pub fn style_header(text: &str) -> String {
    let theme = current();
    text.style(theme.header).to_string()
}

/// Style a score value with color gradient.
pub fn style_score(score: f64) -> String {
    let theme = current();
    let bar = score_bar(score);
    format!("{:.2} {}", score, bar.style(theme.score))
}

/// Create a visual score bar.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn score_bar(score: f64) -> String {
    let filled = (score.max(0.0) * 10.0).round() as usize;
    let empty = 10_usize.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Style dimmed text.
pub fn dim(text: &str) -> String {
    let theme = current();
    text.style(theme.dim).to_string()
}

/// Style bold text.
pub fn bold(text: &str) -> String {
    let theme = current();
    text.style(theme.bold).to_string()
}

/// Style success text.
pub fn success(text: &str) -> String {
    let theme = current();
    text.style(theme.success).to_string()
}

/// Style warning text.
pub fn warning(text: &str) -> String {
    let theme = current();
    text.style(theme.warning).to_string()
}
