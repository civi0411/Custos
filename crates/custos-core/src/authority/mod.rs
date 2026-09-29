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

use custos_core_domain::{Action, DomainError, Permit};
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
        let risk_class = RiskEvaluator::classify_action(action);

        let decision = self.policy.evaluate(task_id, action).await?;
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
                let permit = self.permits.issue_permit(
                    task_id.to_string(),
                    action.id.clone(),
                    action.name.clone(),
                    None,
                    risk_class,
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
    use custos_core_domain::RiskLevel;

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
}
