use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model tier for routing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Fast, cheap models for simple tasks
    Fast,
    /// Balanced models for general use
    Balanced,
    /// Powerful models for complex reasoning/research
    Complex,
}

/// Information about a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub tier: ModelTier,
    pub max_tokens: u32,
    pub context_window: u32,
    pub cost_per_1k_input: f32,
    pub cost_per_1k_output: f32,
    pub description: String,
}

/// Registry of all available models
pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Gemini models (2026)
        // Note: Gemini 1.5 and 2.0 are retired. Using 2.5 and 3.x series.
        self.register(ModelInfo {
            id: "gemini-2.5-flash-lite".into(),
            name: "Gemini 2.5 Flash-Lite".into(),
            provider: "gemini".into(),
            tier: ModelTier::Fast,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            description: "High-throughput, cost-optimized for scale".into(),
        });
        self.register(ModelInfo {
            id: "gemini-2.5-flash".into(),
            name: "Gemini 2.5 Flash".into(),
            provider: "gemini".into(),
            tier: ModelTier::Balanced,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.00015,
            cost_per_1k_output: 0.0006,
            description: "Fast with controllable thinking budgets".into(),
        });
        self.register(ModelInfo {
            id: "gemini-2.5-pro".into(),
            name: "Gemini 2.5 Pro".into(),
            provider: "gemini".into(),
            tier: ModelTier::Complex,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.00125,
            cost_per_1k_output: 0.005,
            description: "Adaptive thinking for complex reasoning".into(),
        });
        self.register(ModelInfo {
            id: "gemini-3-pro-preview".into(),
            name: "Gemini 3 Pro (Preview)".into(),
            provider: "gemini".into(),
            tier: ModelTier::Complex,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.002,
            cost_per_1k_output: 0.008,
            description: "State-of-the-art reasoning and agentic".into(),
        });

        // Claude models (2026)
        // Claude 4.5 family is current, 3.x models are deprecated
        self.register(ModelInfo {
            id: "claude-haiku-4-5-20251101".into(),
            name: "Claude Haiku 4.5".into(),
            provider: "claude".into(),
            tier: ModelTier::Fast,
            max_tokens: 8192,
            context_window: 200_000,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.005,
            description: "Fastest Claude, great for rapid tasks".into(),
        });
        self.register(ModelInfo {
            id: "claude-sonnet-4-5-20251101".into(),
            name: "Claude Sonnet 4.5".into(),
            provider: "claude".into(),
            tier: ModelTier::Balanced,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
            description: "Best balance of speed and capability".into(),
        });
        self.register(ModelInfo {
            id: "claude-opus-4-5-20251101".into(),
            name: "Claude Opus 4.5".into(),
            provider: "claude".into(),
            tier: ModelTier::Complex,
            max_tokens: 8192,
            context_window: 200_000,
            cost_per_1k_input: 0.005,
            cost_per_1k_output: 0.025,
            description: "Best for coding, agents, computer use".into(),
        });

        // OpenAI models (2026)
        // GPT-4o is legacy, using GPT-4.1 and o-series
        self.register(ModelInfo {
            id: "gpt-4.1-nano".into(),
            name: "GPT-4.1 Nano".into(),
            provider: "codex".into(),
            tier: ModelTier::Fast,
            max_tokens: 16384,
            context_window: 128_000,
            cost_per_1k_input: 0.0001,
            cost_per_1k_output: 0.0004,
            description: "Fastest, most cost-efficient GPT".into(),
        });
        self.register(ModelInfo {
            id: "gpt-4.1-mini".into(),
            name: "GPT-4.1 Mini".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 16384,
            context_window: 128_000,
            cost_per_1k_input: 0.0004,
            cost_per_1k_output: 0.0016,
            description: "Smaller, faster version of GPT-4.1".into(),
        });
        self.register(ModelInfo {
            id: "gpt-4.1".into(),
            name: "GPT-4.1".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 16384,
            context_window: 128_000,
            cost_per_1k_input: 0.002,
            cost_per_1k_output: 0.008,
            description: "Smartest non-reasoning model".into(),
        });
        self.register(ModelInfo {
            id: "o4-mini".into(),
            name: "O4 Mini".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 65536,
            context_window: 128_000,
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.012,
            description: "Reasoning at a fraction of the cost".into(),
        });
        self.register(ModelInfo {
            id: "o3".into(),
            name: "O3".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 100_000,
            context_window: 200_000,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.04,
            description: "Powerful reasoning model".into(),
        });
        self.register(ModelInfo {
            id: "o3-pro".into(),
            name: "O3 Pro".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 100_000,
            context_window: 200_000,
            cost_per_1k_input: 0.02,
            cost_per_1k_output: 0.08,
            description: "Enhanced reasoning, reliable responses".into(),
        });

        // GPT-5 series (flagship models)
        self.register(ModelInfo {
            id: "gpt-5".into(),
            name: "GPT-5".into(),
            provider: "codex".into(),
            tier: ModelTier::Complex,
            max_tokens: 32768,
            context_window: 400_000,
            cost_per_1k_input: 0.00125,
            cost_per_1k_output: 0.01,
            description: "Flagship model with configurable reasoning".into(),
        });
        self.register(ModelInfo {
            id: "gpt-5-codex".into(),
            name: "GPT-5 Codex".into(),
            provider: "codex".into(),
            tier: ModelTier::Complex,
            max_tokens: 32768,
            context_window: 400_000,
            cost_per_1k_input: 0.00125,
            cost_per_1k_output: 0.01,
            description: "Optimized for coding and agentic tasks".into(),
        });
        self.register(ModelInfo {
            id: "gpt-5.1".into(),
            name: "GPT-5.1".into(),
            provider: "codex".into(),
            tier: ModelTier::Complex,
            max_tokens: 32768,
            context_window: 400_000,
            cost_per_1k_input: 0.0015,
            cost_per_1k_output: 0.012,
            description: "Advanced reasoning for coding/agents".into(),
        });
        self.register(ModelInfo {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            provider: "codex".into(),
            tier: ModelTier::Complex,
            max_tokens: 32768,
            context_window: 400_000,
            cost_per_1k_input: 0.00175,
            cost_per_1k_output: 0.014,
            description: "Latest flagship for coding and agents".into(),
        });
    }

    pub fn register(&mut self, model: ModelInfo) {
        self.models.insert(model.id.clone(), model);
    }

    pub fn get(&self, id: &str) -> Option<&ModelInfo> {
        self.models.get(id)
    }

    pub fn list_all(&self) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.models.values().collect();
        models.sort_by(|a, b| {
            a.provider.cmp(&b.provider)
                .then(a.tier.cmp(&b.tier))
                .then(a.name.cmp(&b.name))
        });
        models
    }

    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.provider == provider)
            .collect()
    }

    pub fn list_by_tier(&self, tier: ModelTier) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.tier == tier)
            .collect()
    }

    /// Get the default fast model for a provider
    pub fn default_fast(&self, provider: &str) -> Option<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.provider == provider && m.tier == ModelTier::Fast)
            .next()
    }

    /// Get the default balanced model for a provider
    pub fn default_balanced(&self, provider: &str) -> Option<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.provider == provider && m.tier == ModelTier::Balanced)
            .next()
    }

    /// Get the default complex model for a provider
    pub fn default_complex(&self, provider: &str) -> Option<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.provider == provider && m.tier == ModelTier::Complex)
            .next()
    }

    /// Get recommended model based on task type
    pub fn recommend(&self, provider: &str, is_fast_path: bool) -> Option<&ModelInfo> {
        if is_fast_path {
            self.default_fast(provider)
                .or_else(|| self.default_balanced(provider))
        } else {
            self.default_complex(provider)
                .or_else(|| self.default_balanced(provider))
        }
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::cmp::PartialOrd for ModelTier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for ModelTier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_val = match self {
            ModelTier::Fast => 0,
            ModelTier::Balanced => 1,
            ModelTier::Complex => 2,
        };
        let other_val = match other {
            ModelTier::Fast => 0,
            ModelTier::Balanced => 1,
            ModelTier::Complex => 2,
        };
        self_val.cmp(&other_val)
    }
}

/// Format model list for display
pub fn format_model_list(models: &[&ModelInfo]) -> String {
    let mut output = String::new();

    let mut current_provider = "";
    for model in models {
        if model.provider != current_provider {
            if !current_provider.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("{}:\n", model.provider.to_uppercase()));
            current_provider = &model.provider;
        }

        let tier_badge = match model.tier {
            ModelTier::Fast => "⚡ fast",
            ModelTier::Balanced => "⚖️  balanced",
            ModelTier::Complex => "🧠 complex",
        };

        output.push_str(&format!(
            "  {:<30} [{:<12}] {}\n",
            model.id, tier_badge, model.description
        ));
    }

    output
}
