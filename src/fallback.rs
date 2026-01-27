use anyhow::Result;
use std::time::Instant;
use crate::config::Config;
use crate::models::{self, ModelType};
use crate::telemetry;

/// Errors that should trigger fallback to another model
const FALLBACK_ERROR_PATTERNS: &[&str] = &[
    "rate limit",
    "quota exceeded",
    "too many requests",
    "429",
    "capacity",
    "overloaded",
    "temporarily unavailable",
    "service unavailable",
    "503",
    "timeout",
    "connection refused",
    "API key",
    "authentication",
    "unauthorized",
    "401",
    "403",
];

/// Result of a model execution attempt
#[derive(Debug)]
pub struct ModelAttempt {
    pub model: ModelType,
    pub success: bool,
    pub response: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub should_fallback: bool,
}

/// Fallback chain configuration
#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub models: Vec<ModelType>,
    pub max_attempts_per_model: u32,
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self {
            models: vec![ModelType::Gemini, ModelType::Claude, ModelType::Codex],
            max_attempts_per_model: 1,
        }
    }
}

impl FallbackChain {
    /// Create a chain starting with a specific model
    pub fn starting_with(primary: ModelType) -> Self {
        let models = match primary {
            ModelType::Gemini => vec![ModelType::Gemini, ModelType::Claude, ModelType::Codex],
            ModelType::Claude => vec![ModelType::Claude, ModelType::Gemini, ModelType::Codex],
            ModelType::Codex => vec![ModelType::Codex, ModelType::Claude, ModelType::Gemini],
        };
        Self {
            models,
            max_attempts_per_model: 1,
        }
    }

    /// Create a chain for code-related tasks (Codex first)
    pub fn for_code() -> Self {
        Self {
            models: vec![ModelType::Codex, ModelType::Claude, ModelType::Gemini],
            max_attempts_per_model: 1,
        }
    }

    /// Create a chain for reasoning tasks (Claude first)
    pub fn for_reasoning() -> Self {
        Self {
            models: vec![ModelType::Claude, ModelType::Gemini, ModelType::Codex],
            max_attempts_per_model: 1,
        }
    }

    /// Create a chain for general/fast tasks (Gemini first)
    pub fn for_general() -> Self {
        Self {
            models: vec![ModelType::Gemini, ModelType::Claude, ModelType::Codex],
            max_attempts_per_model: 1,
        }
    }

    /// Filter to only available models (with API keys)
    pub fn available_only(&self) -> Self {
        Self {
            models: self
                .models
                .iter()
                .filter(|m| models::is_available(m))
                .cloned()
                .collect(),
            max_attempts_per_model: self.max_attempts_per_model,
        }
    }
}

