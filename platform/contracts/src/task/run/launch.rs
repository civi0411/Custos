use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchStatus {
    Prepared,
    Launched,
    Refused,
    Failed,
    Unknown,
}

impl LaunchStatus {
    pub fn allows_retry(self) -> bool {
        matches!(self, Self::Refused | Self::Failed)
    }
}

impl std::fmt::Display for LaunchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared => write!(f, "prepared"),
            Self::Launched => write!(f, "launched"),
            Self::Refused => write!(f, "refused"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// One attempt to start a concrete model or native harness execution.
/// Unknown launch results must be reconciled before another launch is attempted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchAttempt {
    pub id: String,
    pub worker_run_id: String,
    pub requested_mode: String,
    pub actual_mode: Option<String>,
    pub harness_id: Option<String>,
    pub status: LaunchStatus,
    pub detail: Option<String>,
    pub prepared_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl LaunchAttempt {
    pub fn new(
        worker_run_id: impl Into<String>,
        requested_mode: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let worker_run_id = worker_run_id.into();
        let requested_mode = requested_mode.into();
        if worker_run_id.trim().is_empty() || requested_mode.trim().is_empty() {
            return Err(DomainError::Validation(
                "Launch attempt requires worker run and requested mode".into(),
            ));
        }
        Ok(Self {
            id: new_id("launch"),
            worker_run_id,
            requested_mode,
            actual_mode: None,
            harness_id: None,
            status: LaunchStatus::Prepared,
            detail: None,
            prepared_at: Utc::now(),
            resolved_at: None,
        })
    }

    pub fn mark_launched(
        &mut self,
        actual_mode: impl Into<String>,
        harness_id: Option<String>,
    ) -> Result<(), DomainError> {
        self.ensure_prepared(LaunchStatus::Launched)?;
        let actual_mode = actual_mode.into();
        if actual_mode.trim().is_empty() {
            return Err(DomainError::Validation(
                "Launched attempt requires the actual execution mode".into(),
            ));
        }
        self.actual_mode = Some(actual_mode);
        self.harness_id = harness_id;
        self.status = LaunchStatus::Launched;
        self.resolved_at = Some(Utc::now());
        Ok(())
    }

    pub fn mark_refused(&mut self, detail: impl Into<String>) -> Result<(), DomainError> {
        self.resolve_without_launch(LaunchStatus::Refused, detail)
    }

    pub fn mark_failed(&mut self, detail: impl Into<String>) -> Result<(), DomainError> {
        self.resolve_without_launch(LaunchStatus::Failed, detail)
    }

    pub fn mark_unknown(&mut self, detail: impl Into<String>) -> Result<(), DomainError> {
        self.resolve_without_launch(LaunchStatus::Unknown, detail)
    }

    fn resolve_without_launch(
        &mut self,
        status: LaunchStatus,
        detail: impl Into<String>,
    ) -> Result<(), DomainError> {
        self.ensure_prepared(status)?;
        let detail = detail.into();
        if detail.trim().is_empty() {
            return Err(DomainError::Validation(
                "Resolved launch attempt requires detail".into(),
            ));
        }
        self.status = status;
        self.detail = Some(detail);
        self.resolved_at = Some(Utc::now());
        Ok(())
    }

    fn ensure_prepared(&self, next: LaunchStatus) -> Result<(), DomainError> {
        if self.status != LaunchStatus::Prepared {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status).to_lowercase(),
                to: format!("{next:?}").to_lowercase(),
            });
        }
        Ok(())
    }
}
