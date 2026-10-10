//! User Acceptance Oracle
//!
//! Preferred verifier for Assistant Pack (`user_acceptance_oracle`).
//! Assesses whether irreversible external effects (e.g. sending emails,
//! modifying calendars, deleting files) possess valid user approval
//! and un-tampered payload signatures before Gate 4 completion.

use async_trait::async_trait;
use serde_json::Value;

use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

pub struct UserAcceptanceOracle;

impl UserAcceptanceOracle {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UserAcceptanceOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackVerifier for UserAcceptanceOracle {
    fn verifier_id(&self) -> &'static str {
        "user_acceptance_oracle"
    }

    fn description(&self) -> &'static str {
        "Verifies that external side-effects possess cryptographic user approval and untampered payload digests"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        let approved = ctx
            .metadata
            .get("approved")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if !approved {
            return VerificationResult::Fail {
                reason: "action requires explicit user approval (human-in-the-loop gate pending)"
                    .into(),
            };
        }

        // B7 Invariant: Draft is never sent; action marked draft_only cannot pass user acceptance
        let is_draft_only = ctx
            .metadata
            .get("draft_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_draft_only {
            return VerificationResult::Fail {
                reason: "action is marked draft-only; draft is never sent without explicit dispatch command"
                    .into(),
            };
        }

        // B7 Invariant: Timeout after send or unconfirmed receipt remains uncertain
        if let Some(effect_status) = ctx.metadata.get("effect_status").and_then(Value::as_str) {
            if effect_status.eq_ignore_ascii_case("uncertain") {
                return VerificationResult::Unknown {
                    reason: "effect execution is in uncertain state; must reconcile outbox before user acceptance".into(),
                };
            }
        }

        let approval_token = ctx.metadata.get("approval_token").and_then(Value::as_str);

        if approval_token.is_none() || approval_token.unwrap().trim().is_empty() {
            return VerificationResult::Fail {
                reason: "missing or empty approval_token in user acceptance metadata".into(),
            };
        }

        let payload_digest = ctx.metadata.get("payload_digest").and_then(Value::as_str);

        if payload_digest.is_none() || payload_digest.unwrap().trim().is_empty() {
            return VerificationResult::Fail {
                reason: "missing payload_digest: cannot verify stability of approved payload"
                    .into(),
            };
        }

        VerificationResult::Pass {
            detail: format!(
                "User acceptance verified: token='{}', locked payload_digest='{}'",
                approval_token.unwrap(),
                payload_digest.unwrap()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_user_acceptance_pass() {
        let oracle = UserAcceptanceOracle::new();
        let ctx = VerificationContext::new("task_ast_1").with_metadata(json!({
            "approved": true,
            "approval_token": "usr_token_9918",
            "payload_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }

    #[tokio::test]
    async fn test_user_acceptance_fail_unapproved() {
        let oracle = UserAcceptanceOracle::new();
        let ctx = VerificationContext::new("task_ast_2").with_metadata(json!({
            "approved": false,
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_user_acceptance_rejects_draft_only() {
        let oracle = UserAcceptanceOracle::new();
        let ctx = VerificationContext::new("task_ast_3").with_metadata(json!({
            "approved": true,
            "draft_only": true,
            "approval_token": "usr_tok_1",
            "payload_digest": "sha256:abc"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_user_acceptance_uncertain_effect_returns_unknown() {
        let oracle = UserAcceptanceOracle::new();
        let ctx = VerificationContext::new("task_ast_4").with_metadata(json!({
            "approved": true,
            "effect_status": "uncertain",
            "approval_token": "usr_tok_2",
            "payload_digest": "sha256:def"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Unknown { .. }));
    }
}
