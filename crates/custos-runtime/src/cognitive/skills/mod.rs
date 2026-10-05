//! Custos Skill Foundation Layer
//!
//! Provides the universal `Skill` trait and `SkillRegistry` that all domain packs use
//! to register their capabilities. The OI Router resolves D6 (`tool_set`) from
//! registered skill IDs at plan-time, enabling auditable, permission-aware tool dispatch.
//!
//! # Architecture
//! ```text
//! RouteDecision.tool_set (D6) ──► SkillRegistry.resolve_tool_set()
//!                                         │
//!                               ┌─────────▼────────────┐
//!                               │  Arc<dyn Skill> list  │
//!                               └─────────┬────────────┘
//!                                         │ run(SkillContext)
//!                                         ▼
//!                                   SkillOutput
//!                                  (artifacts, evidence_hint, taint)
//! ```

pub mod fs;
pub mod http;
pub mod process;
pub mod search;

pub use fs::*;
pub use http::*;
pub use process::*;
pub use search::*;

use async_trait::async_trait;
use custos_domain::{ids::TaskId, DomainError};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

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

/// Central registry where all domain packs register their skills.
/// The 9-Router resolves D6 `tool_set` strings through this registry.
#[derive(Default)]
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
    /// Tier-based permission sets: maps (tier, pack_id) → allowed skill_ids
    tier_permissions: HashMap<(String, String), Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a skill. Called by Pack::initialize() for each domain skill.
    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let id = skill.skill_id().to_string();
        tracing::debug!(skill_id = %id, "registered skill");
        self.skills.insert(id, skill);
    }

    /// Resolve a list of skill_id strings to concrete skill instances.
    /// Used by OI to construct the executable tool set for a worker run.
    pub fn resolve_tool_set(&self, tool_ids: &[String]) -> Vec<Arc<dyn Skill>> {
        tool_ids
            .iter()
            .filter_map(|id| self.skills.get(id).cloned())
            .collect()
    }

    /// Look up a single skill by ID.
    pub fn get(&self, skill_id: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(skill_id).cloned()
    }

    /// Check if a skill_id is registered.
    pub fn contains(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    /// List all registered skill IDs (for observability).
    pub fn all_skill_ids(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Define which skills are permitted for a given reasoning tier + pack combo.
    /// This replaces the hardcoded "all" string in routing.rs.
    pub fn set_tier_permissions(
        &mut self,
        tier: impl Into<String>,
        pack_id: impl Into<String>,
        skill_ids: Vec<String>,
    ) {
        self.tier_permissions
            .insert((tier.into(), pack_id.into()), skill_ids);
    }

    /// Retrieve permitted skill IDs for a routing decision.
    /// Returns None if no explicit mapping is set (caller should use a safe default).
    pub fn permitted_skills_for(&self, tier: &str, pack_id: Option<&str>) -> Option<&Vec<String>> {
        let pack = pack_id.unwrap_or("_any");
        self.tier_permissions
            .get(&(tier.to_string(), pack.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoSkill;

    #[async_trait]
    impl Skill for EchoSkill {
        fn skill_id(&self) -> &'static str {
            "echo"
        }
        fn description(&self) -> &'static str {
            "Test echo skill"
        }
        fn capability_profile(&self) -> SkillCapabilityProfile {
            SkillCapabilityProfile::read_only(1_000)
        }
        async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
            Ok(SkillOutput::clean(self.skill_id(), ctx.args.clone()))
        }
    }

    #[test]
    fn registry_register_and_resolve() {
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(EchoSkill));
        assert!(registry.contains("echo"));
        let resolved = registry.resolve_tool_set(&["echo".to_string()]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].skill_id(), "echo");
    }

    #[test]
    fn tier_permissions_set_and_query() {
        let mut registry = SkillRegistry::new();
        registry.set_tier_permissions(
            "system_two",
            "engineering",
            vec!["cargo_test".to_string(), "git_patch".to_string()],
        );
        let perms = registry.permitted_skills_for("system_two", Some("engineering"));
        assert!(perms.is_some());
        assert!(perms.unwrap().contains(&"cargo_test".to_string()));
    }

    #[test]
    fn unknown_skill_ids_are_filtered_gracefully() {
        let registry = SkillRegistry::new();
        let resolved = registry.resolve_tool_set(&["nonexistent".to_string()]);
        assert!(resolved.is_empty());
    }
}
