//! Domain Pack Core Interfaces & Workbench Contract Specifications
//!
//! Authoritatively maps to Custos SADE Master Specification Parts 10-13,
//! defining DomainPack traits, WorkbenchJob descriptions, WorkerRole definitions,
//! PackProfile constraints, and verification oracle interfaces.

use async_trait::async_trait;
use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A specialized Subagent Recipe / Worker Role within a Domain Pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRole {
    /// Unique identifier: e.g. "repo_scout", "code_implementer", "literature_scout"
    pub role_id: String,
    /// Associated pack identifier: "engineering", "research", or "assistant"
    pub pack_id: String,
    /// Human-readable display name
    pub name: String,
    /// Description of the role's objective
    pub description: String,
    /// Detailed prompt / instructions tailored for this role
    pub system_prompt: String,
    /// Default token budget slice for this role's execution
    pub default_budget_tokens: u64,
    /// Permitted skill identifiers (checked by KernelPort & SkillRegistry)
    pub allowed_skills: Vec<String>,
    /// If true, the role is strictly read-only and cannot mutate workspace/state
    pub read_only: bool,
}

impl WorkerRole {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        role_id: impl Into<String>,
        pack_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        system_prompt: impl Into<String>,
        default_budget_tokens: u64,
        allowed_skills: &[&str],
        read_only: bool,
    ) -> Self {
        Self {
            role_id: role_id.into(),
            pack_id: pack_id.into(),
            name: name.into(),
            description: description.into(),
            system_prompt: system_prompt.into(),
            default_budget_tokens,
            allowed_skills: allowed_skills.iter().map(|s| s.to_string()).collect(),
            read_only,
        }
    }
}

/// A standardized Workbench Job specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchJob {
    /// Unique job identifier: e.g. "engineering__bugfix", "research__deep_research"
    pub job_id: String,
    /// The home workbench lens where this job is primarily surfaced
    pub lens: WorkbenchLens,
    /// Human-readable name
    pub name: String,
    /// Purpose and scope of the job
    pub description: String,
    /// Required input parameters / preconditions
    pub required_inputs: Vec<String>,
    /// Expected output artifacts (e.g. "PatchBundle", "ReadingCard", "OutboxReceipt")
    pub expected_artifacts: Vec<String>,
    /// Recommended execution topology
    pub recommended_topology: ExecutionTopology,
    /// Primary worker role assigned to drive this job
    pub primary_role: String,
    /// Required verifiers that must validate the outcome before completion (Gate 4)
    pub required_verifiers: Vec<String>,
    /// Acceptance rubric describing completion criteria (INV-05)
    pub acceptance_rubric: String,
}

impl WorkbenchJob {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: impl Into<String>,
        lens: WorkbenchLens,
        name: impl Into<String>,
        description: impl Into<String>,
        required_inputs: &[&str],
        expected_artifacts: &[&str],
        recommended_topology: ExecutionTopology,
        primary_role: impl Into<String>,
        required_verifiers: &[&str],
        acceptance_rubric: impl Into<String>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            lens,
            name: name.into(),
            description: description.into(),
            required_inputs: required_inputs.iter().map(|s| s.to_string()).collect(),
            expected_artifacts: expected_artifacts.iter().map(|s| s.to_string()).collect(),
            recommended_topology,
            primary_role: primary_role.into(),
            required_verifiers: required_verifiers.iter().map(|s| s.to_string()).collect(),
            acceptance_rubric: acceptance_rubric.into(),
        }
    }
}

/// Metadata manifest for a Domain Pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub lens: WorkbenchLens,
    pub description: String,
    pub supported_jobs: Vec<String>,
    pub supported_roles: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

/// Canonical configuration and constraints for a domain pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackProfile {
    pub pack_id: String,
    pub name: String,
    pub default_topology: ExecutionTopology,
    pub supported_topologies: Vec<ExecutionTopology>,
    pub preferred_verifier: String,
    pub default_budget_tokens: u64,
    /// If false, automated blind retries on uncertainty are prohibited.
    pub allow_blind_retry: bool,
}

