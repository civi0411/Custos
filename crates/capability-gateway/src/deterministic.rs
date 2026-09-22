use crate::traits::{ExecutionResult, ToolGate};
use async_trait::async_trait;
use custos_authority_engine::AuthorityEngine;
use custos_core_domain::{digest, Action, DomainError};
use std::sync::Arc;

pub struct DeterministicGate {
    pub authority: Arc<AuthorityEngine>,
}

impl DeterministicGate {
    pub fn new(authority: Arc<AuthorityEngine>) -> Self {
        Self { authority }
    }

    pub fn with_default_authority() -> Self {
        Self {
            authority: Arc::new(AuthorityEngine::default()),
        }
    }
}

impl Default for DeterministicGate {
    fn default() -> Self {
        Self::with_default_authority()
    }
}

#[async_trait]
impl ToolGate for DeterministicGate {
    async fn dispatch(&self, action: &Action) -> Result<ExecutionResult, DomainError> {
        self.dispatch_for_task("task_default", action, "agent_executor")
            .await
    }

    async fn dispatch_for_task(
        &self,
        task_id: &str,
        action: &Action,
        actor: &str,
    ) -> Result<ExecutionResult, DomainError> {
        // Step 1: Evaluate policy, classify risk, and issue permit via AuthorityEngine
        let permit = self
            .authority
            .authorize_action(task_id, action, actor)
            .await?;

        // Step 2: Form deterministic execution payload & evidence
        let result_json = serde_json::json!({
            "status": "executed",
            "action": action.name,
            "target": action.target,
            "permit_id": permit.id,
            "risk_class": permit.risk_class.to_string(),
        });

        let evidence_hash = digest(result_json.to_string().as_bytes());

        Ok(ExecutionResult {
            success: true,
            output: result_json,
            permit_id: Some(permit.id),
            evidence: Some(evidence_hash),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::RiskLevel;

    #[tokio::test]
    async fn test_low_risk_action_allowed_with_permit_and_evidence() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_1".into(),
            "read_file".into(),
            "README.md".into(),
            serde_json::json!({"path": "README.md"}),
            RiskLevel::Low,
        );

        let res = gate
            .dispatch_for_task("task_test_1", &action, "developer")
            .await
            .expect("Low-risk action must be authorized");

        assert!(res.success);
        assert!(res.permit_id.is_some());
        assert!(res.evidence.is_some());
        assert_eq!(res.output["status"], "executed");
    }

    #[tokio::test]
    async fn test_high_risk_action_requires_approval() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_2".into(),
            "delete_file".into(),
            "src/main.rs".into(),
            serde_json::json!({"path": "src/main.rs"}),
            RiskLevel::High,
        );

        let err = gate
            .dispatch_for_task("task_test_2", &action, "developer")
            .await
            .unwrap_err();

        match err {
            DomainError::Conflict(msg) => {
                assert!(msg.contains("requires human approval"));
            }
            other => panic!("Expected Conflict error requiring approval, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_critical_risk_action_denied() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_3".into(),
            "drop_database".into(),
            "production.db".into(),
            serde_json::json!({"force": true}),
            RiskLevel::Critical,
        );

        let err = gate
            .dispatch_for_task("task_test_3", &action, "developer")
            .await
            .unwrap_err();

        match err {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("denied"));
            }
            other => panic!("Expected Unauthorized error, got: {other:?}"),
        }
    }
}
