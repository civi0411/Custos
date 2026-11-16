//! Task, Workflow, and Orchestration SPI Trait Ports

use async_trait::async_trait;
use crate::identity::error::DomainError;
use crate::task::{
    CancelReceipt, CandidateRaceResult, CandidateRaceSpec, ContinuationPacket,
    LiveRunReplay, RunHandle, Span, StartRunCommand,
    Task, TaskContract, TaskStatus,
};
use crate::task::events::TaskEvent;
use crate::task::oi::{DecisionSnapshot, StrategyProposal};
use crate::authority::{ActionIntent, Grant, Permit, RiskLevel};
use crate::evidence::{EvidenceBundle, VerificationClaim};

/// Primary entry point into the Task Execution Kernel (CONTRACT-LAYER-01).
#[async_trait]
pub trait KernelPort: Send + Sync {
    async fn create_task(&self, title: String) -> Result<Task, DomainError>;
    async fn create_task_with_contract(
        &self,
        title: String,
        contract: TaskContract,
    ) -> Result<Task, DomainError>;
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError>;
    async fn transition_task(&self, task_id: &str, next: TaskStatus) -> Result<Task, DomainError>;

    async fn request_permit(
        &self,
        task_id: &str,
        intent: &ActionIntent,
        actor: &str,
    ) -> Result<Permit, DomainError>;

    async fn consume_permit(
        &self,
        permit_id: &str,
        intent: &ActionIntent,
    ) -> Result<Permit, DomainError>;

    async fn verify_evidence(
        &self,
        bundle: &EvidenceBundle,
    ) -> Result<VerificationClaim, DomainError>;

    async fn grant_capability(
        &self,
        task_id: &str,
        grantee: &str,
        capability: &str,
    ) -> Result<Grant, DomainError>;

    async fn find_active_grant(
        &self,
        task_id: &str,
        capability: &str,
    ) -> Result<Option<Grant>, DomainError>;

    async fn complete_task(
        &self,
        task_id: &str,
        summary: String,
        claims: Vec<VerificationClaim>,
    ) -> Result<Task, DomainError>;

    fn classify_effect(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> RiskLevel {
        match tool_name {
            "developer__read" | "read_file" | "read" | "list_dir" | "list" | "ls" => {
                RiskLevel::Low
            }
            "developer__write" | "write_file" | "write" => RiskLevel::Medium,
            "developer__bash" | "bash" | "shell" | "exec" => {
                let cmd = args
                    .get("command")
                    .or_else(|| args.get("cmd"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if cmd.contains("rm -rf") || cmd.contains("mkfs") || cmd.contains("dd if=") {
                    RiskLevel::Critical
                } else {
                    RiskLevel::High
                }
            }
            name if name.starts_with("dangerous_") => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        }
    }
}

/// Persistence Port for Tasks, Spans, and Continuations
#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError>;
    async fn list_tasks(&self) -> Result<Vec<Task>, DomainError>;
    async fn save_task(&self, task: &Task) -> Result<(), DomainError>;
    async fn commit_task_event(&self, event: &TaskEvent, task: &Task) -> Result<(), DomainError>;
    async fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, DomainError>;
    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError>;
    async fn save_span(&self, span: &Span) -> Result<(), DomainError>;
    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError>;
    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError>;
    async fn get_continuation(
        &self,
        task_id: &str,
        to_span: u32,
    ) -> Result<Option<ContinuationPacket>, DomainError>;
    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError>;
}

/// Port 5 — Workflow Execution Port
#[async_trait]
pub trait WorkflowPort: Send + Sync {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError>;

    async fn start_run_background(&self, _cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        Err(DomainError::Validation(
            "Background execution is not supported".into(),
        ))
    }

    async fn live_run(
        &self,
        _session_id: &str,
        _run_id: &str,
        _cursor: u64,
    ) -> Result<LiveRunReplay, DomainError> {
        Err(DomainError::Validation(
            "Live run projection is not supported".into(),
        ))
    }

    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError>;

    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError>;
    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError>;

    async fn execute_candidate_race(
        &self,
        _spec: CandidateRaceSpec,
    ) -> Result<CandidateRaceResult, DomainError> {
        Err(DomainError::Validation(
            "Candidate race is not supported by this workflow runtime".into(),
        ))
    }

    async fn run_candidate_race(
        &self,
        spec: CandidateRaceSpec,
    ) -> Result<CandidateRaceResult, DomainError> {
        self.execute_candidate_race(spec).await
    }
}

/// Orchestration Intelligence (OI) Planner Port
#[async_trait]
pub trait OiPlannerPort: Send + Sync {
    async fn plan(&self, snapshot: &DecisionSnapshot) -> Result<StrategyProposal, DomainError>;
}

use crate::task::run::{DispatchClaim, LaunchAttempt, Run, WorkerRun};
use crate::task::workflow::WorkflowRevision;
use crate::task::oi::placement::NodePlacement;
use crate::authority::ports::EffectLedgerPort;
use crate::resources::ports::{DecisionPort, ReplanPort};

/// Composite Persistence Port for sovereign Runs and bounded WorkerRuns.
#[async_trait]
pub trait RunPort: Send + Sync {
    async fn save_run(&self, run: &mut Run) -> Result<(), DomainError>;
    async fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError>;
    async fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError>;
    async fn save_worker_run(&self, wrun: &mut WorkerRun) -> Result<(), DomainError>;
    async fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError>;
    async fn list_worker_runs_for_task(&self, task_id: &str) -> Result<Vec<WorkerRun>, DomainError>;
    async fn list_worker_runs_for_run(&self, run_id: &str) -> Result<Vec<WorkerRun>, DomainError>;
    async fn list_all_active_worker_runs(&self) -> Result<Vec<WorkerRun>, DomainError> {
        Ok(Vec::new())
    }
    async fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError>;
    async fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError>;
    async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError>;
    async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError>;
    async fn save_launch_attempt(&self, attempt: &LaunchAttempt) -> Result<(), DomainError>;
    async fn get_launch_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<LaunchAttempt>, DomainError>;
    async fn list_active_claims(&self) -> Result<Vec<DispatchClaim>, DomainError> {
        Ok(Vec::new())
    }
    async fn fence_claim(&self, _claim_id: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        Ok(0)
    }
}

pub trait RunStore: RunPort + EffectLedgerPort {}
impl<T: RunPort + EffectLedgerPort + ?Sized> RunStore for T {}

/// Persistence Port for Workflow Revisions and Node Placements (RFC 004).
#[async_trait]
pub trait WorkflowRevisionPort: Send + Sync {
    async fn save_revision(
        &self,
        revision: &WorkflowRevision,
        placements: &[NodePlacement],
    ) -> Result<(), DomainError>;
    async fn get_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<WorkflowRevision>, DomainError>;
    async fn list_revisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkflowRevision>, DomainError>;
    async fn get_placements_for_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<NodePlacement>, DomainError>;
}

/// Composite Persistence Port for Workflow Revisions, Decisions, and Replanning.
pub trait WorkflowStore: WorkflowRevisionPort + DecisionPort + ReplanPort {}
impl<T: WorkflowRevisionPort + DecisionPort + ReplanPort + ?Sized> WorkflowStore for T {}

