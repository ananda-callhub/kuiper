/// Calculate confidence score for fast-path routing
///
/// Returns a value between 0.0 and 1.0, where higher values
/// indicate greater confidence that fast-path is appropriate.
pub fn fast_path_confidence(word_count: usize, prompt_lower: &str) -> f32 {
    let mut score = 1.0_f32;

    // Longer prompts are more likely to need orchestration
    if word_count > 10 {
        score -= 0.1;
    }
    if word_count > 20 {
        score -= 0.2;
    }
    if word_count > 30 {
        score -= 0.2;
    }

    // Question words often indicate need for reasoning
    let question_words = ["why", "how", "explain", "describe"];
    if question_words.iter().any(|w| prompt_lower.starts_with(w)) {
        score -= 0.1;
    }

    // Multiple sentences suggest complexity
    let sentence_count = prompt_lower.matches('.').count()
        + prompt_lower.matches('?').count()
        + prompt_lower.matches('!').count();
    if sentence_count > 1 {
        score -= 0.15 * (sentence_count - 1) as f32;
    }

    // Lists suggest multi-step tasks
    if prompt_lower.contains(" and ") || prompt_lower.contains(" then ") {
        score -= 0.2;
    }

    score.max(0.0)
}

/// Adjust fast-path threshold based on recent escalation rate
pub fn adjusted_threshold(base_threshold: f32, recent_escalation_rate: f32) -> f32 {
    // If we're escalating too often, be more conservative
    if recent_escalation_rate > 0.2 {
        base_threshold * 0.8
    } else if recent_escalation_rate < 0.05 {
        // If we rarely escalate, we can be more aggressive
        base_threshold * 1.1
    } else {
        base_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_scoring() {
        // Short prompt should have high confidence
        let score = fast_path_confidence(3, "fix bug");
        assert!(score > 0.8);

        // Long complex prompt should have low confidence
        let score = fast_path_confidence(50, "explain how to design and implement a system");
        assert!(score < 0.5);
    }

    #[test]
    fn test_threshold_adjustment() {
        let base = 0.85;

        // High escalation rate should lower threshold
        assert!(adjusted_threshold(base, 0.3) < base);

        // Low escalation rate should raise threshold
        assert!(adjusted_threshold(base, 0.02) > base);
    }
}
