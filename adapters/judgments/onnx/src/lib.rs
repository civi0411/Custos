//! Local ONNX fast embedding & scoring

use async_trait::async_trait;
use custos_core_domain::DomainError;
use custos_judgment_contracts::{FastJudgment, JudgmentEngine};

pub struct OnnxJudgmentEngine;

impl OnnxJudgmentEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OnnxJudgmentEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JudgmentEngine for OnnxJudgmentEngine {
    async fn judge(&self, prompt: &str) -> Result<FastJudgment, DomainError> {
        Ok(FastJudgment {
            confidence: 0.95,
            route: "fast-path".into(),
            reason: format!(
                "Processed by OnnxJudgmentEngine for length {}",
                prompt.len()
            ),
        })
    }
}
