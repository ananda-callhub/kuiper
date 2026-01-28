use anyhow::Result;
use std::time::Instant;
use crate::config::Config;
use crate::fallback::{self, FallbackChain, TieredFallbackChain};
use crate::models::{self, ModelType};
use crate::routing::heuristics;
use crate::streaming;
use crate::telemetry;

/// Keywords that indicate a complex task requiring full orchestration
const COMPLEX_KEYWORDS: &[&str] = &[
    "design",
    "architecture",
    "compare",
    "tradeoff",
    "trade-off",
    "optimize",
    "refactor",
    "migrate",
    "plan",
    "strategy",
    "review",
    "analyze",
];

/// Determine if a prompt qualifies for fast-path execution
pub fn is_fast_path(prompt: &str, config: &Config) -> bool {
    let word_count = prompt.split_whitespace().count();
    let prompt_lower = prompt.to_lowercase();

    // Check word count threshold
    if word_count > config.fast_path_settings.max_word_count {
        return false;
    }

    // Check for complex keywords
    if COMPLEX_KEYWORDS.iter().any(|k| prompt_lower.contains(k)) {
        return false;
    }

    // Check historical escalation rate
    let confidence = heuristics::fast_path_confidence(word_count, &prompt_lower);
    confidence > (1.0 - config.fast_path_settings.escalation_threshold)
}

/// Execute a fast-path request with automatic fallback
pub async fn run_fast_path(prompt: &str, config: &Config) -> Result<String> {
    // Select the best fallback chain based on prompt
    let chain = fallback::select_chain_for_prompt(prompt);
    let primary_model = chain.models.first().cloned().unwrap_or(ModelType::Gemini);

    println!("[Fast Path] Primary model: {:?}", primary_model);
    println!("[Fast Path] Fallback chain: {:?}", chain.models.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join(" → "));

    // Save prompt to history
    let _ = telemetry::save_prompt(prompt);

    let start = Instant::now();

    // Execute with automatic fallback
    let (response, used_model, attempts) = fallback::execute_with_fallback(prompt, &chain, config).await?;

    let duration = start.elapsed().as_millis() as u64;
    let model_name = models::model_name(&used_model);

    // Log if we had to fallback
    if attempts.len() > 1 {
        let failed: Vec<String> = attempts
            .iter()
            .filter(|a| !a.success)
            .map(|a| format!("{:?}", a.model))
            .collect();
        println!("[Fast Path] Fell back from {} to {}", failed.join(", "), model_name);
    }

    telemetry::log_usage(prompt, true, model_name, duration);

    Ok(response)
}

/// Execute fast-path with streaming output (demo mode)
pub async fn run_fast_path_streaming(prompt: &str, config: &Config) -> Result<String> {
    let chain = fallback::select_chain_for_prompt(prompt);
    let primary_model = chain.models.first().cloned().unwrap_or(ModelType::Gemini);
    let model_name = models::model_name(&primary_model);

    println!("[Fast Path Streaming] Using {}", model_name);

    let _ = telemetry::save_prompt(prompt);

    let start = Instant::now();
    let tokens = models::stream_model(model_name, prompt);
    streaming::stream_output(tokens).await;

    let duration = start.elapsed().as_millis() as u64;
    telemetry::log_usage(prompt, true, model_name, duration);

    Ok(format!("Streamed response for: {}", prompt))
}

/// Execute a fast-path request with tiered fallback (Complex → Balanced → Fast)
/// Use this when you want to start with fast models but allow fallback to any tier
pub async fn run_fast_path_tiered(prompt: &str, config: &Config) -> Result<String> {
    let chain = fallback::select_tiered_chain_for_prompt(prompt, false);

    if chain.instances.is_empty() {
        anyhow::bail!("No models available. Please configure API keys.");
    }

    let primary = &chain.instances[0];
    println!("[Fast Path] Primary model: {} ({:?} tier)", primary.model_id, primary.tier);
    println!(
        "[Fast Path] Fallback chain: {}",
        chain.instances.iter().map(|i| i.model_id.as_str()).collect::<Vec<_>>().join(" → ")
    );

    let _ = telemetry::save_prompt(prompt);
    let start = Instant::now();

    let (response, _provider, model_id, attempts) =
        fallback::execute_with_tiered_fallback(prompt, &chain, config).await?;

    let duration = start.elapsed().as_millis() as u64;

    if attempts.len() > 1 {
        let failed: Vec<String> = attempts
            .iter()
            .filter(|a| !a.success)
            .map(|a| a.model_id.clone())
            .collect();
        println!("[Fast Path] Fell back from {} to {}", failed.join(", "), model_id);
    }

    telemetry::log_usage(prompt, true, &model_id, duration);
    Ok(response)
}

/// Execute a research/complex task with tiered fallback
/// Starts with complex-tier models and falls back to balanced then fast if quota exhausted
pub async fn run_research_path(prompt: &str, config: &Config) -> Result<String> {
    let chain = fallback::select_tiered_chain_for_prompt(prompt, true);

    if chain.instances.is_empty() {
        anyhow::bail!("No models available. Please configure API keys.");
    }

    let primary = &chain.instances[0];
    println!("[Research Mode] Primary model: {} ({:?} tier)", primary.model_id, primary.tier);
    println!(
        "[Research Mode] Fallback chain: {}",
        chain.instances.iter().map(|i| format!("{}({:?})", i.model_id, i.tier)).collect::<Vec<_>>().join(" → ")
    );

    let _ = telemetry::save_prompt(prompt);
    let start = Instant::now();

    let (response, _provider, model_id, attempts) =
        fallback::execute_with_tiered_fallback(prompt, &chain, config).await?;

    let duration = start.elapsed().as_millis() as u64;

    if attempts.len() > 1 {
        let failed: Vec<String> = attempts
            .iter()
            .filter(|a| !a.success)
            .map(|a| format!("{}({:?})", a.model_id, a.model))
            .collect();
        println!(
            "[Research Mode] Fell back from {} to {} (lighter model)",
            failed.join(", "),
            model_id
        );
        telemetry::log_event(
            "research_fallback_to_lighter",
            &format!("from={}, to={}", failed.join(","), model_id),
        );
    }

    telemetry::log_usage(prompt, false, &model_id, duration);
    Ok(response)
}

/// Select the appropriate model for fast-path execution (without fallback)
pub fn select_fast_path_model(prompt: &str, config: &Config) -> ModelType {
    // Prefer Codex for code-like prompts
    if looks_like_code(prompt) && config.models.codex {
        return ModelType::Codex;
    }

    // Default to Gemini for speed
    if config.models.gemini {
        return ModelType::Gemini;
    }

    // Fallback chain
    if config.models.claude {
        return ModelType::Claude;
    }

    ModelType::Codex
}

/// Heuristic check if prompt appears to be code-related
fn looks_like_code(prompt: &str) -> bool {
    let code_indicators = [
        "{", "}", ";", "//", "/*", "=>", "->",
        "fn ", "func ", "def ", "class ", "import ",
        "const ", "let ", "var ",
    ];

    code_indicators.iter().any(|&ind| prompt.contains(ind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_code() {
        assert!(looks_like_code("fn main() { }"));
        assert!(looks_like_code("def hello():"));
        assert!(!looks_like_code("explain how AI works"));
    }

    #[test]
    fn test_is_fast_path() {
        let config = Config::default();

        // Short, simple prompts should be fast path
        assert!(is_fast_path("convert json to yaml", &config));

        // Complex keywords should not be fast path
        assert!(!is_fast_path("design an authentication system", &config));
    }
}
