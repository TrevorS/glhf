//! Terminal styling for beautiful CLI output.
//!
//! This module provides a styling system inspired by Charm Bracelet (Lip Gloss)
//! and Rich/Textual. It automatically detects terminal capabilities and adapts
//! output accordingly.

pub mod components;
pub mod icons;
pub mod output;
pub mod theme;

use console::Term;
use std::sync::OnceLock;

/// Global style context, initialized once.
static STYLE_CONTEXT: OnceLock<StyleContext> = OnceLock::new();

/// Terminal styling context.
#[derive(Debug, Clone, Copy)]
pub struct StyleContext {
    /// Whether colors are enabled.
    pub colors: bool,
    /// Whether to use Unicode icons (vs ASCII fallbacks).
    pub unicode: bool,
    /// Whether output is to a terminal (TTY).
    pub is_tty: bool,
    /// Terminal width (0 if not a terminal).
    pub width: u16,
}

impl Default for StyleContext {
    fn default() -> Self {
        Self::detect()
    }
}

impl StyleContext {
    /// Detect terminal capabilities and create a style context.
    #[allow(clippy::cast_possible_truncation)]
    pub fn detect() -> Self {
        let term = Term::stdout();
        let is_tty = term.is_term();

        // Respect NO_COLOR environment variable
        let no_color = std::env::var("NO_COLOR").is_ok();

        // Check if colors are supported
        let colors = is_tty && !no_color && console::colors_enabled();

        // Check Unicode support (most modern terminals support it)
        let unicode = is_tty && std::env::var("GLHF_ASCII").is_err();

        // Get terminal width
        let width = if is_tty { term.size().1 } else { 80 };

        Self {
            colors,
            unicode,
            is_tty,
            width,
        }
    }
}

/// Get the global style context.
pub fn ctx() -> &'static StyleContext {
    STYLE_CONTEXT.get_or_init(StyleContext::detect)
}

/// Check if styled output should be used.
pub fn should_style() -> bool {
    let ctx = ctx();
    ctx.colors && ctx.is_tty
}

/// Get the terminal width.
pub fn term_width() -> usize {
    ctx().width as usize
}
