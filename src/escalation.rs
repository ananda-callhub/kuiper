/// Patterns that indicate the model is uncertain and should escalate
const UNCERTAINTY_PATTERNS: &[&str] = &[
    "I'm not sure",
    "I cannot guarantee",
    "I don't have enough information",
    "I'm unable to",
    "I cannot",
    "unclear",
    "ambiguous",
    "need more context",
    "please clarify",
    "it depends",
];

/// Patterns that indicate a potential error in the response
const ERROR_PATTERNS: &[&str] = &[
    "error",
    "failed",
    "exception",
    "invalid",
    "cannot process",
];

/// Check if a model response should trigger escalation to full orchestration
pub fn should_escalate(output: &str) -> bool {
    let output_lower = output.to_lowercase();

    // Check for uncertainty patterns
    if UNCERTAINTY_PATTERNS
        .iter()
        .any(|p| output_lower.contains(&p.to_lowercase()))
    {
        return true;
    }

    // Check for very short responses (might indicate failure)
    if output.trim().len() < 20 {
        return true;
    }

    false
}

/// Check if the response contains an error
pub fn contains_error(output: &str) -> bool {
    let output_lower = output.to_lowercase();
    ERROR_PATTERNS
        .iter()
        .any(|p| output_lower.contains(&p.to_lowercase()))
}

/// Determine escalation strategy based on the output
#[derive(Debug, PartialEq)]
pub enum EscalationStrategy {
    /// No escalation needed
    None,
    /// Retry with a different model
    RetryDifferentModel,
    /// Escalate to full orchestration
    FullOrchestration,
    /// Ask user for clarification
    UserClarification,
}

pub fn determine_strategy(output: &str, attempt: u32) -> EscalationStrategy {
    if !should_escalate(output) {
        return EscalationStrategy::None;
    }

    match attempt {
        0 => EscalationStrategy::RetryDifferentModel,
        1 => EscalationStrategy::FullOrchestration,
        _ => EscalationStrategy::UserClarification,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_escalate() {
        assert!(should_escalate("I'm not sure about this."));
        assert!(should_escalate("I cannot guarantee the result."));
        assert!(!should_escalate("Here is a complete and detailed response to your question."));
    }

    #[test]
    fn test_short_response_escalates() {
        assert!(should_escalate("OK"));
        assert!(should_escalate("Done."));
    }

    #[test]
    fn test_escalation_strategy() {
        let uncertain = "I'm not sure about this.";
        assert_eq!(
            determine_strategy(uncertain, 0),
            EscalationStrategy::RetryDifferentModel
        );
        assert_eq!(
            determine_strategy(uncertain, 1),
            EscalationStrategy::FullOrchestration
        );
    }
}
