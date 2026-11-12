//! Port for Judgment & Verification Evaluation (RFC 005 S1 Fabric)
//!
//! Evaluates evidence, criteria, or sub-task results against calibrated thresholds.

use async_trait::async_trait;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentRequest {
    pub judgment_id: String,
    pub task_id: String,
    pub criteria: Vec<String>,
    pub candidate: serde_json::Value,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentResult {
    pub judgment_id: String,
    pub passed: bool,
    pub score: f64,
    pub confidence: f64,
    pub rationale: String,
    #[serde(default)]
    pub violations: Option<Vec<String>>,
}

#[async_trait]
pub trait JudgmentPort: Send + Sync {
    /// Evaluates execution evidence or assertions against contract criteria.
    /// Returns calibrated confidence score [0.0, 1.0].
    async fn evaluate_evidence(
        &self,
        criteria: &[String],
        payload: &str,
    ) -> Result<f32, DomainError>;

    /// Evaluates a full JudgmentRequest and produces a calibrated JudgmentResult.
    async fn judge(&self, request: JudgmentRequest) -> Result<JudgmentResult, DomainError>;
}
