//! Domain Pack Core Interfaces & Workbench Job Modeling
//!
//! Authoritatively maps to Custos SADE Master Specification Parts 10-13,
//! defining DomainPack traits, WorkbenchJob descriptions, WorkerRole definitions,
//! and verification bindings.

use async_trait::async_trait;
use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::profile::PackProfile;
use crate::verifier::PackVerifier;

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
///
/// Models the distinct work units available within each lens:
/// - Engineering: RepoExplain, Diagnose, Bugfix, Feature, Refactor, Review, Migration
/// - Research: SourceQA, Compare, DeepResearch, DatasetAudit, ExperimentDesign, Reproduce, ResultAnalysis, ResearchToCode
/// - Assistant: PersonalQA, InboxTriage, ScheduleProposal, MutateExecution, ReminderAutomation
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
}
