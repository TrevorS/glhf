//! Display formatting helpers for CLI output.

use crate::db::SearchResult;
use crate::document::DisplayLabel;
use crate::utils::truncate_text;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Maximum characters for result snippets.
pub const RESULT_SNIPPET_LEN: usize = 200;

/// Formats a timestamp as relative time (e.g., "2h ago", "3d ago").
pub fn format_relative_time(timestamp: Option<&str>) -> String {
    let Some(ts_str) = timestamp else {
        return "unknown".to_string();
    };

    let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) else {
        return "unknown".to_string();
    };

    let now = Utc::now();
    let ts_utc = ts.with_timezone(&Utc);
    let duration = now.signed_duration_since(ts_utc);

    let seconds = duration.num_seconds();
    if seconds < 0 {
        return "future".to_string();
    }

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else if weeks < 8 {
        format!("{weeks}w ago")
    } else {
        ts_utc.format("%b %d").to_string()
    }
}

/// Formats seconds-since-epoch as a human-readable "ago" string.
#[allow(clippy::cast_possible_wrap)]
pub fn format_seconds_ago(epoch_secs: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let delta = now - epoch_secs;
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86400)
    }
}

/// Formats a duration in a human-readable way.
fn format_duration(dur: chrono::Duration) -> String {
    let total_secs = dur.num_seconds();
    if total_secs < 60 {
        format!("{total_secs}s")
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{hours}h")
        }
    }
}

/// Formats a number with comma separators (e.g., 12847 -> "12,847").
pub fn format_number(n: i64) -> String {
    let s = n.to_string();
    let (prefix, digits) = if let Some(stripped) = s.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", s.as_str())
    };
    let mut result = String::with_capacity(s.len() + digits.len() / 3);
    result.push_str(prefix);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Formats a timestamp as a date string (e.g., "2025-01-15").
pub fn format_date(timestamp: Option<&str>) -> String {
    let Some(ts_str) = timestamp else {
        return "unknown".to_string();
    };

    let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) else {
        return "unknown".to_string();
    };

    ts.format("%Y-%m-%d").to_string()
}

/// Format bytes as human-readable size.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Extracts a display name from an encoded project path.
///
/// The encoded path looks like `-Users-trevor-Projects-foo` or `-Users-trevor--claude`.
/// We extract a reasonable display name by:
/// 1. Looking for `-Projects-` and taking everything after it
/// 2. Looking for `--` (hidden dir marker) and taking everything after it
/// 3. Falling back to the last hyphen-separated segment
pub fn project_name(project: Option<&str>) -> &str {
    let Some(p) = project else {
        return "unknown";
    };

    if p.contains('/') {
        return p.rsplit('/').next().unwrap_or(p);
    }

    if let Some(idx) = p.rfind("-Projects-") {
        let after = &p[idx + "-Projects-".len()..];
        if !after.is_empty() {
            return after;
        }
    }

    if let Some(idx) = p.rfind("--") {
        let after = &p[idx + 2..];
        if !after.is_empty() {
            return after;
        }
    }

    if p.starts_with('-') {
        if let Some(idx) = p.rfind('-') {
            let after = &p[idx + 1..];
            if !after.is_empty() {
                return after;
            }
        }
    }

    p
}

/// Prints the header for a search result.
pub fn print_result_header(
    num: usize,
    result: &SearchResult,
    show_session_id: bool,
    show_scores: bool,
) {
    let project_display = project_name(result.project.as_deref());
    let label = result.display_label();
    let time_display = format_relative_time(result.timestamp.as_deref());
    let score_display = if show_scores {
        format!(" | Score: {:.2}", result.score)
    } else {
        String::new()
    };

    if show_session_id {
        let session_display = result
            .session_id
            .as_ref()
            .map_or("unknown", |s| &s[..s.len().min(8)]);
        println!(
            "[{}] {} | {} | {} | {}{} | sess:{}",
            num,
            result.chunk_kind,
            project_display,
            label,
            time_display,
            score_display,
            session_display
        );
    } else {
        println!(
            "[{}] {} | {} | {} | {}{}",
            num, result.chunk_kind, project_display, label, time_display, score_display
        );
    }
}

