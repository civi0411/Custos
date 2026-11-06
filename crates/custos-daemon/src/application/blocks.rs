//! Canonical Workbench Building Blocks for Custos Daemon Application Layer
//!
//! Provides classification, routing metadata, and architectural grouping
//! for all daemon JSON-RPC / HTTP dispatch handlers:
//! - Engineering: Workspaces, Git, linked worktrees, and native PTY terminals
//! - Research: Scientific claims, notebooks, syntheses, reviews, experiments, and compute targets
//! - Quant: Alpha factors, backtesting simulations, institutional risk gates, and execution blotters
//! - Copilot: Personal assistant, sessions, communication drafts, outbox approvals, and model providers
//! - Foundation: Distributed task DAGs, system settings, domain pack registries, and host automation

use serde::{Deserialize, Serialize};

/// The canonical building block identifier for application-layer dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchBlock {
    Engineering,
    Research,
    Quant,
    Copilot,
    Foundation,
}

impl WorkbenchBlock {
    /// Returns the canonical kebab-case / snake-case name of the block.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Engineering => "engineering",
            Self::Research => "research",
            Self::Quant => "quant",
            Self::Copilot => "copilot",
            Self::Foundation => "foundation",
        }
    }

    /// Returns true if this block corresponds to one of the 4 user-facing Workbenches.
    pub const fn is_workbench(&self) -> bool {
        matches!(
            self,
            Self::Engineering | Self::Research | Self::Quant | Self::Copilot
        )
    }
}

/// Classifies a daemon method (e.g. `custos.workspaces.list` or `quant.backtest`)
/// into its responsible Workbench Building Block.
pub fn classify_method(method: &str) -> WorkbenchBlock {
    let parts: Vec<&str> = method.split('.').collect();
    let prefix = match parts.as_slice() {
        ["v1" | "custos", domain, ..] => *domain,
        [domain, ..] => *domain,
        [] => "",
    };

    match prefix {
        // Engineering Workbench
        "workspaces" | "workspace" | "github" | "terminal" => WorkbenchBlock::Engineering,

        // Research Workbench
        "research" | "notebook" | "reviews" | "synthesis" | "artifacts" | "notes" | "compute"
        | "experiment" => WorkbenchBlock::Research,

        // Quant Workbench
        "quant" => WorkbenchBlock::Quant,

        // Copilot Workbench
        "assistant" | "sessions" | "session" | "bridge" | "providers" | "models"
        | "local_models" | "oauth" | "keys" | "llm" | "harness" => WorkbenchBlock::Copilot,

        // Foundation Block
        "tasks" | "workflow" | "oi" | "browser" | "emulator" | "fleet" | "automation" | "packs"
        | "packages" | "settings" | "system" | "capabilities" | "resources" | "ping" | "health"
        | "pipeline" => WorkbenchBlock::Foundation,

        _ => WorkbenchBlock::Foundation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_all_workbench_blocks() {
        assert_eq!(
            classify_method("custos.workspaces.list"),
            WorkbenchBlock::Engineering
        );
        assert_eq!(
            classify_method("v1.terminal.spawn"),
            WorkbenchBlock::Engineering
        );

        assert_eq!(
            classify_method("research.claim.extract"),
            WorkbenchBlock::Research
        );
        assert_eq!(
            classify_method("custos.notebook.execute"),
            WorkbenchBlock::Research
        );
        assert_eq!(
            classify_method("compute.gpu.lease"),
            WorkbenchBlock::Research
        );

        assert_eq!(classify_method("quant.backtest.run"), WorkbenchBlock::Quant);
        assert_eq!(
            classify_method("v1.quant.risk_audit"),
            WorkbenchBlock::Quant
        );

        assert_eq!(
            classify_method("assistant.draft.list"),
            WorkbenchBlock::Copilot
        );
        assert_eq!(classify_method("sessions.create"), WorkbenchBlock::Copilot);
        assert_eq!(
            classify_method("providers.anthropic.stream"),
            WorkbenchBlock::Copilot
        );

        assert_eq!(classify_method("tasks.create"), WorkbenchBlock::Foundation);
        assert_eq!(
            classify_method("packs.inventory"),
            WorkbenchBlock::Foundation
        );
        assert_eq!(
            classify_method("system.capabilities"),
            WorkbenchBlock::Foundation
        );
    }
}
