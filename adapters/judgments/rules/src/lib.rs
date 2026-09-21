//! Deterministic Rule-based Judgment

use async_trait::async_trait;
use custos_core_domain::DomainError;
use custos_judgment_contracts::{FastJudgment, JudgmentEngine};

pub struct RuleJudgmentEngine;

impl RuleJudgmentEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RuleJudgmentEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JudgmentEngine for RuleJudgmentEngine {
    async fn judge(&self, prompt: &str) -> Result<FastJudgment, DomainError> {
        Ok(FastJudgment {
            confidence: 0.95,
            route: "fast-path".into(),
            reason: format!(
                "Processed by RuleJudgmentEngine for length {}",
                prompt.len()
            ),
        })
    }
}