/// Prints a compact single-line search result.
pub fn print_result_compact(num: usize, result: &SearchResult, show_scores: bool) {
    let project_display = project_name(result.project.as_deref());
    let label = result.display_label();
    let time_display = format_relative_time(result.timestamp.as_deref());
    let session_display = result
        .session_id
        .as_ref()
        .map_or("--------", |s| &s[..s.len().min(8)]);
    let snippet = truncate_text(&result.content, 60);

    if show_scores {
        println!(
            "[{num}] {:.2} | {project_display} | {label} | {time_display} | {session_display} | \"{snippet}\"",
            result.score
        );
    } else {
        println!(
            "[{num}] {project_display} | {label} | {time_display} | {session_display} | \"{snippet}\""
        );
    }
}

/// Prints a single message in session view format.
pub fn print_session_message(msg: &SearchResult) {
    let time = format_relative_time(msg.timestamp.as_deref());
    let label = msg.display_label();

    let header = format!("[{}] {} | {}", label, msg.chunk_kind, time);
    println!("\n{header}");
    println!("{}", "─".repeat(40));

    let content = if msg.content.len() > 2000 {
        let mut end = 2000;
        while end > 0 && !msg.content.is_char_boundary(end) {
            end -= 1;
        }
        format!(
            "{}...\n[truncated, {} total chars]",
            &msg.content[..end],
            msg.content.len()
        )
    } else {
        msg.content.clone()
    };
    println!("{content}");
}

/// Prints a summary of a session without full content.
pub fn print_session_summary(session_id: &str, project: Option<&str>, messages: &[SearchResult]) {
    let project_display = project_name(project);

    let mut kind_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages {
        *kind_counts.entry(msg.chunk_kind.as_str()).or_insert(0) += 1;
    }

    let mut role_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages.iter().filter(|m| m.chunk_kind == "message") {
        if let Some(role) = &msg.role {
            *role_counts.entry(role.as_str()).or_insert(0) += 1;
        }
    }

    let mut tool_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages.iter().filter(|m| m.chunk_kind == "tool_use") {
        if let Some(tool) = &msg.tool_name {
            *tool_counts.entry(tool.as_str()).or_insert(0) += 1;
        }
    }

    let first_ts = messages.first().and_then(|m| m.timestamp.as_ref());
    let last_ts = messages.last().and_then(|m| m.timestamp.as_ref());
    let duration = match (first_ts, last_ts) {
        (Some(first), Some(last)) => {
            if let (Ok(f), Ok(l)) = (
                chrono::DateTime::parse_from_rfc3339(first),
                chrono::DateTime::parse_from_rfc3339(last),
            ) {
                let dur = l.signed_duration_since(f);
                format_duration(dur)
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    };

    let started = format_relative_time(first_ts.map(std::string::String::as_str));

    println!("Session: {session_id}");
    println!("Project: {project_display}");
    println!("Duration: {duration} (started {started})");
    println!("Messages: {} total", messages.len());

    if !role_counts.is_empty() {
        let mut roles: Vec<_> = role_counts.into_iter().collect();
        roles.sort_by_key(|r| std::cmp::Reverse(r.1));
        for (role, count) in roles {
            println!("  - {role}: {count}");
        }
    }

    if let Some(&tool_use_count) = kind_counts.get("tool_use") {
        println!("  - tool calls: {tool_use_count}");
    }
    if let Some(&tool_result_count) = kind_counts.get("tool_result") {
        println!("  - tool results: {tool_result_count}");
    }

    if !tool_counts.is_empty() {
        let mut tools: Vec<_> = tool_counts.into_iter().collect();
        tools.sort_by_key(|t| std::cmp::Reverse(t.1));
        let top_tools: Vec<_> = tools
            .iter()
            .take(5)
            .map(|(t, c)| format!("{t} ({c})"))
            .collect();
        println!("Tools used: {}", top_tools.join(", "));
    }
}
