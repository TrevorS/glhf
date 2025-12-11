//! Conversation JSONL file parsing.

use crate::document::{ChunkKind, Document};
use crate::error::Result;
use crate::ingest::extract_project_from_path;
use crate::utils::truncate_text;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parses a conversation JSONL file into [`Document`]s.
///
/// Each line in the file is expected to be a JSON object. Only lines with
/// `type` of "user" or "assistant" are processed; other types (like
/// "file-history-snapshot") are skipped. Malformed JSON lines are also skipped.
///
/// This function extracts:
/// - **Message** chunks: User prompts and assistant text responses
/// - **`ToolUse`** chunks: Tool invocations with name, id, and input
/// - **`ToolResult`** chunks: Tool outputs with content and error status
pub fn parse_jsonl_file(path: &Path) -> Result<Vec<Document>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut documents = Vec::new();

    let project = extract_project_from_path(path);

    for line in reader.lines() {
        let Ok(line) = line else { continue };

        if line.trim().is_empty() {
            continue;
        }

        let Ok(value): std::result::Result<Value, _> = serde_json::from_str(&line) else {
            continue;
        };

        // Only process user/assistant messages
        let msg_type = value.get("type").and_then(|v| v.as_str());
        if !matches!(msg_type, Some("user" | "assistant")) {
            continue;
        }

        // Extract common metadata
        let timestamp = value
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let session_id = value
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(String::from);

        let role = value
            .get("message")
            .and_then(|m| m.get("role"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract all chunks from this message
        let chunks = extract_chunks(
            &value,
            path,
            project.as_deref(),
            timestamp,
            session_id,
            role,
        );
        documents.extend(chunks);
    }

    Ok(documents)
}

/// Extracts all chunks (Message, `ToolUse`, `ToolResult`) from a message value.
fn extract_chunks(
    value: &Value,
    path: &Path,
    project: Option<&str>,
    timestamp: Option<DateTime<Utc>>,
    session_id: Option<String>,
    role: Option<String>,
) -> Vec<Document> {
    let mut documents = Vec::new();

    let Some(message) = value.get("message") else {
        return documents;
    };

    let Some(content) = message.get("content") else {
        return documents;
    };

    match content {
        // Simple string content - just a message
        Value::String(s) => {
            if !s.trim().is_empty() {
                let doc = Document::new(ChunkKind::Message, s.clone(), path.to_path_buf())
                    .with_project(project.map(String::from))
                    .with_timestamp(timestamp)
                    .with_session_id(session_id)
                    .with_role(role);
                documents.push(doc);
            }
        }

        // Array of content blocks
        Value::Array(blocks) => {
            let mut text_parts: Vec<String> = Vec::new();

            for block in blocks {
                let block_type = block.get("type").and_then(|v| v.as_str());

                match block_type {
                    Some("text") => {
                        // Accumulate text blocks
                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                            if !text.trim().is_empty() {
                                text_parts.push(text.to_string());
                            }
                        }
                    }

                    Some("tool_use") => {
                        // Create a ToolUse chunk
                        if let Some(doc) =
                            extract_tool_use(block, path, project, timestamp, session_id.clone())
                        {
                            documents.push(doc);
                        }
                    }

                    Some("tool_result") => {
                        // Create a ToolResult chunk
                        if let Some(doc) =
                            extract_tool_result(block, path, project, timestamp, session_id.clone())
                        {
                            documents.push(doc);
                        }
                    }

                    _ => {
                        // Unknown block type - try to extract any text
                        if let Some(text) = extract_text_from_block(block) {
                            if !text.trim().is_empty() {
                                text_parts.push(text);
                            }
                        }
                    }
                }
            }

            // Create a Message chunk from accumulated text
            if !text_parts.is_empty() {
                let combined_text = text_parts.join("\n");
                let doc = Document::new(ChunkKind::Message, combined_text, path.to_path_buf())
                    .with_project(project.map(String::from))
                    .with_timestamp(timestamp)
                    .with_session_id(session_id)
                    .with_role(role);
                documents.push(doc);
            }
        }

        _ => {}
    }

    documents
}

/// Extracts a `ToolUse` document from a `tool_use` block.
fn extract_tool_use(
    block: &Value,
    path: &Path,
    project: Option<&str>,
    timestamp: Option<DateTime<Utc>>,
    session_id: Option<String>,
) -> Option<Document> {
    let tool_name = block.get("name").and_then(|v| v.as_str())?;
    let tool_id = block.get("id").and_then(|v| v.as_str()).map(String::from);

    // Get input as JSON string
    let tool_input = block.get("input").map(std::string::ToString::to_string);

    // Extract searchable content from input
    let content = extract_tool_use_content(tool_name, block.get("input"));

    let doc = Document::new(ChunkKind::ToolUse, content, path.to_path_buf())
        .with_project(project.map(String::from))
        .with_timestamp(timestamp)
        .with_session_id(session_id)
        .with_tool_name(Some(tool_name.to_string()))
        .with_tool_id(tool_id)
        .with_tool_input(tool_input);

    Some(doc)
}

