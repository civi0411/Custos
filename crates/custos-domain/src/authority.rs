//! Authority, Grants, and Execution Permits
//!
//! Provides cryptographically sound capability grants and ephemeral execution permits
//! governing side-effect execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskClass::Low => write!(f, "low"),
            RiskClass::Medium => write!(f, "medium"),
            RiskClass::High => write!(f, "high"),
            RiskClass::Critical => write!(f, "critical"),
        }
    }
}

/// A Grant endows an agent/session with authority to perform certain capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub id: String,
    pub task_id: String,
    pub grantee: String,
    pub capability: String,
    pub constraints: serde_json::Value,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Grant {
    pub fn new(
        task_id: String,
        grantee: String,
        capability: String,
        constraints: serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: new_id("grant"),
            task_id,
            grantee,
            capability,
            constraints,
            granted_at: Utc::now(),
            expires_at,
        }
    }

    pub fn is_valid(&self) -> bool {
        if let Some(exp) = self.expires_at {
            if Utc::now() > exp {
                return false;
            }
        }
        true
    }
}

/// An ExecutionPermit is an ephemeral token required to execute an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permit {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub capability: String,
    pub grant_id: Option<String>,
    pub risk_class: RiskClass,
    #[serde(default)]
    pub argument_digest: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub max_uses: u32,
    #[serde(default)]
    pub used_at: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Canonical alias matching AGENTS.md glossary and system specification.
pub type ExecutionPermit = Permit;
/// Version 1 Canonical Contract alias for SSOT.
pub type PermitV1 = Permit;
/// Strongly-typed or alias for Permit ID
pub type PermitId = String;

impl Permit {
    pub fn new(
        task_id: String,
        action_id: String,
        capability: String,
        grant_id: Option<String>,
        risk_class: RiskClass,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: new_id("permit"),
            task_id,
            action_id,
            capability,
            grant_id,
            risk_class,
            argument_digest: String::new(),
            idempotency_key: None,
            max_uses: 1,
            used_at: None,
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
        }
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    pub fn with_argument_digest(mut self, digest: impl Into<String>) -> Self {
        self.argument_digest = digest.into();
        self
    }

    pub fn verify_active(&self) -> Result<(), DomainError> {
        if self.used_at.is_some() {
            return Err(DomainError::Unauthorized(format!(
                "Execution permit {} has already been consumed (Single-use violation / INV-04)",
                self.id
            )));
        }
        if Utc::now() > self.expires_at {
            return Err(DomainError::Unauthorized(format!(
                "Execution permit {} has expired",
                self.id
            )));
        }
        Ok(())
    }

    pub fn consume(&mut self, expected_digest: &str) -> Result<(), DomainError> {
        self.verify_active()?;

        // If the permit has a bound argument digest, verify it matches
        if !self.argument_digest.is_empty() && self.argument_digest != expected_digest {
            return Err(DomainError::Unauthorized(format!(
                "Argument digest mismatch for permit {}: expected {}, received {} (Confused Deputy Guard / INV-04)",
                self.id, self.argument_digest, expected_digest
            )));
        }

        self.used_at = Some(Utc::now());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Success,
    Failure,
    Timeout,
    Rejected,
}

/// ExecutionReceipt proves that an action was executed under an ExecutionPermit.
/// Complies with schemas/protocol/execution-receipt.v1.schema.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub receipt_id: String,
    pub permit_id: String,
    pub action_id: String,
    pub status: ReceiptStatus,
    pub output_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub executed_at: DateTime<Utc>,
    #[serde(default)]
    pub assurance: crate::action::Assurance,
}

/// Canonical alias matching AGENTS.md glossary.
pub type Receipt = ExecutionReceipt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permit_single_use_and_argument_digest_enforcement() {
        let digest_valid = "sha256:11112222333344445555666677778888";
        let digest_tampered = "sha256:99998888777766665555444433332222";

        let mut permit = Permit::new(
            "task_01".into(),
            "act_01".into(),
            "write_file".into(),
            None,
            RiskClass::High,
            60,
        )
        .with_argument_digest(digest_valid);

        assert!(permit.verify_active().is_ok());

        // 1. Confused deputy attempt with tampered parameters must fail
        let tamper_res = permit.consume(digest_tampered);
        assert!(tamper_res.is_err());
        assert!(matches!(tamper_res.unwrap_err(), DomainError::Unauthorized(msg) if msg.contains("Argument digest mismatch")));

        // 2. Legitimate consume succeeds
        assert!(permit.consume(digest_valid).is_ok());
        assert!(permit.used_at.is_some());

        // 3. Replay attack must be blocked (Single-use violation)
        let replay_res = permit.consume(digest_valid);
        assert!(replay_res.is_err());
        assert!(matches!(replay_res.unwrap_err(), DomainError::Unauthorized(msg) if msg.contains("already been consumed")));
    }
}
