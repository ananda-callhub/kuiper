use anyhow::Result;
use std::time::Instant;
use tokio::join;
use crate::config::Config;
use crate::models::{self, ModelType};
use crate::cache;
use crate::fallback::{self, FallbackChain};
use crate::retry::{retry_with_backoff, RetryConfig};
use crate::telemetry;

/// Result from a single model
#[derive(Debug, Clone)]
pub struct ModelResult {
    pub model: ModelType,
    pub response: String,
    pub duration_ms: u64,
    pub success: bool,
    pub was_fallback: bool,
}

/// Aggregated results from parallel execution
#[derive(Debug)]
pub struct OrchestratorResult {
    pub results: Vec<ModelResult>,
    pub best_response: String,
    pub consensus_score: f32,
}

/// Run all available models in parallel and aggregate results
pub async fn run_parallel(prompt: &str, config: &Config) -> Result<OrchestratorResult> {
    telemetry::log_event("parallel_orchestration_start", prompt);

    let start = Instant::now();

    // Run all models in parallel with individual fallback
    let (gemini_result, claude_result, codex_result) = join!(
        run_model_with_fallback(ModelType::Gemini, prompt, config),
        run_model_with_fallback(ModelType::Claude, prompt, config),
        run_model_with_fallback(ModelType::Codex, prompt, config),
    );

    let results = vec![gemini_result, claude_result, codex_result];

    // Filter successful results
    let successful: Vec<&ModelResult> = results.iter().filter(|r| r.success).collect();

    if successful.is_empty() {
        anyhow::bail!("All models failed to generate a response");
    }

    // Calculate consensus and select best response
    let (best_response, consensus_score) = aggregate_results(&successful);

    let total_duration = start.elapsed().as_millis() as u64;
    telemetry::log_event(
        "parallel_orchestration_complete",
        &format!(
            "models={}, consensus={:.2}, duration={}ms",
            successful.len(),
            consensus_score,
            total_duration
        ),
    );

    Ok(OrchestratorResult {
        results,
        best_response,
        consensus_score,
    })
}

/// Run two models in parallel (faster, cheaper)
pub async fn run_dual(
    prompt: &str,
    primary: ModelType,
    secondary: ModelType,
    config: &Config,
) -> Result<OrchestratorResult> {
    telemetry::log_event("dual_orchestration_start", prompt);

    let (primary_result, secondary_result) = join!(
        run_model_with_fallback(primary.clone(), prompt, config),
        run_model_with_fallback(secondary.clone(), prompt, config),
    );

    let results = vec![primary_result, secondary_result];
    let successful: Vec<&ModelResult> = results.iter().filter(|r| r.success).collect();

    if successful.is_empty() {
        anyhow::bail!("Both models failed to generate a response");
    }

    let (best_response, consensus_score) = aggregate_results(&successful);

    Ok(OrchestratorResult {
        results,
        best_response,
        consensus_score,
    })
}

/// Run a single model with automatic fallback to others if it fails
async fn run_model_with_fallback(primary: ModelType, prompt: &str, config: &Config) -> ModelResult {
    let start = Instant::now();
    let chain = FallbackChain::starting_with(primary.clone());

    // Try with the full fallback chain
    match fallback::execute_with_fallback(prompt, &chain, config).await {
        Ok((response, used_model, _attempts)) => {
            let was_fallback = used_model != primary;

            ModelResult {
                model: used_model,
                response,
                duration_ms: start.elapsed().as_millis() as u64,
                success: true,
                was_fallback,
            }
        }
        Err(e) => {
            ModelResult {
                model: primary,
                response: format!("Error: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
                was_fallback: false,
            }
        }
    }
}

/// Run a single model with retry, caching, and error handling (no fallback)
async fn run_model_safe(model: ModelType, prompt: &str, config: &Config) -> ModelResult {
    let model_name = models::model_name(&model);
    let start = Instant::now();

    // Check if model is available
    if !models::is_available(&model) {
        return ModelResult {
            model,
            response: format!("Model {} not available (no API key)", model_name),
            duration_ms: 0,
            success: false,
            was_fallback: false,
        };
    }

    // Try to get from cache first
    if let Ok(cache_store) = cache::ResponseCache::load() {
        if let Some(cached) = cache_store.get(prompt, model_name) {
            return ModelResult {
                model,
                response: cached.to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                success: true,
                was_fallback: false,
            };
        }
    }

    // Run with retry
    let retry_config = RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 500,
        max_delay_ms: 5000,
        exponential_base: 2,
    };

    let result = retry_with_backoff(
        || {
            let m = model.clone();
            let p = prompt.to_string();
            let c = config.clone();
            async move { models::run_model(m, &p, &c).await }
        },
        &retry_config,
    )
    .await;

    match result {
        Ok(response) => {
            // Cache the successful response
            if let Ok(mut cache_store) = cache::ResponseCache::load() {
                cache_store.set(prompt, model_name, &response);
                let _ = cache_store.save();
            }

            ModelResult {
                model,
                response,
                duration_ms: start.elapsed().as_millis() as u64,
                success: true,
                was_fallback: false,
            }
        }
        Err(e) => ModelResult {
            model,
            response: format!("Error: {}", e),
            duration_ms: start.elapsed().as_millis() as u64,
            success: false,
            was_fallback: false,
        },
    }
}

/// Aggregate results from multiple models
fn aggregate_results(results: &[&ModelResult]) -> (String, f32) {
    if results.is_empty() {
        return (String::new(), 0.0);
    }

    if results.len() == 1 {
        return (results[0].response.clone(), 1.0);
    }

    // Simple consensus: check for similar responses
    let responses: Vec<&str> = results.iter().map(|r| r.response.as_str()).collect();

    // Calculate similarity scores between all pairs
    let mut total_similarity = 0.0;
    let mut comparisons = 0;

    for i in 0..responses.len() {
        for j in (i + 1)..responses.len() {
            total_similarity += jaccard_similarity(responses[i], responses[j]);
            comparisons += 1;
        }
    }

    let consensus_score = if comparisons > 0 {
        total_similarity / comparisons as f32
    } else {
        1.0
    };

    // Select best response:
    // 1. Prefer non-fallback responses (direct model match)
    // 2. Then prefer by response length (heuristic for quality)
    let best = results
        .iter()
        .max_by(|a, b| {
            // Prefer non-fallback
            if a.was_fallback != b.was_fallback {
                return b.was_fallback.cmp(&a.was_fallback);
            }
            // Then prefer by response length
            a.response.len().cmp(&b.response.len())
        })
        .unwrap();

    (best.response.clone(), consensus_score)
}

/// Calculate Jaccard similarity between two texts (word-level)
fn jaccard_similarity(a: &str, b: &str) -> f32 {
    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return 1.0;
    }

    intersection as f32 / union as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_similarity_identical() {
        let sim = jaccard_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_similarity_different() {
        let sim = jaccard_similarity("hello world", "goodbye moon");
        assert!(sim < 0.5);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let sim = jaccard_similarity("hello world foo", "hello world bar");
        assert!(sim > 0.3 && sim < 0.8);
    }
}