/// Extracts searchable content from `tool_use` input based on tool type.
fn extract_tool_use_content(tool_name: &str, input: Option<&Value>) -> String {
    let Some(input) = input else {
        return tool_name.to_string();
    };

    match tool_name {
        "Bash" => {
            // Extract command
            input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or(tool_name)
                .to_string()
        }
        "Read" | "Write" => {
            // Extract file_path
            input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or(tool_name)
                .to_string()
        }
        "Edit" => {
            // Extract file_path and old_string/new_string summary
            let file_path = input.get("file_path").and_then(|v| v.as_str());
            let old_str = input.get("old_string").and_then(|v| v.as_str());
            let new_str = input.get("new_string").and_then(|v| v.as_str());

            let mut parts = Vec::new();
            if let Some(fp) = file_path {
                parts.push(fp.to_string());
            }
            if let Some(old) = old_str {
                parts.push(truncate_text(old, 100));
            }
            if let Some(new) = new_str {
                parts.push(truncate_text(new, 100));
            }

            if parts.is_empty() {
                tool_name.to_string()
            } else {
                parts.join(" | ")
            }
        }
        "Grep" | "Glob" => {
            // Extract pattern and path
            let pattern = input.get("pattern").and_then(|v| v.as_str());
            let path = input.get("path").and_then(|v| v.as_str());

            match (pattern, path) {
                (Some(p), Some(pa)) => format!("{p} in {pa}"),
                (Some(p), None) => p.to_string(),
                (None, Some(pa)) => pa.to_string(),
                (None, None) => tool_name.to_string(),
            }
        }
        "Task" => {
            // Extract prompt
            input
                .get("prompt")
                .and_then(|v| v.as_str())
                .map_or_else(|| tool_name.to_string(), |s| truncate_text(s, 200))
        }
        "WebFetch" | "WebSearch" => {
            // Extract url or query
            input
                .get("url")
                .or_else(|| input.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or(tool_name)
                .to_string()
        }
        _ => {
            // Generic: try to get any string field or just use tool name
            if let Value::Object(obj) = input {
                obj.values()
                    .find_map(|v| v.as_str())
                    .map_or_else(|| tool_name.to_string(), |s| truncate_text(s, 200))
            } else {
                tool_name.to_string()
            }
        }
    }
}

/// Extracts a `ToolResult` document from a `tool_result` block.
fn extract_tool_result(
    block: &Value,
    path: &Path,
    project: Option<&str>,
    timestamp: Option<DateTime<Utc>>,
    session_id: Option<String>,
) -> Option<Document> {
    let tool_use_id = block
        .get("tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    let is_error = block.get("is_error").and_then(serde_json::Value::as_bool);

    // Extract content
    let content = extract_tool_result_content(block);
    if content.trim().is_empty() {
        return None;
    }

    let doc = Document::new(ChunkKind::ToolResult, content, path.to_path_buf())
        .with_project(project.map(String::from))
        .with_timestamp(timestamp)
        .with_session_id(session_id)
        .with_tool_id(tool_use_id)
        .with_is_error(is_error);

    Some(doc)
}

/// Extracts content from a `tool_result` block.
fn extract_tool_result_content(block: &Value) -> String {
    let Some(content) = block.get("content") else {
        return String::new();
    };

    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => extract_text_strings_from_array(arr, &["text", "content"]),
        _ => String::new(),
    }
}

/// Extracts text from a generic content block.
fn extract_text_from_block(block: &Value) -> Option<String> {
    // Try "text" field first (common for text blocks)
    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    // Try "content" field (for tool results)
    if let Some(content) = block.get("content") {
        match content {
            Value::String(s) => return Some(s.clone()),
            Value::Array(arr) => {
                let text = extract_text_strings_from_array(arr, &["text"]);
                if !text.is_empty() {
                    return Some(text);
                }
            }
            _ => {}
        }
    }

    None
}

/// Extracts text strings from an array of JSON values by checking specified fields.
///
/// Iterates through the array and for each item, tries the field names in order
/// until it finds a string value. All found strings are joined with newlines.
fn extract_text_strings_from_array(arr: &[Value], field_names: &[&str]) -> String {
    let texts: Vec<String> = arr
        .iter()
        .filter_map(|item| {
            for field in field_names {
                if let Some(text) = item.get(*field).and_then(|v| v.as_str()) {
                    return Some(text.to_string());
                }
            }
            None
        })
        .collect();
    texts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tool_use_content_bash() {
        let input: Value = serde_json::json!({
            "command": "git status"
        });
        let content = extract_tool_use_content("Bash", Some(&input));
        assert_eq!(content, "git status");
    }

    #[test]
    fn test_extract_tool_use_content_read() {
        let input: Value = serde_json::json!({
            "file_path": "/home/user/test.rs"
        });
        let content = extract_tool_use_content("Read", Some(&input));
        assert_eq!(content, "/home/user/test.rs");
    }

    #[test]
    fn test_extract_tool_use_content_grep() {
        let input: Value = serde_json::json!({
            "pattern": "TODO",
            "path": "/src"
        });
        let content = extract_tool_use_content("Grep", Some(&input));
        assert_eq!(content, "TODO in /src");
    }

    #[test]
    fn test_extract_text_strings_from_array() {
        let arr: Vec<Value> = vec![
            serde_json::json!({"text": "hello"}),
            serde_json::json!({"text": "world"}),
        ];
        let result = extract_text_strings_from_array(&arr, &["text"]);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn test_extract_text_strings_from_array_fallback() {
        let arr: Vec<Value> = vec![
            serde_json::json!({"content": "fallback"}),
            serde_json::json!({"text": "primary"}),
        ];
        let result = extract_text_strings_from_array(&arr, &["text", "content"]);
        assert_eq!(result, "fallback\nprimary");
    }
}