/// Execute with fallback - tries each model in the chain until one succeeds
pub async fn execute_with_fallback(
    prompt: &str,
    chain: &FallbackChain,
    config: &Config,
) -> Result<(String, ModelType, Vec<ModelAttempt>)> {
    let available_chain = chain.available_only();

    if available_chain.models.is_empty() {
        anyhow::bail!(
            "No models available. Please set at least one API key:\n\
             - GEMINI_API_KEY\n\
             - ANTHROPIC_API_KEY\n\
             - OPENAI_API_KEY"
        );
    }

    let mut attempts: Vec<ModelAttempt> = Vec::new();

    for model in &available_chain.models {
        let model_name = models::model_name(model);
        let start = Instant::now();

        eprintln!("[Fallback] Trying {}...", model_name);

        match models::run_model(model.clone(), prompt, config).await {
            Ok(response) => {
                let duration = start.elapsed().as_millis() as u64;

                attempts.push(ModelAttempt {
                    model: model.clone(),
                    success: true,
                    response: Some(response.clone()),
                    error: None,
                    duration_ms: duration,
                    should_fallback: false,
                });

                telemetry::log_event(
                    "fallback_success",
                    &format!("model={}, attempts={}", model_name, attempts.len()),
                );

                return Ok((response, model.clone(), attempts));
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                let error_msg = e.to_string();
                let should_fallback = is_fallback_error(&error_msg);

                attempts.push(ModelAttempt {
                    model: model.clone(),
                    success: false,
                    response: None,
                    error: Some(error_msg.clone()),
                    duration_ms: duration,
                    should_fallback,
                });

                if should_fallback {
                    eprintln!(
                        "[Fallback] {} failed ({}), trying next model...",
                        model_name,
                        truncate_error(&error_msg)
                    );
                    telemetry::log_event(
                        "fallback_triggered",
                        &format!("model={}, error={}", model_name, truncate_error(&error_msg)),
                    );
                } else {
                    // Non-recoverable error, don't fallback
                    eprintln!("[Fallback] {} failed with non-recoverable error", model_name);
                    anyhow::bail!("Model {} failed: {}", model_name, error_msg);
                }
            }
        }
    }

    // All models failed
    let tried_models: Vec<&str> = attempts.iter().map(|a| models::model_name(&a.model)).collect();
    anyhow::bail!(
        "All models failed. Tried: {}. Last error: {}",
        tried_models.join(", "),
        attempts
            .last()
            .and_then(|a| a.error.as_ref())
            .unwrap_or(&"Unknown error".to_string())
    )
}

/// Check if an error should trigger fallback to another model
pub fn is_fallback_error(error: &str) -> bool {
    let error_lower = error.to_lowercase();
    FALLBACK_ERROR_PATTERNS
        .iter()
        .any(|pattern| error_lower.contains(pattern))
}

/// Get the next model in the fallback chain
pub fn next_model(current: &ModelType, chain: &FallbackChain) -> Option<ModelType> {
    let available = chain.available_only();
    let pos = available.models.iter().position(|m| m == current)?;

    available.models.get(pos + 1).cloned()
}

/// Select the best fallback chain based on prompt characteristics
pub fn select_chain_for_prompt(prompt: &str) -> FallbackChain {
    let prompt_lower = prompt.to_lowercase();

    // Code-related prompts
    if prompt_lower.contains("code")
        || prompt_lower.contains("function")
        || prompt_lower.contains("implement")
        || prompt_lower.contains("debug")
        || prompt.contains("{")
        || prompt.contains("}")
    {
        return FallbackChain::for_code();
    }

    // Reasoning/analysis prompts
    if prompt_lower.contains("analyze")
        || prompt_lower.contains("explain")
        || prompt_lower.contains("compare")
        || prompt_lower.contains("review")
        || prompt_lower.contains("why")
    {
        return FallbackChain::for_reasoning();
    }

    // Default to general (Gemini first for speed)
    FallbackChain::for_general()
}

fn truncate_error(error: &str) -> String {
    if error.len() <= 50 {
        error.to_string()
    } else {
        format!("{}...", &error[..47])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fallback_error() {
        assert!(is_fallback_error("Rate limit exceeded"));
        assert!(is_fallback_error("Error 429: Too many requests"));
        assert!(is_fallback_error("Service temporarily unavailable"));
        assert!(is_fallback_error("quota exceeded for today"));
        assert!(!is_fallback_error("Invalid JSON in response"));
        assert!(!is_fallback_error("Unknown error occurred"));
    }

    #[test]
    fn test_fallback_chain_starting_with() {
        let chain = FallbackChain::starting_with(ModelType::Claude);
        assert_eq!(chain.models[0], ModelType::Claude);
    }

    #[test]
    fn test_select_chain_for_prompt() {
        let code_chain = select_chain_for_prompt("write a function to sort");
        assert_eq!(code_chain.models[0], ModelType::Codex);

        let reasoning_chain = select_chain_for_prompt("explain how this works");
        assert_eq!(reasoning_chain.models[0], ModelType::Claude);

        let general_chain = select_chain_for_prompt("hello world");
        assert_eq!(general_chain.models[0], ModelType::Gemini);
    }
}
