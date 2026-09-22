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
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

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
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
        }
    }

    pub fn verify_active(&self) -> Result<(), DomainError> {
        if Utc::now() > self.expires_at {
            return Err(DomainError::Unauthorized(format!(
                "Execution permit {} has expired",
                self.id
            )));
        }
        Ok(())
    }
}
