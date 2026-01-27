use anyhow::Result;
use crate::config::kuiper_data_dir;
use crate::telemetry;
use serde::{Deserialize, Serialize};
use std::fs;

const TUNING_FILE: &str = "tuning.json";

/// Auto-tuning parameters learned from telemetry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TuningParams {
    /// Adjusted word count threshold for fast-path
    pub fast_path_word_threshold: usize,

    /// Confidence threshold for fast-path routing
    pub fast_path_confidence: f32,

    /// Historical escalation rate
    pub escalation_rate: f32,

    /// Model preference scores (higher = preferred)
    pub model_scores: ModelScores,

    /// Last update timestamp
    pub last_updated: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelScores {
    pub gemini: f32,
    pub claude: f32,
    pub codex: f32,
}

impl Default for TuningParams {
    fn default() -> Self {
        Self {
            fast_path_word_threshold: 25,
            fast_path_confidence: 0.85,
            escalation_rate: 0.1,
            model_scores: ModelScores {
                gemini: 1.0,
                claude: 1.0,
                codex: 1.0,
            },
            last_updated: 0,
        }
    }
}

impl TuningParams {
    /// Load tuning parameters from disk
    pub fn load() -> Result<Self> {
        let path = kuiper_data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
            .join(TUNING_FILE);

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let params: TuningParams = serde_json::from_str(&content)?;

        Ok(params)
    }

    /// Save tuning parameters to disk
    pub fn save(&self) -> Result<()> {
        let path = kuiper_data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
            .join(TUNING_FILE);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Update parameters based on recent telemetry
    pub fn update_from_telemetry(&mut self) -> Result<()> {
        // Get escalation rate from last 24 hours
        let escalation_rate = telemetry::get_escalation_rate(24)?;

        // Adjust fast-path confidence based on escalation rate
        if escalation_rate > 0.2 {
            // Too many escalations, be more conservative
            self.fast_path_confidence = (self.fast_path_confidence + 0.05).min(0.95);
            self.fast_path_word_threshold = self.fast_path_word_threshold.saturating_sub(2);
        } else if escalation_rate < 0.05 {
            // Few escalations, can be more aggressive
            self.fast_path_confidence = (self.fast_path_confidence - 0.02).max(0.7);
            self.fast_path_word_threshold = (self.fast_path_word_threshold + 1).min(50);
        }

        self.escalation_rate = escalation_rate;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.save()?;

        Ok(())
    }
}

/// Calculate fast-path score using tuned parameters
pub fn fast_path_score(prompt_len: usize, params: &TuningParams) -> f32 {
    let mut score = 1.0_f32;

    // Penalize longer prompts
    if prompt_len > params.fast_path_word_threshold {
        score -= 0.3;
    }
    if prompt_len > params.fast_path_word_threshold * 2 {
        score -= 0.3;
    }

    // Adjust based on historical escalation rate
    if params.escalation_rate > 0.15 {
        score -= 0.2;
    }
    if params.escalation_rate > 0.25 {
        score -= 0.2;
    }

    score.max(0.0)
}

/// Determine if prompt should use fast-path based on tuned parameters
pub fn should_use_fast_path(prompt: &str, params: &TuningParams) -> bool {
    let word_count = prompt.split_whitespace().count();
    let score = fast_path_score(word_count, params);

    score >= params.fast_path_confidence
}

/// Select best model based on tuned scores and prompt characteristics
pub fn select_best_model(prompt: &str, params: &TuningParams) -> &'static str {
    let prompt_lower = prompt.to_lowercase();

    // Code tasks prefer Codex
    if prompt_lower.contains("code")
        || prompt_lower.contains("function")
        || prompt_lower.contains("implement")
        || prompt.contains("{")
        || prompt.contains("}")
    {
        if params.model_scores.codex >= 0.5 {
            return "codex";
        }
    }

    // Analysis/reasoning tasks prefer Claude
    if prompt_lower.contains("analyze")
        || prompt_lower.contains("explain")
        || prompt_lower.contains("compare")
        || prompt_lower.contains("review")
    {
        if params.model_scores.claude >= 0.5 {
            return "claude";
        }
    }

    // Default to highest scoring model
    if params.model_scores.gemini >= params.model_scores.claude
        && params.model_scores.gemini >= params.model_scores.codex
    {
        "gemini"
    } else if params.model_scores.claude >= params.model_scores.codex {
        "claude"
    } else {
        "codex"
    }
}

/// Update model score based on success/failure
pub fn update_model_score(params: &mut TuningParams, model: &str, success: bool, quality: f32) {
    let adjustment = if success {
        0.01 * quality
    } else {
        -0.05
    };

    match model {
        "gemini" => {
            params.model_scores.gemini = (params.model_scores.gemini + adjustment).clamp(0.0, 2.0)
        }
        "claude" => {
            params.model_scores.claude = (params.model_scores.claude + adjustment).clamp(0.0, 2.0)
        }
        "codex" => {
            params.model_scores.codex = (params.model_scores.codex + adjustment).clamp(0.0, 2.0)
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_path_score() {
        let params = TuningParams::default();

        // Short prompt should have high score
        let score = fast_path_score(5, &params);
        assert!(score > 0.9);

        // Long prompt should have lower score
        let score = fast_path_score(60, &params);
        assert!(score < 0.5);
    }

    #[test]
    fn test_select_best_model() {
        let params = TuningParams::default();

        assert_eq!(select_best_model("write a function", &params), "codex");
        assert_eq!(select_best_model("analyze this data", &params), "claude");
        assert_eq!(select_best_model("what is AI", &params), "gemini");
    }
}
