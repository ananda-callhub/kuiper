use anyhow::Result;
use std::time::Instant;
use crate::config::Config;
use crate::fallback::{self, FallbackChain};
use crate::models::{self, ModelType};
use crate::streaming;
use crate::telemetry;
use crate::escalation;

/// A step in the execution plan
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub description: String,
    pub model: ModelType,
    pub prompt: String,
    pub depends_on: Option<usize>,
}

/// Create an execution plan for a complex task
pub async fn create_plan(prompt: &str, config: &Config) -> Result<Vec<PlanStep>> {
    telemetry::log_event("planning_start", prompt);

    // Save prompt to history
    let _ = telemetry::save_prompt(prompt);

    // For now, create a simple single-step plan
    // TODO: Use Codex to generate multi-step plans for complex tasks
    let plan = vec![PlanStep {
        description: "Execute task".to_string(),
        model: select_model_for_task(prompt),
        prompt: prompt.to_string(),
        depends_on: None,
    }];

    telemetry::log_event("planning_complete", &format!("{} steps", plan.len()));
    Ok(plan)
}

/// Execute a plan step by step with automatic fallback
pub async fn execute_plan(plan: Vec<PlanStep>, config: &Config) -> Result<()> {
    let mut previous_output: Option<String> = None;
    let mut attempt = 0u32;

    for (i, step) in plan.iter().enumerate() {
        println!("\n[Step {}/{}] {}", i + 1, plan.len(), step.description);

        let prompt = if let Some(ref prev) = previous_output {
            format!("Previous context:\n{}\n\nTask: {}", prev, step.prompt)
        } else {
            step.prompt.clone()
        };

        // Create fallback chain starting with the planned model
        let chain = FallbackChain::starting_with(step.model.clone());
        println!("Fallback chain: {:?}", chain.models.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join(" → "));

        let start = Instant::now();

        // Check if any model is available
        let available = chain.available_only();
        let output = if available.models.is_empty() {
            // Demo mode: use simulated streaming
            let model_name = models::model_name(&step.model);
            println!("[Demo Mode] No API keys available, using simulated response");
            let tokens = models::stream_model(model_name, &prompt);
            streaming::stream_output(tokens).await;
            format!("Simulated response for step {}", i + 1)
        } else {
            // Execute with automatic fallback
            match fallback::execute_with_fallback(&prompt, &chain, config).await {
                Ok((response, used_model, attempts)) => {
                    let model_name = models::model_name(&used_model);

                    // Log if we had to fallback
                    if attempts.len() > 1 {
                        let failed: Vec<String> = attempts
                            .iter()
                            .filter(|a| !a.success)
                            .map(|a| format!("{:?}", a.model))
                            .collect();
                        println!("[Controller] Fell back from {} to {}", failed.join(", "), model_name);
                    }

                    if config.streaming {
                        streaming::print_streamed(&response);
                    } else {
                        println!("{}", response);
                    }

                    response
                }
                Err(e) => {
                    eprintln!("[Controller] All models failed: {}", e);
                    format!("Error: {}", e)
                }
            }
        };

        let duration = start.elapsed().as_millis() as u64;
        let model_name = models::model_name(&step.model);
        telemetry::log_usage(&prompt, false, model_name, duration);

        // Check for escalation
        if escalation::should_escalate(&output) {
            let strategy = escalation::determine_strategy(&output, attempt);
            match strategy {
                escalation::EscalationStrategy::RetryDifferentModel => {
                    eprintln!("⚠️  Uncertain response detected, would retry with different model");
                    attempt += 1;
                }
                escalation::EscalationStrategy::FullOrchestration => {
                    eprintln!("⚠️  Escalating to full orchestration");
                    telemetry::log_event("escalation_triggered", &step.prompt);
                }
                escalation::EscalationStrategy::UserClarification => {
                    eprintln!("⚠️  Need user clarification for this task");
                }
                escalation::EscalationStrategy::None => {}
            }
        }

        previous_output = Some(output);
        telemetry::log_event("step_complete", &format!("step_{}", i + 1));
    }

    telemetry::log_event("plan_complete", "success");
    Ok(())
}

/// Select the best model for a given task based on heuristics
fn select_model_for_task(prompt: &str) -> ModelType {
    let prompt_lower = prompt.to_lowercase();

    // Code-related tasks -> Codex
    if prompt_lower.contains("code")
        || prompt_lower.contains("function")
        || prompt_lower.contains("debug")
        || prompt_lower.contains("implement")
        || prompt_lower.contains("fix bug")
        || contains_code_patterns(prompt)
    {
        return ModelType::Codex;
    }

    // Analysis, reasoning, writing -> Claude
    if prompt_lower.contains("analyze")
        || prompt_lower.contains("explain")
        || prompt_lower.contains("write")
        || prompt_lower.contains("review")
        || prompt_lower.contains("compare")
    {
        return ModelType::Claude;
    }

    // Default to Gemini for general tasks
    ModelType::Gemini
}

fn contains_code_patterns(prompt: &str) -> bool {
    prompt.contains("{")
        || prompt.contains("}")
        || prompt.contains("fn ")
        || prompt.contains("func ")
        || prompt.contains("def ")
        || prompt.contains("class ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_model_for_task() {
        assert_eq!(select_model_for_task("write a function"), ModelType::Codex);
        assert_eq!(select_model_for_task("analyze this data"), ModelType::Claude);
        assert_eq!(select_model_for_task("what is the weather"), ModelType::Gemini);
    }
}
