//! Unicode icons with ASCII fallbacks.

use super::ctx;

/// Icons for different content types.
pub struct Icons;

impl Icons {
    /// User message icon.
    pub fn user() -> &'static str {
        if ctx().unicode {
            "👤"
        } else {
            "[user]"
        }
    }

    /// Assistant message icon.
    pub fn assistant() -> &'static str {
        if ctx().unicode {
            "🤖"
        } else {
            "[asst]"
        }
    }

    /// Tool use icon.
    pub fn tool() -> &'static str {
        if ctx().unicode {
            "🔧"
        } else {
            "[tool]"
        }
    }

    /// Tool result (success) icon.
    pub fn result() -> &'static str {
        if ctx().unicode {
            "✅"
        } else {
            "[done]"
        }
    }

    /// Error icon.
    pub fn error() -> &'static str {
        if ctx().unicode {
            "❌"
        } else {
            "[err]"
        }
    }

    /// Project/folder icon.
    pub fn project() -> &'static str {
        if ctx().unicode {
            "📁"
        } else {
            "[proj]"
        }
    }

    /// Search icon.
    pub fn search() -> &'static str {
        if ctx().unicode {
            "🔍"
        } else {
            "[find]"
        }
    }

    /// Database icon.
    pub fn database() -> &'static str {
        if ctx().unicode {
            "📊"
        } else {
            "[db]"
        }
    }

    /// Session icon.
    pub fn session() -> &'static str {
        if ctx().unicode {
            "📋"
        } else {
            "[sess]"
        }
    }

    /// Time/clock icon.
    pub fn time() -> &'static str {
        if ctx().unicode {
            "⏱️"
        } else {
            "[time]"
        }
    }

    /// Link/related icon.
    pub fn link() -> &'static str {
        if ctx().unicode {
            "🔗"
        } else {
            "[link]"
        }
    }

    /// Calendar/recent icon.
    pub fn calendar() -> &'static str {
        if ctx().unicode {
            "📅"
        } else {
            "[date]"
        }
    }

    /// Lightning/quick icon.
    pub fn lightning() -> &'static str {
        if ctx().unicode {
            "⚡"
        } else {
            ">"
        }
    }

    /// Message/chat icon.
    pub fn message() -> &'static str {
        if ctx().unicode {
            "💬"
        } else {
            "[msg]"
        }
    }

    /// Check mark.
    pub fn check() -> &'static str {
        if ctx().unicode {
            "✓"
        } else {
            "[ok]"
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
            "•"
        } else {
            "*"
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