impl PackProfile {
    pub fn new(
        pack_id: impl Into<String>,
        name: impl Into<String>,
        default_topology: ExecutionTopology,
        supported_topologies: Vec<ExecutionTopology>,
        preferred_verifier: impl Into<String>,
        default_budget_tokens: u64,
        allow_blind_retry: bool,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            name: name.into(),
            default_topology,
            supported_topologies,
            preferred_verifier: preferred_verifier.into(),
            default_budget_tokens,
            allow_blind_retry,
        }
    }

    /// Checks if a topology is supported by this pack.
    pub fn supports_topology(&self, topology: ExecutionTopology) -> bool {
        self.supported_topologies.contains(&topology)
    }
}

/// Result of an oracle verification evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VerificationResult {
    /// Requirement fully verified by independent evidence.
    Pass { detail: String },
    /// Requirement failed with a reproducible violation.
    Fail { reason: String },
    /// Inconclusive evidence or environment error.
    Unknown { reason: String },
    /// Underlying evidence or artifact has become stale.
    Stale { reason: String },
}

impl VerificationResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, VerificationResult::Pass { .. })
    }
}

pub type VerifierResult = VerificationResult;

/// Context supplied to a verifier execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationContext {
    pub task_id: String,
    pub workspace_path: Option<String>,
    pub target_artifact: Option<String>,
    pub metadata: serde_json::Value,
}

impl VerificationContext {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            workspace_path: None,
            target_artifact: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Independent oracle interface for validating task completion before closing deliberation.
#[async_trait]
pub trait PackVerifier: Send + Sync {
    fn verifier_id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult;
}

/// Canonical trait that all Domain Packs must implement.
#[async_trait]
pub trait DomainPack: Send + Sync {
    /// Returns the descriptive manifest for this pack
    fn manifest(&self) -> DomainPackManifest;

    /// Returns the execution profile and budget constraints
    fn profile(&self) -> PackProfile;

    /// Returns all specialized Worker Roles (Subagents) supported by this pack
    fn roles(&self) -> Vec<WorkerRole>;

    /// Returns all standardized Workbench Jobs offered by this pack
    fn jobs(&self) -> Vec<WorkbenchJob>;

    /// Returns the collection of verification oracles owned by this pack
    fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>>;

    /// Asynchronous bootstrap hook during daemon startup
    async fn initialize(&self) -> Result<(), DomainError>;

    /// Returns the concurrency quota and lane execution policy for this domain pack (SADE R7)
    fn lane_policy(&self) -> custos_domain::LanePolicy {
        custos_domain::LanePolicy::default()
    }

    /// Returns an optional workbench-specific execution supervision profile (thresholds and exemptions).
    fn supervision_profile(&self) -> Option<custos_domain::supervision::SupervisionProfile> {
        None
    }

    /// Tool declarations exposed by this pack for governed agent runtime disclosure
    fn tool_declarations(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Executes a tool call owned by this domain pack
    async fn execute_tool(
        &self,
        tool_name: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        Err(DomainError::Validation(format!(
            "Domain pack does not implement execution for tool '{tool_name}'"
        )))
    }
}

/// Extension interface implemented by sovereign domain workbenches (SADE R8 / Master Plan).
pub trait WorkbenchModule: Send + Sync {
    /// Canonical identifier for the workbench (e.g. "coding", "assistant", "research", "quant")
    fn id(&self) -> &'static str;

    /// Legacy or alternative alias identifiers (e.g. "engineering", "copilot")
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// DomainPack implementation providing manifest, roles, jobs, and verifiers
    fn pack(&self) -> Arc<dyn DomainPack>;

    /// Registers the workbench's specialized cognitive skills into the runtime registry
    fn register_skills(&self, registry: &mut dyn crate::contracts::skill::SkillSink);

    /// Optional idempotent SQL migrations for this workbench (applied only when enabled)
    fn migrations(&self) -> &'static [&'static str] {
        &[]
    }
}


/// Lifecycle hooks invoked when a pack is dynamically loaded or unloaded from the registry
#[async_trait]
pub trait PackLifecycle: Send + Sync {
    /// Hook invoked immediately after the pack is registered into the active runtime
    async fn on_load(&self) -> Result<(), DomainError> {
        Ok(())
    }

    /// Hook invoked prior to pack deregistration and resource disposal
    async fn on_unload(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

impl<T: DomainPack + ?Sized> PackLifecycle for T {}
