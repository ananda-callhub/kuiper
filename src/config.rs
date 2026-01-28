use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "default_budget")]
    pub default_budget: f32,

    #[serde(default = "default_true")]
    pub fast_path: bool,

    #[serde(default = "default_true")]
    pub streaming: bool,

    #[serde(default)]
    pub fast_path_settings: FastPathSettings,

    #[serde(default)]
    pub models: ModelsConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FastPathSettings {
    #[serde(default = "default_word_count")]
    pub max_word_count: usize,

    #[serde(default = "default_escalation_threshold")]
    pub escalation_threshold: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelsConfig {
    #[serde(default = "default_true")]
    pub gemini: bool,

    #[serde(default = "default_true")]
    pub claude: bool,

    #[serde(default = "default_true")]
    pub codex: bool,

    #[serde(default)]
    pub gemini_settings: ModelSettings,

    #[serde(default)]
    pub claude_settings: ModelSettings,

    #[serde(default)]
    pub codex_settings: ModelSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelSettings {
    #[serde(default)]
    pub model: String,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

// Default value functions
fn default_budget() -> f32 { 0.25 }
fn default_true() -> bool { true }
fn default_word_count() -> usize { 25 }
fn default_escalation_threshold() -> f32 { 0.15 }
fn default_max_tokens() -> u32 { 8192 }

impl Default for FastPathSettings {
    fn default() -> Self {
        Self {
            max_word_count: default_word_count(),
            escalation_threshold: default_escalation_threshold(),
        }
    }
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            gemini: true,
            claude: true,
            codex: true,
            gemini_settings: ModelSettings {
                model: "gemini-2.5-flash".to_string(),
                max_tokens: 8192,
            },
            claude_settings: ModelSettings {
                model: "claude-sonnet-4-5-20251101".to_string(),
                max_tokens: 8192,
            },
            codex_settings: ModelSettings {
                model: "gpt-5.2".to_string(),
                max_tokens: 32768,
            },
        }
    }
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_budget: default_budget(),
            fast_path: true,
            streaming: true,
            fast_path_settings: FastPathSettings::default(),
            models: ModelsConfig::default(),
        }
    }
}

pub fn load_config() -> Result<Config> {
    // Try user config first
    if let Some(user_config) = user_config_path() {
        if user_config.exists() {
            let content = fs::read_to_string(&user_config)
                .with_context(|| format!("Failed to read config from {:?}", user_config))?;
            return toml::from_str(&content)
                .with_context(|| "Failed to parse user config");
        }
    }

    // Fall back to defaults
    let defaults = include_str!("../config/defaults.toml");
    toml::from_str(defaults).with_context(|| "Failed to parse default config")
}

pub fn user_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kuiper").join("config.toml"))
}

pub fn kuiper_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kuiper"))
}
