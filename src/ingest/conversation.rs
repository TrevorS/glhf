use crate::ingest::extract_project_from_path;
use crate::models::document::{DocType, Document};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parses a conversation JSONL file into Documents
pub fn parse_jsonl_file(path: &Path) -> Result<Vec<Document>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut documents = Vec::new();

    let project = extract_project_from_path(path);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Only process user/assistant messages
        let msg_type = value.get("type").and_then(|v| v.as_str());
        if !matches!(msg_type, Some("user") | Some("assistant")) {
            continue;
        }

        // Extract content from message
        let content = extract_message_content(&value);
        if content.trim().is_empty() {
            continue;
        }

        // Extract metadata
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

        let doc = Document::new(DocType::Conversation, content, path.to_path_buf())
            .with_project(project.clone())
            .with_timestamp(timestamp)
            .with_session_id(session_id)
            .with_role(role);

        documents.push(doc);
    }

    Ok(documents)
}

/// Extracts text content from a message, handling both string and array formats
fn extract_message_content(value: &Value) -> String {
    let message = match value.get("message") {
        Some(m) => m,
        None => return String::new(),
    };

    let content = match message.get("content") {
        Some(c) => c,
        None => return String::new(),
    };

    match content {
        // Simple string content
        Value::String(s) => s.clone(),

        // Array of content blocks (tool results, etc.)
        Value::Array(blocks) => {
            let texts: Vec<String> = blocks.iter().filter_map(extract_text_from_block).collect();
            texts.join("\n")
        }

        _ => String::new(),
    }
}

/// Extracts text from a content block
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
                let texts: Vec<String> = arr
                    .iter()
                    .filter_map(|item| item.get("text").and_then(|v| v.as_str()).map(String::from))
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
            _ => {}
        }
    }

    None
}
