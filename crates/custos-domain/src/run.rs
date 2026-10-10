//! Task Run Entity & Execution Sessions
//!
//! A Run represents an active execution attempt or session for a Task.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Active,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Pending => write!(f, "pending"),
            RunStatus::Active => write!(f, "active"),
            RunStatus::Suspended => write!(f, "suspended"),
            RunStatus::Completed => write!(f, "completed"),
            RunStatus::Failed => write!(f, "failed"),
            RunStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(&self, next: RunStatus) -> bool {
        match self {
            RunStatus::Pending => matches!(next, RunStatus::Active | RunStatus::Cancelled),
            RunStatus::Active => matches!(
                next,
                RunStatus::Suspended
                    | RunStatus::Completed
                    | RunStatus::Failed
                    | RunStatus::Cancelled
            ),
            RunStatus::Suspended => matches!(next, RunStatus::Active | RunStatus::Cancelled),
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub task_id: String,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    pub status: RunStatus,
    pub attempt: u32,
    pub current_span_num: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl Run {
    pub fn new(task_id: String, attempt: u32) -> Self {
        Self {
            id: new_id("run"),
            task_id,
            workflow_revision: None,
            status: RunStatus::Pending,
            attempt,
            current_span_num: 0,
            started_at: Utc::now(),
            ended_at: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_workflow_revision(mut self, revision: impl Into<String>) -> Self {
        self.workflow_revision = Some(revision.into());
        self
    }

    pub fn transition(&mut self, next: RunStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        if next.is_terminal() {
            self.ended_at = Some(Utc::now());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRun {
    pub id: String,
    #[serde(default)]
    pub run_id: Option<String>,
    pub task_id: String,
    pub worker_id: String,
    pub attempt_id: u32,
    pub max_attempts: u32,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub continuation_packet_ref: Option<String>,
}

impl WorkerRun {
    pub fn new(task_id: String, worker_id: String, max_attempts: u32) -> Self {
        Self {
            id: new_id("wrun"),
            run_id: None,
            task_id,
            worker_id,
            attempt_id: 1,
            max_attempts,
            status: RunStatus::Pending,
            started_at: Utc::now(),
            ended_at: None,
            continuation_packet_ref: None,
        }
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn can_retry(&self) -> bool {
        self.attempt_id < self.max_attempts
    }

    pub fn record_attempt(&mut self) -> Result<u32, DomainError> {
        if !self.can_retry() {
            return Err(DomainError::Validation(format!(
                "Max attempts ({}) reached for worker run {}",
                self.max_attempts, self.id
            )));
        }
        self.attempt_id += 1;
        self.status = RunStatus::Active;
        Ok(self.attempt_id)
    }

    pub fn transition(&mut self, next: RunStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        if next.is_terminal() {
            self.ended_at = Some(Utc::now());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttempt {
    pub id: String,
    pub worker_run_id: String,
    pub attempt_number: u32,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub exit_reason: Option<String>,
}

impl NodeAttempt {
    pub fn new(worker_run_id: String, attempt_number: u32) -> Self {
        Self {
            id: new_id("node_att"),
            worker_run_id,
            attempt_number,
            status: RunStatus::Active,
            started_at: Utc::now(),
            ended_at: None,
            exit_reason: None,
        }
    }

    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.ended_at = Some(Utc::now());
    }

    pub fn fail(&mut self, reason: String) {
        self.status = RunStatus::Failed;
        self.ended_at = Some(Utc::now());
        self.exit_reason = Some(reason);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Pending,
    Dispatched,
    Released,
    Expired,
}

impl std::fmt::Display for ClaimStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimStatus::Pending => write!(f, "pending"),
            ClaimStatus::Dispatched => write!(f, "dispatched"),
            ClaimStatus::Released => write!(f, "released"),
            ClaimStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Sovereign dispatch claim for atomic task/node dispatch (Orca fencing parity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchClaim {
    pub id: String,
    pub task_id: String,
    pub node_id: Option<String>,
    pub assignee: String,
    pub depth: u32,
    pub status: ClaimStatus,
    pub claimed_at: DateTime<Utc>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub worker_run_id: Option<String>,
}

impl DispatchClaim {
    pub fn new(task_id: impl Into<String>, assignee: impl Into<String>, depth: u32) -> Self {
        Self {
            id: new_id("claim"),
            task_id: task_id.into(),
            node_id: None,
            assignee: assignee.into(),
            depth,
            status: ClaimStatus::Pending,
            claimed_at: Utc::now(),
            dispatched_at: None,
            released_at: None,
            worker_run_id: None,
        }
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    pub fn mark_dispatched(&mut self, worker_run_id: impl Into<String>) -> Result<(), DomainError> {
        if self.status != ClaimStatus::Pending {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: "dispatched".into(),
            });
        }
        self.status = ClaimStatus::Dispatched;
        self.worker_run_id = Some(worker_run_id.into());
        self.dispatched_at = Some(Utc::now());
        Ok(())
    }

    pub fn release(&mut self) -> Result<(), DomainError> {
        if self.status == ClaimStatus::Released {
            return Ok(());
        }
        self.status = ClaimStatus::Released;
        self.released_at = Some(Utc::now());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunCommand {
    pub task_id: String,
    pub actor: String,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    #[serde(default)]
    pub preferred_mode: Option<String>,
    #[serde(default)]
    pub harness_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl StartRunCommand {
    pub fn new(task_id: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            actor: actor.into(),
            workflow_revision: None,
            preferred_mode: None,
            harness_id: None,
            workspace_root: None,
            session_id: None,
            turn_id: None,
            prompt: None,
            model: None,
        }
    }

    pub fn with_workflow_revision(mut self, revision: impl Into<String>) -> Self {
        self.workflow_revision = Some(revision.into());
        self
    }

    pub fn with_preferred_mode(mut self, mode: impl Into<String>) -> Self {
        self.preferred_mode = Some(mode.into());
        self
    }

    pub fn with_harness_id(mut self, harness_id: impl Into<String>) -> Self {
        self.harness_id = Some(harness_id.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHandle {
    pub run_id: String,
    pub task_id: String,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RunHandle {
    pub fn new(
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        status: RunStatus,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            status,
            started_at,
            completed_at: None,
            output: None,
        }
    }

    pub fn with_completion(mut self, completed_at: Option<DateTime<Utc>>, output: Option<String>) -> Self {
        self.completed_at = completed_at;
        self.output = output;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelReceipt {
    pub run_id: String,
    pub cancelled_at: DateTime<Utc>,
    pub reason: String,
    pub uncertain_effects_count: u32,
}

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

/// A measurement never collapses missing telemetry into a zero value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "certainty", content = "value")]
pub enum UsageMeasurement {
    Known(u64),
    Estimated(u64),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageExecutorKind {
    Model,
    NativeHarness,
    Tool,
    Compute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub task_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub executor_kind: UsageExecutorKind,
    pub executor_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: UsageMeasurement,
    pub output_tokens: UsageMeasurement,
    pub cost_micros: UsageMeasurement,
    pub wall_time_ms: UsageMeasurement,
    pub recorded_at: DateTime<Utc>,
}

impl UsageRecord {
    pub fn new(
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        attempt_id: impl Into<String>,
        executor_kind: UsageExecutorKind,
        executor_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let task_id = task_id.into();
        let run_id = run_id.into();
        let attempt_id = attempt_id.into();
        let executor_id = executor_id.into();
        if [
            task_id.as_str(),
            run_id.as_str(),
            attempt_id.as_str(),
            executor_id.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(DomainError::Validation(
                "Usage record requires task, run, attempt, and executor identity".into(),
            ));
        }

        Ok(Self {
            id: new_id("usage"),
            task_id,
            run_id,
            attempt_id,
            executor_kind,
            executor_id,
            provider: None,
            model: None,
            input_tokens: UsageMeasurement::Unknown,
            output_tokens: UsageMeasurement::Unknown,
            cost_micros: UsageMeasurement::Unknown,
            wall_time_ms: UsageMeasurement::Unknown,
            recorded_at: Utc::now(),
        })
    }

    pub fn with_model_identity(
        mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let provider = provider.into();
        let model = model.into();
        if provider.trim().is_empty() || model.trim().is_empty() {
            return Err(DomainError::Validation(
                "Model usage identity cannot be empty".into(),
            ));
        }
        self.provider = Some(provider);
        self.model = Some(model);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_lifecycle() {
        let mut run = Run::new("task_001".into(), 1);
        assert_eq!(run.status, RunStatus::Pending);
        assert!(run.transition(RunStatus::Active).is_ok());
        assert!(run.transition(RunStatus::Suspended).is_ok());
        assert!(run.transition(RunStatus::Active).is_ok());
        assert!(run.transition(RunStatus::Completed).is_ok());
        assert!(run.status.is_terminal());
        assert!(run.ended_at.is_some());
        assert!(run.transition(RunStatus::Active).is_err());
    }

    #[test]
    fn test_worker_run_bounded_retry() {
        let mut wrun = WorkerRun::new("task_002".into(), "worker_alpha".into(), 3);
        assert_eq!(wrun.attempt_id, 1);
        assert!(wrun.can_retry());

        assert_eq!(wrun.record_attempt().unwrap(), 2);
        assert!(wrun.can_retry());

        assert_eq!(wrun.record_attempt().unwrap(), 3);
        assert!(!wrun.can_retry());

        // Exceeded bounded retry
        assert!(wrun.record_attempt().is_err());
    }

    #[test]
    fn test_dispatch_claim_lifecycle() {
        let mut claim = DispatchClaim::new("task_claim_1", "worker_1", 1);
        assert_eq!(claim.status, ClaimStatus::Pending);
        assert_eq!(claim.depth, 1);
        assert!(claim.dispatched_at.is_none());

        assert!(claim.mark_dispatched("wrun_1").is_ok());
        assert_eq!(claim.status, ClaimStatus::Dispatched);
        assert_eq!(claim.worker_run_id, Some("wrun_1".to_string()));
        assert!(claim.dispatched_at.is_some());

        // Cannot transition from dispatched to dispatched again
        assert!(claim.mark_dispatched("wrun_2").is_err());

        // Release
        assert!(claim.release().is_ok());
        assert_eq!(claim.status, ClaimStatus::Released);
        assert!(claim.released_at.is_some());
    }

    #[test]
    fn unknown_launch_cannot_be_blindly_retried_or_reclassified() {
        let mut attempt = LaunchAttempt::new("worker_run_1", "native");
        assert!(attempt.is_ok());
        if let Ok(ref mut value) = attempt {
            assert!(value
                .mark_unknown("attach outcome was not observed")
                .is_ok());
            assert!(!value.status.allows_retry());
            assert!(value.mark_launched("native", Some("codex".into())).is_err());
        }
    }

    #[test]
    fn usage_defaults_to_unknown_instead_of_zero() {
        let usage = UsageRecord::new(
            "task_1",
            "run_1",
            "attempt_1",
            UsageExecutorKind::NativeHarness,
            "codex",
        );
        assert!(usage.is_ok());
        if let Ok(value) = usage {
            assert_eq!(value.input_tokens, UsageMeasurement::Unknown);
            assert_eq!(value.cost_micros, UsageMeasurement::Unknown);
        }
    }
}
