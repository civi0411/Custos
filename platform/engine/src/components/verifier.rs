//! Domain Pack Verifiers (RFC 003 §4 / Gate 4 Proof-Closure)
//!
//! Provides independent oracles for validating task completion before
//! closing deliberation or committing external effects.

use async_trait::async_trait;
use std::sync::Arc;

use crate::evidence::{EvidenceBundle, Verifier};
pub use custos_contracts::contributions::pack::{
    PackVerifier, VerificationContext, VerificationResult, VerifierResult,
};

/// Bridges any `PackVerifier` into the unified `crate::evidence::Verifier` system.
pub struct PackVerifierAdapter(pub Arc<dyn PackVerifier>);

#[async_trait]
impl Verifier for PackVerifierAdapter {
    fn verifier_id(&self) -> &str {
        self.0.verifier_id()
    }

    async fn verify(
        &self,
        bundle: &EvidenceBundle,
    ) -> Result<custos_contracts::VerificationClaim, custos_contracts::DomainError> {
        let ctx = VerificationContext {
            task_id: bundle.task_id.clone(),
            workspace_path: bundle
                .proof
                .get("workspace_path")
                .and_then(|v| v.as_str())
                .map(String::from),
            target_artifact: bundle
                .proof
                .get("target_artifact")
                .and_then(|v| v.as_str())
                .map(String::from),
            metadata: bundle.proof.clone(),
        };
        let result = self.0.verify(&ctx).await;
        let passed = result.is_pass();
        let details = match result {
            VerificationResult::Pass { detail } => {
                serde_json::json!({ "status": "pass", "detail": detail })
            }
            VerificationResult::Fail { reason } => {
                serde_json::json!({ "status": "fail", "reason": reason })
            }
            VerificationResult::Unknown { reason } => {
                serde_json::json!({ "status": "unknown", "reason": reason })
            }
            VerificationResult::Stale { reason } => {
                serde_json::json!({ "status": "stale", "reason": reason })
            }
        };
        Ok(custos_contracts::VerificationClaim::new(
            bundle.task_id.clone(),
            bundle.claim_statement.clone(),
            self.0.verifier_id().to_string(),
            passed,
            details,
        ))
    }
}
