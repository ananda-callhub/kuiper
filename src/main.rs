mod cache;
mod cli;
mod config;
mod controller;
mod escalation;
mod fallback;
mod models;
mod orchestrator;
mod retry;
mod routing;
mod streaming;
mod telemetry;
mod tuning;

use anyhow::Result;
use cli::{Cli, Commands};
use clap::Parser;
use models::{ModelRegistry, ModelTier, ModelType, format_model_list};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::load_config()?;

    // Load or create tuning parameters
    let mut tuning_params = tuning::TuningParams::load().unwrap_or_default();

    match cli.command {
        Commands::Do { prompt, fast, full, budget, parallel, model, research } => {
            let _budget = budget.unwrap_or(config.default_budget);

            // Save prompt to history
            let _ = telemetry::save_prompt(&prompt);

            // Handle specific model selection
            if let Some(model_id) = model {
                println!("[Model] Using specific model: {}", model_id);

                // Determine provider from model ID
                if let Some(provider) = models::provider_for_model(&model_id) {
                    let result = models::run_model_with_id(provider, &model_id, &prompt, &config).await?;
                    streaming::print_streamed(&result);
                } else {
                    // Try to guess provider from model ID prefix
                    let provider = if model_id.starts_with("gemini") {
                        ModelType::Gemini
                    } else if model_id.starts_with("claude") {
                        ModelType::Claude
                    } else {
                        ModelType::Codex // Default to OpenAI
                    };
                    let result = models::run_model_with_id(provider, &model_id, &prompt, &config).await?;
                    streaming::print_streamed(&result);
                }
                return Ok(());
            }

            if parallel {
                // Parallel multi-model orchestration
                println!("[Parallel Mode] Running all models...\n");
                let result = orchestrator::run_parallel(&prompt, &config).await?;

                println!("\n=== Results ===");
                for r in &result.results {
                    let status = if r.success { "✓" } else { "✗" };
                    let fallback = if r.was_fallback { " (fallback)" } else { "" };
                    println!("[{}] {:?}{} ({}ms)", status, r.model, fallback, r.duration_ms);
                }
                println!("\nConsensus Score: {:.2}", result.consensus_score);
                println!("\n=== Best Response ===\n");
                streaming::print_streamed(&result.best_response);
            } else {
                let use_fast_path = if full || research {
                    false
                } else if fast {
                    true
                } else {
                    // Use tuned parameters for routing decision
                    config.fast_path && tuning::should_use_fast_path(&prompt, &tuning_params)
                };

                if use_fast_path {
                    telemetry::log_event("fast_path_start", &prompt);
                    println!("[Mode] Fast path (using fast-tier models)");
                    let result = routing::fast_path::run_fast_path(&prompt, &config).await?;

                    if escalation::should_escalate(&result) {
                        telemetry::log_event("escalation_triggered", &prompt);
                        let plan = controller::create_plan(&prompt, &config).await?;
                        controller::execute_plan(plan, &config).await?;
                    }
                } else {
                    if research {
                        // Research mode: use complex models with tiered fallback
                        // Falls back to lighter models if complex model quota exhausted
                        telemetry::log_event("research_mode_start", &prompt);
                        println!("[Mode] Research (using complex-tier models with tiered fallback)");
                        let result = routing::fast_path::run_research_path(&prompt, &config).await?;
                        streaming::print_streamed(&result);
                    } else {
                        // Full orchestration: multi-step planning
                        telemetry::log_event("full_path_start", &prompt);
                        println!("[Mode] Full orchestration");
                        let plan = controller::create_plan(&prompt, &config).await?;
                        controller::execute_plan(plan, &config).await?;
                    }
                }
            }

            // Update tuning parameters periodically
            let _ = tuning_params.update_from_telemetry();
        }

        Commands::Retry => {
            let last = telemetry::get_last_prompt()?;
            println!("Retrying: {}", last);
            let plan = controller::create_plan(&last, &config).await?;
            controller::execute_plan(plan, &config).await?;
        }

        Commands::Escalate { prompt } => {
            telemetry::log_event("forced_escalation", &prompt);
            let _ = telemetry::save_prompt(&prompt);
            println!("[Escalate] Forcing full orchestration...\n");
            let plan = controller::create_plan(&prompt, &config).await?;
            controller::execute_plan(plan, &config).await?;
        }

        Commands::Config => {
            println!("{}", toml::to_string_pretty(&config)?);
        }

        Commands::Models { provider, fast, complex } => {
            let registry = ModelRegistry::new();

            let models: Vec<_> = if let Some(p) = provider {
                registry.list_by_provider(&p)
            } else if fast {
                registry.list_by_tier(ModelTier::Fast)
            } else if complex {
                registry.list_by_tier(ModelTier::Complex)
            } else {
                registry.list_all()
            };

            if models.is_empty() {
                println!("No models found matching the criteria.");
            } else {
                println!("Available Models:\n");
                println!("{}", format_model_list(&models));
                println!("\nUsage:");
                println!("  kuiper do --model <model-id> \"your prompt\"");
                println!("  kuiper do --fast \"quick task\"      # Uses fast-tier models");
                println!("  kuiper do --research \"complex task\" # Uses complex-tier models");

                // Show availability status
                println!("\nAPI Key Status:");
                let gemini_status = if models::is_available(&ModelType::Gemini) { "✓" } else { "✗" };
                let claude_status = if models::is_available(&ModelType::Claude) { "✓" } else { "✗" };
                let codex_status = if models::is_available(&ModelType::Codex) { "✓" } else { "✗" };
                println!("  [{}] GEMINI_API_KEY", gemini_status);
                println!("  [{}] ANTHROPIC_API_KEY", claude_status);
                println!("  [{}] OPENAI_API_KEY", codex_status);
            }
        }

        Commands::Cache { action } => {
            match action.as_str() {
                "stats" => {
                    let cache = cache::ResponseCache::load()?;
                    let stats = cache.stats();
                    println!("Cache Statistics:");
                    println!("  Total entries:   {}", stats.total_entries);
                    println!("  Valid entries:   {}", stats.valid_entries);
                    println!("  Expired entries: {}", stats.expired_entries);
                }
                "clear" => {
                    let mut cache = cache::ResponseCache::load()?;
                    cache.clear();
                    cache.save()?;
                    println!("Cache cleared.");
                }
                "cleanup" => {
                    let mut cache = cache::ResponseCache::load()?;
                    let removed = cache.cleanup();
                    cache.save()?;
                    println!("Removed {} expired entries.", removed);
                }
                _ => {
                    println!("Unknown cache action. Use: stats, clear, cleanup");
                }
            }
        }

        Commands::Tuning { action } => {
            match action.as_str() {
                "show" => {
                    let params = tuning::TuningParams::load()?;
                    println!("Tuning Parameters:");
                    println!("  Fast-path word threshold: {}", params.fast_path_word_threshold);
                    println!("  Fast-path confidence:     {:.2}", params.fast_path_confidence);
                    println!("  Escalation rate:          {:.2}", params.escalation_rate);
                    println!("  Model scores:");
                    println!("    Gemini: {:.2}", params.model_scores.gemini);
                    println!("    Claude: {:.2}", params.model_scores.claude);
                    println!("    Codex:  {:.2}", params.model_scores.codex);
                }
                "reset" => {
                    let params = tuning::TuningParams::default();
                    params.save()?;
                    println!("Tuning parameters reset to defaults.");
                }
                "update" => {
                    let mut params = tuning::TuningParams::load()?;
                    params.update_from_telemetry()?;
                    println!("Tuning parameters updated from telemetry.");
                }
                _ => {
                    println!("Unknown tuning action. Use: show, reset, update");
                }
            }
        }
    }

    Ok(())
}
