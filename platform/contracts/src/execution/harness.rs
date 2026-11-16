//! Agent Harness capability profiles, mediation assurances, and descriptor types.
//!
//! Enforces Gate 4 (Native Bypass Assurance): External/native harnesses that run their
//! own tools cannot fraudulently claim `custos-mediated` assurance.

use serde::{Deserialize, Serialize};

use crate::{ActionIntent, Assurance, DomainError};

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

        if !self.is_custos_governed() && assurance == Assurance::Unknown {
            return Err(DomainError::Validation(format!(
                "Gate 4 Violation: Harness '{}' ({:?}) emitted action '{}' with missing/unknown assurance (must explicitly declare 'provider-governed' or 'observe-only')",
                self.harness_id, self.tool_mediation, intent.name
            )));
        }

        Ok(())
    }
}

/// Outcome of running a native harness turn or process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub observed_effects: Vec<ActionIntent>,
    pub mediation_level: ToolMediationLevel,
}

/// Conformance and maturity level of a registered agent harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessMaturity {
    /// Dedicated native adapter with structured protocol parsing, live observer, and verified error recovery.
    VerifiedNative,
    /// Discovered CLI binary on host PATH; supports interactive PTY/TUI launch, but no structured event protocol.
    DiscoveredCli,
}

fn default_maturity() -> HarnessMaturity {
    HarnessMaturity::DiscoveredCli
}

/// Descriptive metadata and capability profile of a registered agent harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub binary_path: String,
    pub is_available: bool,
    #[serde(default)]
    pub launch_cmd: Option<String>,
    #[serde(default = "default_maturity")]
    pub maturity: HarnessMaturity,
    pub profile: HarnessProfile,
}

