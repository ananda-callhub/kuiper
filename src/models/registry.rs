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
        // Gemini models (using actual API model IDs)
        self.register(ModelInfo {
            id: "gemini-1.5-flash".into(),
            name: "Gemini 1.5 Flash".into(),
            provider: "gemini".into(),
            tier: ModelTier::Fast,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.000075,
            cost_per_1k_output: 0.0003,
            description: "Fast and efficient for simple tasks".into(),
        });
        self.register(ModelInfo {
            id: "gemini-1.5-pro".into(),
            name: "Gemini 1.5 Pro".into(),
            provider: "gemini".into(),
            tier: ModelTier::Balanced,
            max_tokens: 8192,
            context_window: 2_000_000,
            cost_per_1k_input: 0.00125,
            cost_per_1k_output: 0.005,
            description: "Balanced performance with large context".into(),
        });
        self.register(ModelInfo {
            id: "gemini-2.0-flash-exp".into(),
            name: "Gemini 2.0 Flash (Exp)".into(),
            provider: "gemini".into(),
            tier: ModelTier::Complex,
            max_tokens: 8192,
            context_window: 1_000_000,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            description: "Latest experimental model".into(),
        });

        // Claude models (using actual API model IDs)
        self.register(ModelInfo {
            id: "claude-3-5-haiku-20241022".into(),
            name: "Claude 3.5 Haiku".into(),
            provider: "claude".into(),
            tier: ModelTier::Fast,
            max_tokens: 8192,
            context_window: 200_000,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.005,
            description: "Fastest Claude, great for rapid tasks".into(),
        });
        self.register(ModelInfo {
            id: "claude-3-5-sonnet-20241022".into(),
            name: "Claude 3.5 Sonnet".into(),
            provider: "claude".into(),
            tier: ModelTier::Balanced,
            max_tokens: 8192,
            context_window: 200_000,
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
            description: "Best balance of speed and capability".into(),
        });
        self.register(ModelInfo {
            id: "claude-3-opus-20240229".into(),
            name: "Claude 3 Opus".into(),
            provider: "claude".into(),
            tier: ModelTier::Complex,
            max_tokens: 4096,
            context_window: 200_000,
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.075,
            description: "Most capable for complex reasoning".into(),
        });

        // OpenAI models (using actual API model IDs)
        self.register(ModelInfo {
            id: "gpt-4o-mini".into(),
            name: "GPT-4o Mini".into(),
            provider: "codex".into(),
            tier: ModelTier::Fast,
            max_tokens: 16384,
            context_window: 128_000,
            cost_per_1k_input: 0.00015,
            cost_per_1k_output: 0.0006,
            description: "Fast, affordable for simple tasks".into(),
        });
        self.register(ModelInfo {
            id: "gpt-4o".into(),
            name: "GPT-4o".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 16384,
            context_window: 128_000,
            cost_per_1k_input: 0.0025,
            cost_per_1k_output: 0.01,
            description: "Best overall multimodal model".into(),
        });
        self.register(ModelInfo {
            id: "gpt-4-turbo".into(),
            name: "GPT-4 Turbo".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 4096,
            context_window: 128_000,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.03,
            description: "Powerful with large context window".into(),
        });
        self.register(ModelInfo {
            id: "o1-preview".into(),
            name: "O1 Preview".into(),
            provider: "codex".into(),
            tier: ModelTier::Complex,
            max_tokens: 32768,
            context_window: 128_000,
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.06,
            description: "Advanced reasoning model".into(),
        });
        self.register(ModelInfo {
            id: "o1-mini".into(),
            name: "O1 Mini".into(),
            provider: "codex".into(),
            tier: ModelTier::Balanced,
            max_tokens: 65536,
            context_window: 128_000,
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.012,
            description: "Efficient reasoning model".into(),
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
