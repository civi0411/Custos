//! Custos Authority Engine
//!
//! Provides dynamic capability grants, execution permits, risk classification,
//! and human-in-the-loop approvals with cryptographic audit logging.

pub mod approvals;
pub mod audit;
pub mod grants;
pub mod permits;
pub mod policy;
pub mod risk;

pub use approvals::ApprovalManager;
pub use audit::{AuditEntry, AuditLog};
pub use grants::GrantStore;
pub use permits::PermitIssuer;
pub use policy::{DefaultPolicyEvaluator, PolicyDecision, PolicyEvaluator};
pub use risk::RiskEvaluator;

use custos_domain::{Action, DomainError, Permit};
use std::sync::Arc;

/// AuthorityEngine acts as the central authority enforcement point for tasks
#[derive(Clone)]
pub struct AuthorityEngine {
    pub grants: GrantStore,
    pub approvals: ApprovalManager,
    pub permits: PermitIssuer,
    pub audit: AuditLog,
    pub policy: Arc<dyn PolicyEvaluator>,
}

impl Default for AuthorityEngine {
    fn default() -> Self {
        Self::new(Arc::new(DefaultPolicyEvaluator))
    }
}

impl AuthorityEngine {
    pub fn new(policy: Arc<dyn PolicyEvaluator>) -> Self {
        Self {
            grants: GrantStore::new(),
            approvals: ApprovalManager::new(),
            permits: PermitIssuer::new(),
            audit: AuditLog::new(),
            policy,
        }
    }

