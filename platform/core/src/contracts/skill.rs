//! Universal Skill Contract & Registration Port (Layer 1 Core)
//!
//! Provides the universal `Skill` and `SkillSink` traits that domain packs use
//! to declare and register capabilities cleanly without tying Core to runtime state.

use std::sync::Arc;

use async_trait::async_trait;
use custos_domain::{ids::TaskId, DomainError};
use serde::{Deserialize, Serialize};

/// Worker run identifier alias
pub type WorkerRunId = String;

/// Taint level for skill output — directly maps to Custos Taint Tracking Engine (§8.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaintLevel {
    /// Output originated from trusted internal sources (code, tests, DB)
    Clean,
    /// Output came from external/web sources — cannot become policy instruction
    Untrusted,
    /// Contains credentials or PII — must not enter prompt context
    HighlySensitive,
}

/// Write target descriptor for capability_profile audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTarget {
    /// Logical category: "filesystem", "git_worktree", "network", "email", "calendar"
    pub category: String,
    /// Human-readable scope description
    pub scope_hint: String,
}

impl WriteTarget {
    pub fn filesystem(scope: impl Into<String>) -> Self {
        Self {
            category: "filesystem".into(),
            scope_hint: scope.into(),
        }
    }
    pub fn network(scope: impl Into<String>) -> Self {
        Self {
            category: "network".into(),
            scope_hint: scope.into(),
        }
    }
    pub fn git_worktree(scope: impl Into<String>) -> Self {
        Self {
            category: "git_worktree".into(),
            scope_hint: scope.into(),
        }
    }
}

/// Capability declaration that OI audits before granting skill execution.
/// Skills MUST NOT misrepresent their write_set — this is a safety invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCapabilityProfile {
    /// Empty = read-only, safe. Non-empty = OI checks Permit before dispatch.
    pub write_set: Vec<WriteTarget>,
    /// If true, a Permit with matching argument_digest is required before run().
    pub requires_permit: bool,
    /// If true, output is marked Taint::Untrusted and cannot become system policy.
    pub taint_output: bool,
    /// Maximum allowed execution wall-clock time in milliseconds.
    pub max_execution_ms: u64,
}

impl SkillCapabilityProfile {
    /// Constructor for a purely read-only skill (most common case).
    pub fn read_only(max_execution_ms: u64) -> Self {
        Self {
            write_set: vec![],
            requires_permit: false,
            taint_output: false,
            max_execution_ms,
        }
    }

    /// Constructor for a read-only skill whose output comes from external sources.
    pub fn external_read(max_execution_ms: u64) -> Self {
        Self {
            write_set: vec![],
            requires_permit: false,
            taint_output: true, // External content → always Untrusted
            max_execution_ms,
        }
    }

    /// Derives the authoritative risk level based on declared write_set and permit requirements (SADE R8)
    pub fn risk_level(&self) -> custos_domain::RiskLevel {
        if self.requires_permit {
            if self
                .write_set
                .iter()
                .any(|w| w.category == "git_worktree" || w.category == "process")
            {
                custos_domain::RiskLevel::High
            } else {
                custos_domain::RiskLevel::Medium
            }
        } else if self.write_set.is_empty() {
            custos_domain::RiskLevel::Low
        } else {
            custos_domain::RiskLevel::Medium
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScope {
    pub root: std::path::PathBuf,
}

impl WorkspaceScope {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

/// Execution context passed to every skill invocation.
/// Carries routing metadata from the 9-Router (D1–D9) alongside task identity.
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// JSON arguments provided by the worker/OI plan node
    pub args: serde_json::Value,
    /// Task that owns this execution
    pub task_id: TaskId,
    /// Worker run that is calling this skill
    pub worker_run_id: WorkerRunId,
    /// Resolved provider ID from D2 (for context only)
    pub provider_id: String,
    /// Resolved pack ID from D5
    pub pack_id: Option<String>,
    /// Optional workspace boundary for filesystem operations
    pub scope_root: Option<WorkspaceScope>,
}

impl SkillContext {
    pub fn new(
        args: serde_json::Value,
        task_id: impl Into<TaskId>,
        worker_run_id: impl Into<WorkerRunId>,
        provider_id: impl Into<String>,
        pack_id: Option<String>,
    ) -> Self {
        Self {
            args,
            task_id: task_id.into(),
            worker_run_id: worker_run_id.into(),
            provider_id: provider_id.into(),
            pack_id,
            scope_root: None,
        }
    }

    pub fn with_scope_root(mut self, scope: WorkspaceScope) -> Self {
        self.scope_root = Some(scope);
        self
    }
}

/// Result produced by a skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Matches Skill::skill_id() for audit trail
    pub skill_id: String,
    /// Structured result payload — parsed by verifiers and OI
    pub result: serde_json::Value,
    /// CAS references for large artifacts (diffs, logs, papers)
    pub artifacts: Vec<String>,
    /// Optional hint string to feed into EvidenceClaim construction
    pub evidence_hint: Option<String>,
    /// Taint level of this output — propagates to any downstream data
    pub taint: TaintLevel,
}

impl SkillOutput {
    pub fn clean(skill_id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            skill_id: skill_id.into(),
            result,
            artifacts: vec![],
            evidence_hint: None,
            taint: TaintLevel::Clean,
        }
    }

    pub fn untrusted(skill_id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            skill_id: skill_id.into(),
            result,
            artifacts: vec![],
            evidence_hint: None,
            taint: TaintLevel::Untrusted,
        }
    }
}

/// Skill execution error
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("missing required argument: {0}")]
    MissingArg(String),
    #[error("argument validation failed: {0}")]
    InvalidArg(String),
    #[error("execution timeout after {0}ms")]
    Timeout(u64),
    #[error("sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("permit required but not provided")]
    PermitRequired,
    #[error("external fetch failed: {0}")]
    FetchError(String),
    #[error("no workspace scope configured")]
    NoScope,
    #[error("path out of workspace scope: {0}")]
    OutOfScope(String),
    #[error("integer overflow in offset/limit calculation")]
    Overflow,
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Universal Skill trait — the fundamental unit of capability in Custos.
///
/// Every domain-specific skill (AstSearchSkill, CargoTestSkill, DoiVerifySkill, etc.)
/// implements this trait. The `skill_id()` is the exact string used in D6 `tool_set`.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Unique, stable skill identifier. Used as D6 tool_set entry in RouteDecision.
    /// Convention: `{domain}_{verb}` e.g. "cargo_test", "doi_verify", "email_send"
    fn skill_id(&self) -> &'static str;

    /// Human-readable description for observability and debug
    fn description(&self) -> &'static str;

    /// Capability declaration — OI reads this BEFORE granting execution permission.
    /// A skill with write_set entries requires a valid Permit.
    fn capability_profile(&self) -> SkillCapabilityProfile;

    /// Execute the skill. Called only after OI has validated capability_profile.
    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError>;
}

/// Pure registration port implemented by runtime skill registries (`custos-runtime`).
pub trait SkillSink: Send + Sync {
    /// Register a concrete skill instance.
    fn register(&mut self, skill: Arc<dyn Skill>);

    /// Define which skills are permitted for a given reasoning tier + pack combination.
    fn set_tier_permissions(&mut self, tier: &str, pack_id: &str, skill_ids: Vec<String>);
}
