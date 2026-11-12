use custos_domain::pipeline::DataInvariant;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Assistant Workbench Invariant: Guarantees effectful actions have explicit permits before dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernedOutboxInvariant {
    pub action_id: String,
    pub intent_kind: String,
    pub permit_id: Option<String>,
    pub is_read_only: bool,
}

impl GovernedOutboxInvariant {
    pub fn new(
        action_id: impl Into<String>,
        intent_kind: impl Into<String>,
        permit_id: Option<impl Into<String>>,
        is_read_only: bool,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            intent_kind: intent_kind.into(),
            permit_id: permit_id.map(Into::into),
            is_read_only,
        }
    }
}

impl DataInvariant for GovernedOutboxInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.action_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Action ID cannot be empty".to_string(),
            ));
        }
        if !self.is_read_only && self.permit_id.as_deref().unwrap_or("").trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "Action '{}' of type '{}' is effectful and requires a valid ExecutionPermit",
                self.action_id, self.intent_kind
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governed_outbox_invariant_validation() {
        assert!(GovernedOutboxInvariant::new("act_1", "fs.read", None::<String>, true)
            .verify()
            .is_ok());
        assert!(
            GovernedOutboxInvariant::new("act_2", "fs.write", Some("permit_99"), false)
                .verify()
                .is_ok()
        );
        assert!(
            GovernedOutboxInvariant::new("act_3", "shell.exec", None::<String>, false)
                .verify()
                .is_err()
        );
    }
}

