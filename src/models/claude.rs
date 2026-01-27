use anyhow::{Context, Result};
use crate::config::Config;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Option<Vec<ContentBlock>>,
    error: Option<ClaudeError>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct ClaudeError {
    message: String,
}

/// Run a prompt against Claude API using config default model
pub async fn run(prompt: &str, config: &Config) -> Result<String> {
    let model = &config.models.claude_settings.model;
    run_with_model(prompt, model, config).await
}

/// Run a prompt against Claude API with a specific model
pub async fn run_with_model(prompt: &str, model_id: &str, config: &Config) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY not set. Export it or use --fast to skip API calls.")?;

    let request = ClaudeRequest {
        model: model_id.to_string(),
        max_tokens: config.models.claude_settings.max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send request to Claude")?;

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .context("Failed to parse Claude response")?;

    if let Some(error) = claude_response.error {
        anyhow::bail!("Claude API error: {}", error.message);
    }

    let text = claude_response
        .content
        .and_then(|c| c.into_iter().next())
        .map(|b| b.text)
        .unwrap_or_else(|| "No response generated".to_string());

    Ok(text)
}

/// Simulate streaming by returning tokens (for demo/offline mode)
pub fn stream(prompt: &str) -> Vec<String> {
    let response = format!(
        "[Claude] Analyzing: {} | This is a simulated streaming response from Claude model with nuanced reasoning.",
        truncate(prompt, 50)
    );

    response
        .split_whitespace()
        .map(|word| format!("{} ", word))
        .collect()
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
