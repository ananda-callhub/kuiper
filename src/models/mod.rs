pub mod claude;
pub mod codex;
pub mod gemini;
pub mod registry;

use anyhow::Result;
use crate::config::Config;
pub use registry::{ModelInfo, ModelRegistry, ModelTier, format_model_list};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Gemini,
    Claude,
    Codex,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Gemini => write!(f, "gemini"),
            ModelType::Claude => write!(f, "claude"),
            ModelType::Codex => write!(f, "codex"),
        }
    }
}

impl ModelType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gemini" | "google" => Some(ModelType::Gemini),
            "claude" | "anthropic" => Some(ModelType::Claude),
            "codex" | "openai" | "gpt" => Some(ModelType::Codex),
            _ => None,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            ModelType::Gemini => "gemini",
            ModelType::Claude => "claude",
            ModelType::Codex => "codex",
        }
    }
}

/// Run a prompt against the specified model with a specific model ID
pub async fn run_model_with_id(
    provider: ModelType,
    model_id: &str,
    prompt: &str,
    config: &Config,
) -> Result<String> {
    match provider {
        ModelType::Gemini => gemini::run_with_model(prompt, model_id, config).await,
        ModelType::Claude => claude::run_with_model(prompt, model_id, config).await,
        ModelType::Codex => codex::run_with_model(prompt, model_id, config).await,
    }
}

/// Run a prompt against the specified model using config defaults
pub async fn run_model(model: ModelType, prompt: &str, config: &Config) -> Result<String> {
    match model {
        ModelType::Gemini => gemini::run(prompt, config).await,
        ModelType::Claude => claude::run(prompt, config).await,
        ModelType::Codex => codex::run(prompt, config).await,
    }
}

/// Run using fast-path optimized model
pub async fn run_fast(model: ModelType, prompt: &str, config: &Config) -> Result<String> {
    let registry = ModelRegistry::new();
    let provider = model.provider_name();

    let model_info = registry.recommend(provider, true);
    let model_id = model_info
        .map(|m| m.id.as_str())
        .unwrap_or_else(|| default_model_id(&model, true));

    run_model_with_id(model, model_id, prompt, config).await
}

/// Run using complex/research model
pub async fn run_complex(model: ModelType, prompt: &str, config: &Config) -> Result<String> {
    let registry = ModelRegistry::new();
    let provider = model.provider_name();

    let model_info = registry.recommend(provider, false);
    let model_id = model_info
        .map(|m| m.id.as_str())
        .unwrap_or_else(|| default_model_id(&model, false));

    run_model_with_id(model, model_id, prompt, config).await
}

/// Get default model ID for a provider and path type
fn default_model_id(model: &ModelType, is_fast: bool) -> &'static str {
    match (model, is_fast) {
        (ModelType::Gemini, true) => "gemini-2.5-flash-lite",
        (ModelType::Gemini, false) => "gemini-2.5-pro",
        (ModelType::Claude, true) => "claude-haiku-4-5-20251101",
        (ModelType::Claude, false) => "claude-sonnet-4-5-20251101",
        (ModelType::Codex, true) => "gpt-4.1-nano",
        (ModelType::Codex, false) => "gpt-5.2",
    }
}

/// Get a simulated token stream for a model (for demo/offline mode)
pub fn stream_model(model: &str, prompt: &str) -> Vec<String> {
    match model {
        "gemini" => gemini::stream(prompt),
        "claude" => claude::stream(prompt),
        "codex" => codex::stream(prompt),
        _ => vec![format!("Unknown model: {}", model)],
    }
}

/// Check if a model is available (API key configured)
pub fn is_available(model: &ModelType) -> bool {
    match model {
        ModelType::Gemini => std::env::var("GEMINI_API_KEY").is_ok(),
        ModelType::Claude => std::env::var("ANTHROPIC_API_KEY").is_ok(),
        ModelType::Codex => std::env::var("OPENAI_API_KEY").is_ok(),
    }
}

/// Get the first available model in preference order
pub fn first_available(preference: &[ModelType]) -> Option<ModelType> {
    preference.iter().find(|m| is_available(m)).cloned()
}

/// Get model name as string
pub fn model_name(model: &ModelType) -> &'static str {
    match model {
        ModelType::Gemini => "gemini",
        ModelType::Claude => "claude",
        ModelType::Codex => "codex",
    }
}

/// List all available models across all providers
pub fn list_models() -> String {
    let registry = ModelRegistry::new();
    let models = registry.list_all();
    format_model_list(&models)
}

/// List models for a specific provider
pub fn list_models_for_provider(provider: &str) -> String {
    let registry = ModelRegistry::new();
    let models = registry.list_by_provider(provider);
    format_model_list(&models)
}

/// Validate that a model ID exists
pub fn validate_model_id(model_id: &str) -> bool {
    let registry = ModelRegistry::new();
    registry.get(model_id).is_some()
}

/// Get provider for a model ID
pub fn provider_for_model(model_id: &str) -> Option<ModelType> {
    let registry = ModelRegistry::new();
    registry.get(model_id).and_then(|m| ModelType::from_str(&m.provider))
}
