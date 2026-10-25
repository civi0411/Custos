//! User Acceptance Oracle
//!
//! Preferred verifier for Assistant Pack (`user_acceptance_oracle`).
//! Assesses whether irreversible external effects (e.g. sending emails,
//! modifying calendars, deleting files) possess valid user approval
//! and un-tampered payload signatures before Gate 4 completion.

use std::sync::Arc;
use async_trait::async_trait;
use custos_core::authority::GrantStore;
use serde_json::Value;

use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

#[derive(Clone, Default)]
pub struct UserAcceptanceOracle {
    grant_store: Option<Arc<GrantStore>>,
}

impl UserAcceptanceOracle {
    pub fn new() -> Self {
        Self { grant_store: None }
    }

    pub fn with_grant_store(grant_store: Arc<GrantStore>) -> Self {
        Self {
            grant_store: Some(grant_store),
        }
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

        let approval_id = ctx
            .metadata
            .get("approval_id")
            .or_else(|| ctx.metadata.get("approval_token"))
            .and_then(Value::as_str);

        let approval_id = match approval_id {
            Some(id) if !id.trim().is_empty() => id.trim(),
            _ => {
                return VerificationResult::Fail {
                    reason: "no trusted approval record (missing approval_id/token)".into(),
                };
            }
        };

        let payload_digest = match ctx.metadata.get("payload_digest").and_then(Value::as_str) {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => {
                return VerificationResult::Fail {
                    reason: "missing payload_digest: cannot verify stability of approved payload"
                        .into(),
                };
            }
        };

        // B4/F12 Invariant: Self-asserted approved: bool or approval_token is NEVER trusted.
        // Must resolve against authoritative GrantStore.
        let grant_store = match &self.grant_store {
            Some(store) => store,
            None => {
                return VerificationResult::Fail {
                    reason: "no trusted approval record".into(),
                };
            }
        };

        let grant = match grant_store.get(approval_id) {
            Some(g) => g,
            None => {
                return VerificationResult::Fail {
                    reason: "no trusted approval record".into(),
                };
            }
        };

        if !grant.is_valid() {
            return VerificationResult::Fail {
                reason: "no trusted approval record (grant expired)".into(),
            };
        }

        let status = grant
            .constraints
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("granted");

        if !status.eq_ignore_ascii_case("granted") && !status.eq_ignore_ascii_case("approved") {
            return VerificationResult::Fail {
                reason: format!("approval record status is '{}', not granted", status),
            };
        }

        let record_digest = grant
            .constraints
            .get("payload_digest")
            .or_else(|| grant.constraints.get("argument_digest"))
            .and_then(Value::as_str);

        match record_digest {
            Some(expected_digest) if expected_digest == payload_digest => {
                VerificationResult::Pass {
                    detail: format!(
                        "User acceptance verified: approval_id='{}', locked payload_digest='{}'",
                        approval_id, payload_digest
                    ),
                }
            }
            Some(expected_digest) => {
                VerificationResult::Fail {
                    reason: format!(
                        "payload digest mismatch with trusted approval record: expected '{}', got '{}'",
                        expected_digest, payload_digest
                    ),
                }
            }
            None => {
                VerificationResult::Fail {
                    reason: "no trusted approval record (missing payload_digest in grant constraints)".into(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::Grant;
    use serde_json::json;

    #[tokio::test]
    async fn test_user_acceptance_pass_with_trusted_grant() {
        let store = Arc::new(GrantStore::new());
        let mut grant = Grant::new(
            "task_ast_1".into(),
            "user".into(),
            "send_email".into(),
            json!({
                "status": "granted",
                "payload_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }),
            None,
        );
        grant.id = "usr_token_9918".into();
        store.insert(grant);

        let oracle = UserAcceptanceOracle::with_grant_store(store);
        let ctx = VerificationContext::new("task_ast_1").with_metadata(json!({
            "approved": true,
            "approval_token": "usr_token_9918",
            "payload_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }

    #[tokio::test]
    async fn test_user_acceptance_rejects_caller_asserted_approval_without_grant_store_record() {
        // Gate B4 requirement: approved=true + token bat ky khong co GrantStore record -> Rejected
        let oracle = UserAcceptanceOracle::new(); // No grant store attached
        let ctx = VerificationContext::new("task_ast_forged").with_metadata(json!({
            "approved": true,
            "approval_token": "forged_token_123",
            "payload_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { ref reason } if reason.contains("no trusted approval record")));
    }

    #[tokio::test]
    async fn test_user_acceptance_rejects_payload_digest_mismatch() {
        let store = Arc::new(GrantStore::new());
        let mut grant = Grant::new(
            "task_ast_1".into(),
            "user".into(),
            "send_email".into(),
            json!({
                "status": "granted",
                "payload_digest": "expected_digest_abc"
            }),
            None,
        );
        grant.id = "valid_token".into();
        store.insert(grant);

        let oracle = UserAcceptanceOracle::with_grant_store(store);
        let ctx = VerificationContext::new("task_ast_1").with_metadata(json!({
            "approval_id": "valid_token",
            "payload_digest": "tampered_digest_xyz"
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { ref reason } if reason.contains("payload digest mismatch")));
    }

    #[tokio::test]
    async fn test_user_acceptance_rejects_draft_only() {
        let store = Arc::new(GrantStore::new());
        let mut grant = Grant::new(
            "task_ast_3".into(),
            "user".into(),
            "send_email".into(),
            json!({
                "status": "granted",
                "payload_digest": "sha256:abc"
            }),
            None,
        );
        grant.id = "usr_tok_1".into();
        store.insert(grant);

        let oracle = UserAcceptanceOracle::with_grant_store(store);
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
        let store = Arc::new(GrantStore::new());
        let mut grant = Grant::new(
            "task_ast_4".into(),
            "user".into(),
            "send_email".into(),
            json!({
                "status": "granted",
                "payload_digest": "sha256:def"
            }),
            None,
        );
        grant.id = "usr_tok_2".into();
        store.insert(grant);

        let oracle = UserAcceptanceOracle::with_grant_store(store);
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
