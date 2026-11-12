//! Port 1 — KernelPort
//!
//! The ONLY door from Runtime/Bridge/Daemon into the Trusted Kernel (CONTRACT-LAYER-01).
//! Callers never receive a database handle; they get tasks, permits and verified claims.

use async_trait::async_trait;
use custos_domain::{
    ActionIntent, DomainError, Permit, Task, TaskContract, TaskStatus, VerificationClaim,
};

use crate::evidence::EvidenceBundle;
pub use crate::kernel::trusted::TrustedKernel;

#[async_trait]
pub trait KernelPort: Send + Sync {
    async fn create_task(&self, title: String) -> Result<Task, DomainError>;
    /// Create a task whose `TaskContract.evidence_requirements` gate completion.
    async fn create_task_with_contract(
        &self,
        title: String,
        contract: TaskContract,
    ) -> Result<Task, DomainError>;
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError>;
    async fn transition_task(&self, task_id: &str, next: TaskStatus) -> Result<Task, DomainError>;

    /// Authority step: evaluate policy + grants, mint a single-use permit bound to the
    /// intent's argument digest. Errors with `Unauthorized`/`Conflict` when denied or when
    /// human approval is still required.
    async fn request_permit(
        &self,
        task_id: &str,
        intent: &ActionIntent,
        actor: &str,
    ) -> Result<Permit, DomainError>;

    /// Burn the permit (single-use, digest-bound). Runtime MUST call this immediately before
    /// dispatching to a SandboxPort/CapabilityPort. Today the ledger is in RAM; Gate C makes it
    /// durable via OutboxPort (T3).
    async fn consume_permit(
        &self,
        permit_id: &str,
        intent: &ActionIntent,
    ) -> Result<Permit, DomainError>;

    /// Evidence step: run the registered deterministic verifier for the bundle.
    async fn verify_evidence(
        &self,
        bundle: &EvidenceBundle,
    ) -> Result<VerificationClaim, DomainError>;

    /// Endow a task with an explicit capability grant.
    async fn grant_capability(
        &self,
        task_id: &str,
        grantee: &str,
        capability: &str,
    ) -> Result<custos_domain::Grant, DomainError>;

    /// Find an active grant for a task and capability.
    async fn find_active_grant(
        &self,
        task_id: &str,
        capability: &str,
    ) -> Result<Option<custos_domain::Grant>, DomainError>;

    /// The ONLY way to reach `Succeeded` (plain `transition_task` is refused). Runs the
    /// Completion Gate: task must be Running and every required contract evidence kind must be
    /// backed by a passing, non-stale claim (INV-02).
    async fn complete_task(
        &self,
        task_id: &str,
        summary: String,
        claims: Vec<VerificationClaim>,
    ) -> Result<Task, DomainError>;

    /// Classify the effect of a requested skill by its declared capability profile (SADE R8).
    fn classify_profile(
        &self,
        profile: &crate::contracts::skill::SkillCapabilityProfile,
    ) -> custos_domain::RiskLevel {
        profile.risk_level()
    }

    /// Classify the effect of a requested tool call into a RiskLevel.
    fn classify_effect(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> custos_domain::RiskLevel {
        match tool_name {
            "developer__read" | "read_file" | "read" | "list_dir" | "list" | "ls" => {
                custos_domain::RiskLevel::Low
            }
            "developer__write" | "write_file" | "write" => custos_domain::RiskLevel::Medium,
            "developer__bash" | "bash" | "shell" | "exec" => {
                let cmd = args
                    .get("command")
                    .or_else(|| args.get("cmd"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if cmd.contains("rm -rf") || cmd.contains("mkfs") || cmd.contains("dd if=") {
                    custos_domain::RiskLevel::Critical
                } else {
                    custos_domain::RiskLevel::High
                }
            }
            name if name.starts_with("dangerous_") => custos_domain::RiskLevel::Critical,
            _ => custos_domain::RiskLevel::Medium,
        }
    }
}
