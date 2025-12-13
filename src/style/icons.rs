//! Minimal geometric ideograms with ASCII fallbacks.

use super::ctx;

/// Helper macro to define icon functions with unicode/ascii variants.
macro_rules! icon {
    ($name:ident, $unicode:literal, $ascii:literal) => {
        pub fn $name() -> &'static str {
            if ctx().unicode {
                $unicode
            } else {
                $ascii
            }
        }
    };
}

/// Icons using minimal, sleek ideograms.
pub struct Icons;

impl Icons {
    icon!(user, "◆", "[u]");
    icon!(assistant, "◇", "[a]");
    icon!(tool, "▸", ">");
    icon!(result, "✓", "+");
    icon!(error, "✗", "x");
    icon!(project, "●", "*");
    icon!(search, "◎", "?");
    icon!(database, "◉", "@");
    icon!(session, "▪", "#");
    icon!(time, "○", "~");
    icon!(link, "◌", "~");
    icon!(calendar, "▫", "=");
    icon!(message, "•", "-");
    icon!(check, "✓", "+");
    icon!(arrow, "→", "->");
    icon!(bullet, "·", "-");
    icon!(match_prefix, "▸▸▸", ">>>");

    /// Context prefix (always spaces).
    pub fn context_prefix() -> &'static str {
        "   "
    }

    /// Get icon for a chunk kind.
    pub fn for_chunk(kind: &str, is_error: bool) -> &'static str {
        if is_error {
            return Self::error();
        }
        match kind {
            "message" => Self::message(),
            "tool_use" => Self::tool(),
            "tool_result" => Self::result(),
            _ => Self::bullet(),
        }
    }

    /// Get icon for a role.
    pub fn for_role(role: &str) -> &'static str {
        match role {
            "user" => Self::user(),
            "assistant" => Self::assistant(),
            _ => Self::message(),
        }
    }
}
