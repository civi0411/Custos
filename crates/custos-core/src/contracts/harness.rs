//! Agent Runtime / Native Harness Port (Port 7)
//!
//! Standardized contract governing native coding agents (Claude Code, Codex, Goose)
//! and governed agent runtimes under RFC 002.
//!
//! Enforces Gate 4 (Native Bypass Assurance): External/native harnesses that run their
//! own tools cannot fraudulently claim `custos-mediated` assurance.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

use custos_domain::{ActionIntent, Assurance, ContextPack, DomainError, WorkerRun};

/// Tool mediation level indicating how the native harness executes external mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolMediationLevel {
    /// Custos intercepts every tool call, requiring explicit ExecutionPermits.
    CustosMediated,
    /// The external harness runs its own tool execution / permissions engine (e.g., Claude Code native bypass).
    ProviderGoverned,
    /// Custos only observes effects after the fact (e.g. git diff, file watches).
    ObserveOnly,
}

impl std::fmt::Display for ToolMediationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CustosMediated => write!(f, "custos-mediated"),
            Self::ProviderGoverned => write!(f, "provider-governed"),
            Self::ObserveOnly => write!(f, "observe-only"),
        }
    }
}

/// Worktree isolation strategy used by the native harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeOwnership {
    /// Operates directly on the shared live repository/workspace.
    SharedLive,
    /// Operates inside an isolated worktree or ephemeral clone.
    IsolatedWorktree,
}

/// Visibility and accuracy of token/compute costs reported by the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostVisibility {
    ExactTokens,
    Estimated,
    None,
}

/// Honest capability profile declared by an Agent Runtime / Native Harness (RFC 002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessProfile {
    pub harness_id: String,
    pub tool_mediation: ToolMediationLevel,
    pub worktree_ownership: WorktreeOwnership,
    pub supports_cancel: bool,
    pub supports_steer: bool,
    pub cost_visibility: CostVisibility,
}

impl HarnessProfile {
    pub fn new(harness_id: impl Into<String>, tool_mediation: ToolMediationLevel) -> Self {
        Self {
            harness_id: harness_id.into(),
            tool_mediation,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: false,
            supports_steer: false,
            cost_visibility: CostVisibility::None,
        }
    }

    pub fn with_supports_cancel(mut self, supports_cancel: bool) -> Self {
        self.supports_cancel = supports_cancel;
        self
    }

    pub fn with_supports_steer(mut self, supports_steer: bool) -> Self {
        self.supports_steer = supports_steer;
        self
    }

    pub fn with_cost_visibility(mut self, cost_visibility: CostVisibility) -> Self {
        self.cost_visibility = cost_visibility;
        self
    }

    pub fn with_worktree_ownership(mut self, ownership: WorktreeOwnership) -> Self {
        self.worktree_ownership = ownership;
        self
    }

    pub fn is_custos_governed(&self) -> bool {
        self.tool_mediation == ToolMediationLevel::CustosMediated
    }

    /// Gate 4 verification: Ensures that an observed or emitted action intent
    /// does not fraudulently claim `custos-mediated` assurance when the harness
    /// has native bypass or external execution (`tool_mediation != CustosMediated`).
    pub fn validate_intent_assurance(&self, intent: &ActionIntent) -> Result<(), DomainError> {
        let assurance = if intent.assurance != Assurance::Unknown {
            intent.assurance
        } else {
            intent
                .parameters
                .get("assurance")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<Assurance>().unwrap_or_default())
                .unwrap_or(Assurance::Unknown)
        };

        if assurance == Assurance::CustosMediated && !self.is_custos_governed() {
            return Err(DomainError::Validation(format!(
                "Gate 4 Violation: Harness '{}' ({:?}) cannot claim 'custos-mediated' assurance for action '{}' (must be 'provider-governed' or 'observe-only')",
                self.harness_id, self.tool_mediation, intent.name
            )));
        }

        // Allow-list enforcement: Non-governed harness must declare concrete assurance (not Unknown)
        if !self.is_custos_governed() && assurance == Assurance::Unknown {
            return Err(DomainError::Validation(format!(
                "Gate 4 Violation: Harness '{}' ({:?}) emitted action '{}' with missing/unknown assurance (must explicitly declare 'provider-governed' or 'observe-only')",
                self.harness_id, self.tool_mediation, intent.name
            )));
        }

        Ok(())
    }
}

/// Outcome of running a native harness turn or process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub observed_effects: Vec<ActionIntent>,
    pub mediation_level: ToolMediationLevel,
}

/// Descriptive metadata and capability profile of a registered agent harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub binary_path: String,
    pub is_available: bool,
    pub profile: HarnessProfile,
}


/// Agent Runtime Port (Deep Dissection of Agent Loop & Native Harnesses)
///
/// Dissects the legacy upstream execution loop into a strictly governed contract.
/// Concrete agent engines (Goose loops, Claude Code / Codex wrappers) implement this trait.
#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    /// Unique identifier of this harness (e.g. "claude-code", "codex", "governed-agent").
    fn harness_id(&self) -> &str;

    /// Honest capability and assurance profile (RFC 002).
    fn profile(&self) -> HarnessProfile;

    /// Executes a single agent step or full worker turn under the governed WorkerRun contract.
    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError>;

    /// Signals cancellation to an ongoing agent run.
    async fn cancel_run(&self, _run_id: &str) -> Result<(), DomainError> {
        if !self.profile().supports_cancel {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support cancellation",
                self.harness_id()
            )));
        }
        Ok(())
    }

    /// Injects steering guidance/feedback to an ongoing agent run.
    async fn steer_run(&self, _run_id: &str, _guidance: &str) -> Result<(), DomainError> {
        if !self.profile().supports_steer {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support steering",
                self.harness_id()
            )));
        }
        Ok(())
    }

    /// Executes a native subprocess instruction with assurance tracking.
    async fn run_native(
        &self,
        _instruction: &str,
        _cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        Err(DomainError::Validation(
            "Native subprocess execution not supported by this harness".into(),
        ))
    }
}
