use anyhow::{Context, Result};
use crate::config::Config;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct OpenAIRequest {
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
struct OpenAIResponse {
    choices: Option<Vec<Choice>>,
    error: Option<OpenAIError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIError {
    message: String,
}

/// Run a prompt against OpenAI/Codex API using config default model
pub async fn run(prompt: &str, config: &Config) -> Result<String> {
    let model = &config.models.codex_settings.model;
    run_with_model(prompt, model, config).await
}

/// Run a prompt against OpenAI/Codex API with a specific model
pub async fn run_with_model(prompt: &str, model_id: &str, config: &Config) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY not set. Export it or use --fast to skip API calls.")?;

    let request = OpenAIRequest {
        model: model_id.to_string(),
        max_tokens: config.models.codex_settings.max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send request to OpenAI")?;

    let openai_response: OpenAIResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI response")?;

    if let Some(error) = openai_response.error {
        anyhow::bail!("OpenAI API error: {}", error.message);
    }

    let text = openai_response
        .choices
        .and_then(|c| c.into_iter().next())
        .map(|c| c.message.content)
        .unwrap_or_else(|| "No response generated".to_string());

    Ok(text)
}

/// Simulate streaming by returning tokens (for demo/offline mode)
pub fn stream(prompt: &str) -> Vec<String> {
    let response = format!(
        "[Codex] Generating code for: {} | fn main() {{ println!(\"Hello from Codex!\"); }}",
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
