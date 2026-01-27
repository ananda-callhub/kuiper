use anyhow::{Context, Result};
use crate::config::Config;
use serde::{Deserialize, Serialize};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiError {
    message: String,
}

/// Run a prompt against Gemini API using config default model
pub async fn run(prompt: &str, config: &Config) -> Result<String> {
    let model = &config.models.gemini_settings.model;
    run_with_model(prompt, model, config).await
}

/// Run a prompt against Gemini API with a specific model
pub async fn run_with_model(prompt: &str, model_id: &str, config: &Config) -> Result<String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .context("GEMINI_API_KEY not set. Export it or use --fast to skip API calls.")?;

    let url = format!(
        "{}/{}:generateContent?key={}",
        GEMINI_API_URL, model_id, api_key
    );

    let request = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: prompt.to_string(),
            }],
        }],
        generation_config: GenerationConfig {
            max_output_tokens: config.models.gemini_settings.max_tokens,
        },
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Failed to send request to Gemini")?;

    let gemini_response: GeminiResponse = response
        .json()
        .await
        .context("Failed to parse Gemini response")?;

    if let Some(error) = gemini_response.error {
        anyhow::bail!("Gemini API error: {}", error.message);
    }

    let text = gemini_response
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .unwrap_or_else(|| "No response generated".to_string());

    Ok(text)
}

/// Simulate streaming by returning tokens (for demo/offline mode)
pub fn stream(prompt: &str) -> Vec<String> {
    let response = format!(
        "[Gemini] Processing: {} | This is a simulated streaming response from Gemini model.",
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
