use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use crate::config::kuiper_data_dir;

#[derive(Serialize)]
struct TelemetryEvent {
    timestamp: String,
    event_type: String,
    details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fast_path: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_len: Option<usize>,
}

/// Log an event to the telemetry file (JSONL format)
pub fn log_event(event_type: &str, details: &str) {
    let event = TelemetryEvent {
        timestamp: Utc::now().to_rfc3339(),
        event_type: event_type.to_string(),
        details: truncate(details, 200),
        fast_path: None,
        model: None,
        duration_ms: None,
        prompt_len: None,
    };

    // Print to stderr for debugging
    eprintln!("[{}] {}: {}", event.timestamp, event_type, truncate(details, 50));

    // Log to file
    if let Err(e) = log_jsonl(&event) {
        eprintln!("Warning: Failed to log event: {}", e);
    }
}

/// Log usage statistics with full context
pub fn log_usage(prompt: &str, fast_path: bool, model: &str, duration_ms: u64) {
    let event = TelemetryEvent {
        timestamp: Utc::now().to_rfc3339(),
        event_type: "usage".to_string(),
        details: truncate(prompt, 100),
        fast_path: Some(fast_path),
        model: Some(model.to_string()),
        duration_ms: Some(duration_ms),
        prompt_len: Some(prompt.len()),
    };

    eprintln!(
        "[{}] usage: fast_path={}, model={}, duration_ms={}",
        event.timestamp, fast_path, model, duration_ms
    );

    if let Err(e) = log_jsonl(&event) {
        eprintln!("Warning: Failed to log usage: {}", e);
    }
}

/// Get the last prompt that was executed
pub fn get_last_prompt() -> Result<String> {
    let data_dir = kuiper_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    let history_file = data_dir.join("history.txt");

    if !history_file.exists() {
        anyhow::bail!("No previous prompts found. Run 'kuiper do <prompt>' first.");
    }

    let content = fs::read_to_string(&history_file)?;
    content
        .lines()
        .last()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("History file is empty"))
}

/// Save a prompt to history
pub fn save_prompt(prompt: &str) -> Result<()> {
    let data_dir = kuiper_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    fs::create_dir_all(&data_dir)?;

    let history_file = data_dir.join("history.txt");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_file)?;

    writeln!(file, "{}", prompt)?;
    Ok(())
}

/// Get recent escalation rate for adaptive thresholds
pub fn get_escalation_rate(window_hours: u32) -> Result<f32> {
    let data_dir = kuiper_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    let log_file = data_dir.join("telemetry.jsonl");

    if !log_file.exists() {
        return Ok(0.1); // Default rate
    }

    let content = fs::read_to_string(&log_file)?;
    let cutoff = Utc::now() - chrono::Duration::hours(window_hours as i64);

    let mut total_requests = 0u32;
    let mut escalations = 0u32;

    for line in content.lines() {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ts) = event.get("timestamp").and_then(|t| t.as_str()) {
                if let Ok(event_time) = chrono::DateTime::parse_from_rfc3339(ts) {
                    if event_time.with_timezone(&Utc) > cutoff {
                        if let Some(event_type) = event.get("event_type").and_then(|e| e.as_str()) {
                            if event_type == "usage" || event_type.contains("path") {
                                total_requests += 1;
                            }
                            if event_type.contains("escalation") {
                                escalations += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if total_requests == 0 {
        Ok(0.1)
    } else {
        Ok(escalations as f32 / total_requests as f32)
    }
}

// Internal helpers

fn log_jsonl(event: &TelemetryEvent) -> Result<()> {
    let data_dir = kuiper_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    fs::create_dir_all(&data_dir)?;

    let log_file = data_dir.join("telemetry.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    writeln!(file, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
    }
}
