//! Search result filtering and project/session detection.

use crate::commands::SearchOptions;
use crate::config;
use crate::db::SearchResult;
use crate::format::project_name;
use chrono::DateTime;

/// Resolves the project filter, expanding `.` to the current working directory.
pub fn resolve_project_filter(project: Option<&str>) -> Option<String> {
    project.map(|p| {
        if p == "." {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| cwd.to_str().map(String::from))
                .unwrap_or_else(|| ".".to_string())
        } else {
            p.to_string()
        }
    })
}

/// Returns the current project name based on the working directory.
pub fn current_project_name() -> Option<String> {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
}

/// Detects the current Claude Code session ID.
///
/// Uses two-tier detection:
/// 1. Primary: Check `CLAUDE_SESSION_ID` env var
/// 2. Fallback: Find most recently modified non-agent JSONL in project directory
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn detect_current_session() -> Option<String> {
    if let Ok(session_id) = std::env::var("CLAUDE_SESSION_ID") {
        if !session_id.is_empty() {
            return Some(session_id);
        }
    }

    let project_dir = current_project_dir()?;
    std::fs::read_dir(project_dir)
        .ok()?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".jsonl") && !name.starts_with("agent-")
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((e, modified))
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(e, _)| {
            e.file_name()
                .to_string_lossy()
                .trim_end_matches(".jsonl")
                .to_string()
        })
}

/// Returns the Claude projects directory for the current working directory.
fn current_project_dir() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let encoded = encode_project_path(&cwd);
    let projects_dir = config::projects_dir().ok()?;
    Some(projects_dir.join(encoded))
}

/// Encodes a path the way Claude Code does: `/` -> `-`, `/.` -> `--`
fn encode_project_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace("/.", "--").replace('/', "-")
}

/// Filters a search result based on options.
pub fn filter_result(
    result: &SearchResult,
    options: &SearchOptions,
    resolved_project: Option<&str>,
    current_project: Option<&str>,
    current_session: Option<&str>,
) -> bool {
    if options.messages_only && result.chunk_kind != "message" {
        return false;
    }

    if options.tools_only && result.chunk_kind == "message" {
        return false;
    }

    if let Some(ref tool) = options.tool {
        match &result.tool_name {
            Some(name) if name.eq_ignore_ascii_case(tool) => {}
            _ => return false,
        }
    }

    if let Some(project_filter) = resolved_project {
        let filter_lower = project_filter.to_lowercase();
        match &result.project {
            Some(project) if project.to_lowercase().contains(&filter_lower) => {}
            _ => return false,
        }
    }

    if !options.exclude_projects.is_empty() {
        let result_project = project_name(result.project.as_deref());
        for excluded in &options.exclude_projects {
            if result_project
                .to_lowercase()
                .contains(&excluded.to_lowercase())
            {
                return false;
            }
        }
    }

    if options.exclude_this_project {
        if let Some(cur_proj) = current_project {
            let result_project = project_name(result.project.as_deref());
            if result_project.eq_ignore_ascii_case(cur_proj) {
                return false;
            }
        }
    }

    if options.this_session {
        if let Some(cur_sess) = current_session {
            match &result.session_id {
                Some(sess) if sess.starts_with(cur_sess) || cur_sess.starts_with(sess) => {}
                _ => return false,
            }
        }
    }

    if options.exclude_this_session {
        if let Some(cur_sess) = current_session {
            if let Some(ref sess) = result.session_id {
                if sess.starts_with(cur_sess) || cur_sess.starts_with(sess) {
                    return false;
                }
            }
        }
    }

    if options.errors && result.is_error != Some(true) {
        return false;
    }

    if let Some(since) = options.since {
        let ts_ok = result
            .timestamp
            .as_ref()
            .and_then(|ts_str| DateTime::parse_from_rfc3339(ts_str).ok())
            .is_some_and(|ts| ts >= since);
        if !ts_ok {
            return false;
        }
    }

    true
}
