//! Side-Effect Action Definitions & Risk Levels
//!
//! Every action that modifies the workspace or talks to the network is wrapped
//! in an Action subject to CapabilityGateway and PolicyEngine verification.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionLifecycleState {
    Intent,
    Permitted,
    Dispatching,
    Receipt,
    Uncertain,
    Failed,
}

impl ActionLifecycleState {
    pub fn can_transition_to(&self, next: ActionLifecycleState) -> bool {
        match self {
            ActionLifecycleState::Intent => matches!(
                next,
                ActionLifecycleState::Permitted | ActionLifecycleState::Failed
            ),
            ActionLifecycleState::Permitted => matches!(
                next,
                ActionLifecycleState::Dispatching | ActionLifecycleState::Failed
            ),
            ActionLifecycleState::Dispatching => matches!(
                next,
                ActionLifecycleState::Receipt
                    | ActionLifecycleState::Uncertain
                    | ActionLifecycleState::Failed
            ),
            ActionLifecycleState::Receipt => false,
            ActionLifecycleState::Uncertain => matches!(
                next,
                ActionLifecycleState::Receipt | ActionLifecycleState::Failed
            ),
            ActionLifecycleState::Failed => false,
        }
    }

    /// Core Invariant 3: Uncertain actions must never be blind retried.
    pub fn allows_blind_retry(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    pub name: String,
    pub target: String,
    pub parameters: serde_json::Value,
    pub risk_level: RiskLevel,
    pub lifecycle_state: ActionLifecycleState,
    pub evidence_required: bool,
    #[serde(default)]
    pub permit_id: Option<String>,
}

impl Action {
    pub fn new(
        id: String,
        name: String,
        target: String,
        parameters: serde_json::Value,
        risk_level: RiskLevel,
    ) -> Self {
        let evidence_required = matches!(risk_level, RiskLevel::High | RiskLevel::Critical);
        Self {
            id,
            task_id: None,
            name,
            target,
            parameters,
            risk_level,
            lifecycle_state: ActionLifecycleState::Intent,
            evidence_required,
            permit_id: None,
        }
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_permit_id(mut self, permit_id: impl Into<String>) -> Self {
        self.permit_id = Some(permit_id.into());
        self
    }

    pub fn transition(&mut self, next: ActionLifecycleState) -> Result<(), DomainError> {
        if !self.lifecycle_state.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.lifecycle_state),
                to: format!("{:?}", next),
            });
        }
        self.lifecycle_state = next;
        Ok(())
    }

    /// Computes deterministic SHA-256 digest of action parameters for Permit binding (INV-04)
    pub fn argument_digest(&self) -> String {
        let serialized = crate::ids::canonical_json(&self.parameters).unwrap_or_default();
        format!("sha256:{}", crate::ids::digest(serialized.as_bytes()))
    }
}

/// Canonical alias matching AGENTS.md glossary and system specification.
pub type ActionIntent = Action;
/// Version 1 Canonical Contract alias for SSOT.
pub type ActionIntentV1 = Action;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_lifecycle_and_no_blind_retry() {
        let mut action = Action::new(
            "act_lifecycle".into(),
            "write_file".into(),
            "src/lib.rs".into(),
            serde_json::json!({"content": "pub fn test() {}"}),
            RiskLevel::High,
        );

        assert_eq!(action.lifecycle_state, ActionLifecycleState::Intent);
        assert!(action.evidence_required);

        // Valid progression: Intent -> Permitted -> Dispatching -> Uncertain
        assert!(action.transition(ActionLifecycleState::Permitted).is_ok());
        assert!(action.transition(ActionLifecycleState::Dispatching).is_ok());
        assert!(action.transition(ActionLifecycleState::Uncertain).is_ok());

        // Invariant 3: In Uncertain state, blind retry is forbidden!
        assert!(!action.lifecycle_state.allows_blind_retry());

        // Cannot transition back to Dispatching directly without reconciliation
        assert!(action
            .transition(ActionLifecycleState::Dispatching)
            .is_err());

        // Reconciled to Receipt:
        assert!(action.transition(ActionLifecycleState::Receipt).is_ok());

        // Receipt is terminal for this action attempt
        assert!(action
            .transition(ActionLifecycleState::Dispatching)
            .is_err());
    }
}
