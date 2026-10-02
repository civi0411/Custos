use crate::routing::RoutingSignals;
use custos_domain::RiskLevel;

pub struct SignalExtractor;

impl Default for SignalExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extracts routing signals from a user prompt and available execution context.
    pub fn extract(
        &self,
        prompt: &str,
        has_history: bool,
        budget_available: bool,
    ) -> RoutingSignals {
        let text = prompt.trim();
        let lower = text.to_lowercase();

        // 1. Deterministic Solution Check
        let deterministic = is_deterministic_query(&lower);

        // 2. Risk Level Evaluation
        let (risk, reversible) = evaluate_risk(&lower);

        // 3. Ambiguity & Uncertainty
        let ambiguity = if deterministic {
            0.0
        } else {
            compute_ambiguity(&lower, text.len())
        };
        let uncertainty = if deterministic {
            0.0
        } else {
            compute_uncertainty(&lower, has_history)
        };

        // 4. Token Estimation
        let est_input_tokens = (text.len() / 4).max(10) as u64;
        let system_one_tokens = (est_input_tokens * 3).clamp(500, 4_000);
        let system_two_tokens = (est_input_tokens * 8).clamp(2_000, 24_000);

        // 5. Probabilities & Uplift
        let (s1_prob, s2_prob) = if deterministic {
            (1.0, 1.0)
        } else if ambiguity > 0.6 || uncertainty > 0.5 {
            (0.40, 0.88)
        } else {
            (0.85, 0.95)
        };

        RoutingSignals {
            deterministic_solution_available: deterministic,
            ambiguity,
            uncertainty,
            context_sufficient: has_history || text.len() > 20,
            reversible,
            risk,
            evidence_required: matches!(risk, RiskLevel::High | RiskLevel::Critical),
            budget_available,
            system_one_success_probability: s1_prob,
            system_two_success_probability: s2_prob,
            system_one_tokens,
            system_two_tokens,
            token_cost_weight: 0.000_01,
        }
    }
}

fn is_deterministic_query(text: &str) -> bool {
    text == "ping"
        || text == "help"
        || text == "version"
        || text == "--version"
        || text.starts_with("echo ")
        || text.starts_with("what is the date")
        || text.starts_with("what time is it")
        || (text.starts_with("calculate ") && !text.contains("derivative"))
}

fn evaluate_risk(text: &str) -> (RiskLevel, bool) {
    if text.contains("rm -rf")
        || text.contains("drop table")
        || text.contains("delete from")
        || text.contains("force push")
        || text.contains("format drive")
    {
        (RiskLevel::Critical, false)
    } else if text.contains("delete ")
        || text.contains("overwrite")
        || text.contains("rebase")
        || text.contains("kill ")
        || text.contains("pkill ")
    {
        (RiskLevel::High, false)
    } else if text.contains("write ")
        || text.contains("edit ")
        || text.contains("modify ")
        || text.contains("replace ")
        || text.contains("update ")
    {
        (RiskLevel::Medium, true)
    } else {
        (RiskLevel::Low, true)
    }
}

fn compute_ambiguity(text: &str, len: usize) -> f32 {
    let mut score: f32 = 0.1;

    let ambiguous_keywords = [
        "maybe",
        "somehow",
        "perhaps",
        "bla bla",
        "tùy bạn",
        "tinh chỉnh",
        "vài cái",
        "something like",
        "or whatever",
        "do something",
    ];

    for kw in &ambiguous_keywords {
        if text.contains(kw) {
            score += 0.25;
        }
    }

    if len < 15 {
        score += 0.2;
    }

    score.clamp(0.0, 1.0)
}

fn compute_uncertainty(text: &str, has_history: bool) -> f32 {
    let mut score: f32 = 0.1;

    let complex_domains = [
        "deploy",
        "kubernetes",
        "distributed",
        "consensus",
        "p2p",
        "cryptography",
        "zero-knowledge",
        "kernel",
        "hypervisor",
        "migration",
        "concurrency",
    ];

    for domain in &complex_domains {
        if text.contains(domain) {
            score += 0.3;
        }
    }

    if !has_history {
        score += 0.15;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deterministic() {
        let extractor = SignalExtractor::new();
        let signals = extractor.extract("ping", false, true);
        assert!(signals.deterministic_solution_available);
        assert_eq!(signals.risk, RiskLevel::Low);
    }

    #[test]
    fn test_extract_high_risk() {
        let extractor = SignalExtractor::new();
        let signals = extractor.extract("rm -rf /tmp/test", true, true);
        assert_eq!(signals.risk, RiskLevel::Critical);
        assert!(!signals.reversible);
        assert!(signals.evidence_required);
    }

    #[test]
    fn test_extract_ambiguity() {
        let extractor = SignalExtractor::new();
        let signals = extractor.extract("do something somehow or whatever", false, true);
        assert!(signals.ambiguity > 0.5);
    }
}
