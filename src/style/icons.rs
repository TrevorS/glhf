//! Unicode icons with ASCII fallbacks.

use super::ctx;

/// Icons for different content types.
/// Uses minimal, sleek ideograms rather than emoji.
pub struct Icons;

impl Icons {
    /// User message icon.
    pub fn user() -> &'static str {
        if ctx().unicode {
            "◆"
        } else {
            "[u]"
        }
    }

    /// Assistant message icon.
    pub fn assistant() -> &'static str {
        if ctx().unicode {
            "◇"
        } else {
            "[a]"
        }
    }

    /// Tool use icon.
    pub fn tool() -> &'static str {
        if ctx().unicode {
            "▸"
        } else {
            ">"
        }
    }

    /// Tool result (success) icon.
    pub fn result() -> &'static str {
        if ctx().unicode {
            "✓"
        } else {
            "+"
        }
    }

    /// Error icon.
    pub fn error() -> &'static str {
        if ctx().unicode {
            "✗"
        } else {
            "x"
        }
    }

    /// Project/folder icon.
    pub fn project() -> &'static str {
        if ctx().unicode {
            "●"
        } else {
            "*"
        }
    }

    /// Search icon.
    pub fn search() -> &'static str {
        if ctx().unicode {
            "◎"
        } else {
            "?"
        }
    }

    /// Database icon.
    pub fn database() -> &'static str {
        if ctx().unicode {
            "◉"
        } else {
            "@"
        }
    }

    /// Session icon.
    pub fn session() -> &'static str {
        if ctx().unicode {
            "▪"
        } else {
            "#"
        }
    }

    /// Time/clock icon.
    pub fn time() -> &'static str {
        if ctx().unicode {
            "○"
        } else {
            "~"
        }
    }

    /// Link/related icon.
    pub fn link() -> &'static str {
        if ctx().unicode {
            "◌"
        } else {
            "~"
        }
    }

    /// Calendar/recent icon.
    pub fn calendar() -> &'static str {
        if ctx().unicode {
            "▫"
        } else {
            "="
        }
    }

    /// Lightning/quick icon.
    pub fn lightning() -> &'static str {
        if ctx().unicode {
            "›"
        } else {
            ">"
        }
    }

    /// Message/chat icon.
    pub fn message() -> &'static str {
        if ctx().unicode {
            "•"
        } else {
            "-"
        }
    }

    /// Check mark.
    pub fn check() -> &'static str {
        if ctx().unicode {
            "✓"
        } else {
            "+"
        }
    }

    /// Arrow right.
    pub fn arrow() -> &'static str {
        if ctx().unicode {
            "→"
        } else {
            "->"
        }
    }

    /// Bullet point.
    pub fn bullet() -> &'static str {
        if ctx().unicode {
            "·"
        } else {
            "-"
        }
    }

    /// Match highlight prefix.
    pub fn match_prefix() -> &'static str {
        if ctx().unicode {
            "▸▸▸"
        } else {
            ">>>"
        }
    }

    /// Context prefix.
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