    /// Authorizes an action: evaluates policy, checks grants, and issues an ExecutionPermit
    pub async fn authorize_action(
        &self,
        task_id: &str,
        action: &Action,
        actor: &str,
    ) -> Result<Permit, DomainError> {
        if task_id.trim().is_empty() || actor.trim().is_empty() {
            return Err(DomainError::Unauthorized(
                "Task and actor identity are required".into(),
            ));
        }
        if action
            .task_id
            .as_deref()
            .is_some_and(|bound_task| bound_task != task_id)
        {
            return Err(DomainError::Unauthorized(
                "Action belongs to a different Task".into(),
            ));
        }
        let risk_class = RiskEvaluator::classify_action(action);
        let policy_decision = self.policy.evaluate(task_id, action).await?;
        let grant = self.grants.find_grant_for_action(task_id, action, actor);
        let decision = match policy_decision {
            PolicyDecision::Deny { reason } => PolicyDecision::Deny { reason },
            PolicyDecision::RequireApproval { .. } if grant.is_some() => PolicyDecision::Allow,
            other => other,
        };

        match decision {
            PolicyDecision::Deny { reason } => {
                self.audit.record(
                    "action_denied",
                    task_id,
                    actor,
                    serde_json::json!({ "action_id": action.id, "reason": reason }),
                );
                Err(DomainError::Unauthorized(format!(
                    "Action {} denied: {}",
                    action.name, reason
                )))
            }
            PolicyDecision::RequireApproval { reason } => {
                self.audit.record(
                    "approval_required",
                    task_id,
                    actor,
                    serde_json::json!({ "action_id": action.id, "reason": reason }),
                );
                Err(DomainError::Conflict(format!(
                    "Action {} requires human approval: {}",
                    action.name, reason
                )))
            }
            PolicyDecision::Allow => {
                let digest = action.argument_digest();
                let permit = self.permits.issue_permit_with_digest(
                    task_id.to_string(),
                    action.id.clone(),
                    action.name.clone(),
                    grant.map(|grant| grant.id),
                    risk_class,
                    digest,
                    300, // 5 min TTL
                );

                self.audit.record(
                    "permit_issued",
                    task_id,
                    actor,
                    serde_json::json!({ "permit_id": permit.id, "action_id": action.id }),
                );

                Ok(permit)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{Grant, RiskLevel};

    #[tokio::test]
    async fn test_authorize_low_risk_action() {
        let engine = AuthorityEngine::default();
        let action = Action::new(
            "act_01".into(),
            "read_code".into(),
            "src/lib.rs".into(),
            serde_json::json!({}),
            RiskLevel::Low,
        );

        let permit = engine
            .authorize_action("task_01", &action, "test_runner")
            .await
            .expect("Should authorize low risk action");

        assert_eq!(permit.action_id, "act_01");
        assert_eq!(engine.audit.len(), 1);
    }

    #[tokio::test]
    async fn test_high_risk_requires_approval() {
        let engine = AuthorityEngine::default();
        let action = Action::new(
            "act_02".into(),
            "drop_database".into(),
            "db.sqlite".into(),
            serde_json::json!({}),
            RiskLevel::Critical,
        );

        let res = engine
            .authorize_action("task_01", &action, "test_runner")
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_critical_action_cannot_bypass_policy_via_grant() {
        let engine = AuthorityEngine::default();
        let grant = Grant::new(
            "task_01".into(),
            "actor_a".into(),
            "drop_database".into(),
            serde_json::json!({}),
            None,
        );
        engine.grants.insert(grant);

        let action = Action::new(
            "act_crit".into(),
            "drop_database".into(),
            "db.sqlite".into(),
            serde_json::json!({}),
            RiskLevel::Critical,
        );

        let res = engine.authorize_action("task_01", &action, "actor_a").await;
        assert!(
            matches!(res, Err(DomainError::Unauthorized(_))),
            "Critical operations must remain hard denied even with grant"
        );
    }

    #[tokio::test]
    async fn test_grant_isolation_by_actor() {
        let engine = AuthorityEngine::default();
        let grant = Grant::new(
            "task_01".into(),
            "actor_a".into(),
            "write_code".into(),
            serde_json::json!({}),
            None,
        );
        engine.grants.insert(grant);

        let action = Action::new(
            "act_write".into(),
            "write_code".into(),
            "src/lib.rs".into(),
            serde_json::json!({}),
            RiskLevel::High,
        );

        // actor_b cannot use grant granted to actor_a
        let res_b = engine.authorize_action("task_01", &action, "actor_b").await;
        assert!(
            matches!(res_b, Err(DomainError::Conflict(_))),
            "actor_b must not inherit actor_a grant"
        );

        // actor_a CAN use the grant
        let res_a = engine.authorize_action("task_01", &action, "actor_a").await;
        assert!(res_a.is_ok(), "actor_a should be permitted via grant");
    }

    #[tokio::test]
    async fn grants_require_exact_targets_valid_constraints_and_bound_payloads() {
        let action = Action::new(
            "act_bound".into(),
            "write_code".into(),
            "src/main.rs".into(),
            serde_json::json!({"content": "accepted"}),
            RiskLevel::High,
        )
        .with_task_id("task_01");
        let invalid_constraints = [
            serde_json::json!({"target": "src/main"}),
            serde_json::json!({"target": "src"}),
            serde_json::json!({"target": 5}),
            serde_json::json!({"allowed_targets": ["src/main.rs", 5]}),
            serde_json::json!({"max_risk": "unrecognized"}),
            serde_json::json!({"max_risk": "medium"}),
            serde_json::json!({"argument_digest": "changed"}),
            serde_json::json!({"status": "revoked"}),
            serde_json::json!({"unsupported_scope": "anything"}),
            serde_json::Value::Null,
        ];
        for constraints in invalid_constraints {
            let engine = AuthorityEngine::default();
            engine.grants.insert(Grant::new(
                "task_01".into(),
                "actor".into(),
                "write_code".into(),
                constraints.clone(),
                None,
            ));
            assert!(
                matches!(
                    engine.authorize_action("task_01", &action, "actor").await,
                    Err(DomainError::Conflict(_))
                ),
                "constraint unexpectedly authorized: {constraints}"
            );
        }

        let engine = AuthorityEngine::default();
        engine.grants.insert(Grant::new(
            "task_01".into(),
            "actor".into(),
            "write_code".into(),
            serde_json::json!({"target": "other.rs"}),
            None,
        ));
        let valid = Grant::new(
            "task_01".into(),
            "actor".into(),
            "write_code".into(),
            serde_json::json!({
                "target": "src/main.rs", "max_risk": "high",
                "argument_digest": action.argument_digest()
            }),
            None,
        );
        engine.grants.insert(valid.clone());
        let permit = engine
            .authorize_action("task_01", &action, "actor")
            .await
            .unwrap();
        assert_eq!(permit.grant_id, Some(valid.id));
        let mut changed = action.clone();
        changed.parameters["content"] = serde_json::json!("tampered");
        assert!(matches!(
            engine.authorize_action("task_01", &changed, "actor").await,
            Err(DomainError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn empty_actor_and_conflicting_task_identity_are_rejected() {
        let engine = AuthorityEngine::default();
        engine.grants.insert(Grant::new(
            "task_01".into(),
            "*".into(),
            "read_code".into(),
            serde_json::json!({}),
            None,
        ));
        let action = Action::new(
            "act_read".into(),
            "read_code".into(),
            "src/main.rs".into(),
            serde_json::json!({}),
            RiskLevel::Low,
        )
        .with_task_id("task_01");
        assert!(engine
            .grants
            .find_grant_for_action("task_01", &action, "")
            .is_none());
        assert!(matches!(
            engine.authorize_action("task_01", &action, " ").await,
            Err(DomainError::Unauthorized(_))
        ));
        assert!(matches!(
            engine
                .authorize_action("different_task", &action, "actor")
                .await,
            Err(DomainError::Unauthorized(_))
        ));
    }
}
