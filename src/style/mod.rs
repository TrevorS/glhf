//! Terminal styling for beautiful CLI output.
//!
//! This module provides a styling system inspired by Charm Bracelet (Lip Gloss)
//! and Rich/Textual. It automatically detects terminal capabilities and adapts
//! output accordingly.

pub mod components;
pub mod icons;
pub mod theme;

use console::Term;
use std::sync::OnceLock;

/// Global style context, initialized once.
static STYLE_CONTEXT: OnceLock<StyleContext> = OnceLock::new();

/// Terminal styling context.
#[derive(Debug, Clone)]
pub struct StyleContext {
    /// Whether colors are enabled.
    pub colors: bool,
    /// Whether to use Unicode icons (vs ASCII fallbacks).
    pub unicode: bool,
    /// Whether output is to a terminal (TTY).
    pub is_tty: bool,
    /// Terminal width (0 if not a terminal).
    pub width: usize,
}

impl StyleContext {
    /// Detect terminal capabilities and create a style context.
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
        let width = if is_tty {
            term.size().1 as usize
        } else {
            80 // Default for non-TTY
        };

        Self {
            colors,
            unicode,
            is_tty,
            width,
        }
    }

    /// Force colors on (for testing or --color=always).
    pub fn force_colors() -> Self {
        Self {
            colors: true,
            unicode: true,
            is_tty: true,
            width: 80,
        }
    }

    /// Force colors off (for --color=never).
    pub fn no_colors() -> Self {
        Self {
            colors: false,
            unicode: true,
            is_tty: false,
            width: 80,
        }
    }
}

/// Get the global style context.
pub fn ctx() -> &'static StyleContext {
    STYLE_CONTEXT.get_or_init(StyleContext::detect)
}

/// Initialize the style context with custom settings.
/// Must be called before any styling functions if customization is needed.
pub fn init(context: StyleContext) {
    let _ = STYLE_CONTEXT.set(context);
}

/// Check if styled output should be used.
pub fn should_style() -> bool {
    ctx().colors && ctx().is_tty
}

/// Get the terminal width.
pub fn term_width() -> usize {
    ctx().width
}
