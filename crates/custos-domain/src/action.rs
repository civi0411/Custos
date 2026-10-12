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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Assurance {
    CustosMediated,
    ProviderGoverned,
    ObserveOnly,
    #[default]
    Unknown,
}

impl Assurance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CustosMediated => "custos-mediated",
            Self::ProviderGoverned => "provider-governed",
            Self::ObserveOnly => "observe-only",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Assurance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Assurance {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "custos-mediated" => Self::CustosMediated,
            "provider-governed" => Self::ProviderGoverned,
            "observe-only" => Self::ObserveOnly,
            _ => Self::Unknown,
        })
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
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub assurance: Assurance,
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
        let assurance = parameters
            .get("assurance")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<Assurance>().unwrap_or_default())
            .unwrap_or_default();

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
            idempotency_key: None,
            assurance,
        }
    }

    pub fn with_assurance(mut self, assurance: Assurance) -> Self {
        self.assurance = assurance;
        if let Some(parameters) = self.parameters.as_object_mut() {
            parameters.insert(
                "assurance".into(),
                serde_json::Value::String(assurance.as_str().into()),
            );
        }
        self
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_permit_id(mut self, permit_id: impl Into<String>) -> Self {
        self.permit_id = Some(permit_id.into());
        self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Pending,
    InFlight,
    Succeeded,
    Failed,
    Uncertain,
}

impl EffectStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }

    pub fn can_transition_to(&self, next: EffectStatus) -> bool {
        match self {
            EffectStatus::Pending => matches!(next, EffectStatus::InFlight | EffectStatus::Failed),
            EffectStatus::InFlight => matches!(
                next,
                EffectStatus::Succeeded | EffectStatus::Failed | EffectStatus::Uncertain
            ),
            EffectStatus::Uncertain => {
                matches!(next, EffectStatus::Succeeded | EffectStatus::Failed)
            }
            EffectStatus::Succeeded | EffectStatus::Failed => false,
        }
    }
}

/// Layer 5 in Execution Lifecycle (RFC 001): Durable Effect Attempt bound to a Permit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectAttempt {
    pub id: String,
    pub node_attempt_id: String,
    pub permit_id: String,
    pub idempotency_key: String,
    pub status: EffectStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub receipt: Option<crate::authority::ExecutionReceipt>,
}

impl EffectAttempt {
    pub fn new(
        node_attempt_id: impl Into<String>,
        permit_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            id: crate::ids::new_id("eff_att"),
            node_attempt_id: node_attempt_id.into(),
            permit_id: permit_id.into(),
            idempotency_key: idempotency_key.into(),
            status: EffectStatus::Pending,
            started_at: chrono::Utc::now(),
            ended_at: None,
            receipt: None,
        }
    }

    pub fn mark_in_flight(&mut self) -> Result<(), DomainError> {
        if !self.status.can_transition_to(EffectStatus::InFlight) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", EffectStatus::InFlight),
            });
        }
        self.status = EffectStatus::InFlight;
        Ok(())
    }

    pub fn mark_uncertain(&mut self) -> Result<(), DomainError> {
        if !self.status.can_transition_to(EffectStatus::Uncertain) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", EffectStatus::Uncertain),
            });
        }
        self.status = EffectStatus::Uncertain;
        Ok(())
    }

    pub fn succeed(
        &mut self,
        receipt: crate::authority::ExecutionReceipt,
    ) -> Result<(), DomainError> {
        if !self.status.can_transition_to(EffectStatus::Succeeded) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", EffectStatus::Succeeded),
            });
        }
        self.status = EffectStatus::Succeeded;
        self.ended_at = Some(chrono::Utc::now());
        self.receipt = Some(receipt);
        Ok(())
    }

    pub fn fail(
        &mut self,
        receipt: Option<crate::authority::ExecutionReceipt>,
    ) -> Result<(), DomainError> {
        if !self.status.can_transition_to(EffectStatus::Failed) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", EffectStatus::Failed),
            });
        }
        self.status = EffectStatus::Failed;
        self.ended_at = Some(chrono::Utc::now());
        self.receipt = receipt;
        Ok(())
    }
}

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

    #[test]
    fn assurance_update_accepts_non_object_parameters_without_panicking() {
        let action = Action::new(
            "act_scalar".into(),
            "opaque_action".into(),
            "opaque_target".into(),
            serde_json::json!("opaque payload"),
            RiskLevel::Low,
        )
        .with_assurance(Assurance::ObserveOnly);

        assert_eq!(action.assurance, Assurance::ObserveOnly);
        assert_eq!(action.parameters, serde_json::json!("opaque payload"));
    }
}
