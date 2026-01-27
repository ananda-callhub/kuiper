use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "kuiper")]
#[command(author, version, about = "Multi-model AI orchestrator CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute an AI task with automatic model routing
    Do {
        /// The prompt or task description
        prompt: String,

        /// Force fast-path (single model, no planning)
        #[arg(long, conflicts_with = "full")]
        fast: bool,

        /// Force full orchestration (planning + multi-model)
        #[arg(long, conflicts_with = "fast")]
        full: bool,

        /// Run all models in parallel and aggregate results
        #[arg(long, short)]
        parallel: bool,

        /// Use a specific model (e.g., gpt-4o, claude-sonnet-4-20250514, gemini-1.5-pro)
        #[arg(long, short)]
        model: Option<String>,

        /// Use complex/research models (more capable but slower)
        #[arg(long, conflicts_with = "fast")]
        research: bool,

        /// Maximum budget for this request in dollars
        #[arg(long, short)]
        budget: Option<f32>,
    },

    /// Retry the last executed task
    Retry,

    /// Force escalation to full orchestration
    Escalate {
        /// The prompt to escalate
        prompt: String,
    },

    /// Show current configuration
    Config,

    /// List and manage available models
    Models {
        /// Filter by provider: gemini, claude, codex
        #[arg(long, short)]
        provider: Option<String>,

        /// Show only fast-tier models
        #[arg(long)]
        fast: bool,

        /// Show only complex-tier models
        #[arg(long)]
        complex: bool,
    },

    /// Manage response cache
    Cache {
        /// Action: stats, clear, cleanup
        #[arg(default_value = "stats")]
        action: String,
    },

    /// Manage auto-tuning parameters
    Tuning {
        /// Action: show, reset, update
        #[arg(default_value = "show")]
        action: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(["kuiper", "do", "test prompt"]);
        match cli.command {
            Commands::Do { prompt, fast, full, parallel, model, research, budget } => {
                assert_eq!(prompt, "test prompt");
                assert!(!fast);
                assert!(!full);
                assert!(!parallel);
                assert!(!research);
                assert!(model.is_none());
                assert!(budget.is_none());
            }
            _ => panic!("Expected Do command"),
        }
    }

    #[test]
    fn test_cli_model_flag() {
        let cli = Cli::parse_from(["kuiper", "do", "--model", "gpt-4o", "test"]);
        match cli.command {
            Commands::Do { model, .. } => {
                assert_eq!(model, Some("gpt-4o".to_string()));
            }
            _ => panic!("Expected Do command"),
        }
    }

    #[test]
    fn test_cli_research_flag() {
        let cli = Cli::parse_from(["kuiper", "do", "--research", "test"]);
        match cli.command {
            Commands::Do { research, .. } => {
                assert!(research);
            }
            _ => panic!("Expected Do command"),
        }
    }

    #[test]
    fn test_cli_models_command() {
        let cli = Cli::parse_from(["kuiper", "models"]);
        match cli.command {
            Commands::Models { provider, fast, complex } => {
                assert!(provider.is_none());
                assert!(!fast);
                assert!(!complex);
            }
            _ => panic!("Expected Models command"),
        }
    }

    #[test]
    fn test_cli_models_provider_filter() {
        let cli = Cli::parse_from(["kuiper", "models", "--provider", "claude"]);
        match cli.command {
            Commands::Models { provider, .. } => {
                assert_eq!(provider, Some("claude".to_string()));
            }
            _ => panic!("Expected Models command"),
        }
    }
}
