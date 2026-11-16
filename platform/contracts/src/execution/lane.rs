//! Execution Lanes & Lane Policy Specification (SADE R7)
//!
//! Authoritatively maps to Custos SADE Master Specification §5 & §7,
//! defining priority-stratified execution lanes (`Interactive`, `Agent`, `Batch`, `Ambient`)
//! and per-workbench concurrency policies (`LanePolicy`) to prevent resource exhaustion,
//! eliminate head-of-line blocking, and guarantee real-time interactive responsiveness.

use serde::{Deserialize, Serialize};

/// Hierarchical priority execution lanes for concurrent agent tasks and compute workloads
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLane {
    /// Ambient background tasks: vector indexing, OCR, sync, heartbeat.
    /// Lowest priority; first to yield and strictly preemptible.
    Ambient = 0,
    /// Batch computing: backtest sweeps, offline model training, eval validation.
    /// Checkpointable; runs when agent/interactive slots are free.
    Batch = 1,
    /// Autonomous agent iterations: coding worktree agents, literature scouts, claim extractors.
    /// Governed by workbench quota.
    #[default]
    Agent = 2,
    /// Interactive user session: direct chat, diff inspection, terminal commands, human approvals.
    /// Top priority; non-blocking; can preempt ambient and batch workloads.
    Interactive = 3,
}

impl ExecutionLane {
    /// Numeric priority rank (higher value = higher scheduling priority)
    pub fn priority_rank(&self) -> u8 {
        match self {
            Self::Interactive => 100,
            Self::Agent => 75,
            Self::Batch => 50,
            Self::Ambient => 25,
        }
    }

    /// Whether this lane can preempt an existing workload in `other` lane
    pub fn can_preempt(&self, other: &ExecutionLane) -> bool {
        self.priority_rank() > other.priority_rank() && other.is_preemptible()
    }

    /// Whether workloads in this lane are permitted to be preempted when resources are exhausted
    pub fn is_preemptible(&self) -> bool {
        matches!(self, Self::Ambient | Self::Batch)
    }

    /// Returns canonical string identifier for metrics, logs, and telemetry
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Agent => "agent",
            Self::Batch => "batch",
            Self::Ambient => "ambient",
        }
    }
}

/// Concurrency quota and preemption policy for a workbench or runtime lane
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanePolicy {
    /// Default lane assigned to workloads from this module when unspecified
    pub default_lane: ExecutionLane,
    /// Maximum concurrent interactive sessions (default: 16)
    pub max_concurrent_interactive: usize,
    /// Maximum concurrent autonomous agents / worktrees (default: 4)
    pub max_concurrent_agents: usize,
    /// Maximum concurrent batch / simulation workloads (default: 2)
    pub max_concurrent_batch: usize,
    /// Maximum concurrent ambient / background tasks (default: 2)
    pub max_concurrent_ambient: usize,
    /// Whether higher-priority lanes may preempt preemptible workloads
    pub allow_preemption: bool,
}

impl Default for LanePolicy {
    fn default() -> Self {
        Self {
            default_lane: ExecutionLane::Agent,
            max_concurrent_interactive: 16,
            max_concurrent_agents: 4,
            max_concurrent_batch: 2,
            max_concurrent_ambient: 2,
            allow_preemption: true,
        }
    }
}

impl LanePolicy {
    /// Maximum concurrency allowed for a specific lane
    pub fn max_concurrency_for_lane(&self, lane: ExecutionLane) -> usize {
        match lane {
            ExecutionLane::Interactive => self.max_concurrent_interactive,
            ExecutionLane::Agent => self.max_concurrent_agents,
            ExecutionLane::Batch => self.max_concurrent_batch,
            ExecutionLane::Ambient => self.max_concurrent_ambient,
        }
    }

    /// Specialized policy for Jarvis / Assistant: high interactive throughput, strict ambient throttling
    pub fn interactive_focused() -> Self {
        Self {
            default_lane: ExecutionLane::Interactive,
            max_concurrent_interactive: 32,
            max_concurrent_agents: 2,
            max_concurrent_batch: 1,
            max_concurrent_ambient: 2,
            allow_preemption: true,
        }
    }

    /// Specialized policy for Orca / Coding: multi-worktree parallel agent fan-out
    pub fn coding_fanout(max_worktrees: usize) -> Self {
        Self {
            default_lane: ExecutionLane::Agent,
            max_concurrent_interactive: 16,
            max_concurrent_agents: max_worktrees.max(1),
            max_concurrent_batch: 2,
            max_concurrent_ambient: 1,
            allow_preemption: true,
        }
    }

    /// Specialized policy for Open Science / Research: massive fan-out for web & literature scouts
    pub fn research_fanout(max_scouts: usize) -> Self {
        Self {
            default_lane: ExecutionLane::Agent,
            max_concurrent_interactive: 16,
            max_concurrent_agents: max_scouts.max(1),
            max_concurrent_batch: 4,
            max_concurrent_ambient: 2,
            allow_preemption: true,
        }
    }

    /// Specialized policy for QuantDinger / Quant: strict admission for batch backtesting and simulation
    pub fn quant_batch(max_simulations: usize) -> Self {
        Self {
            default_lane: ExecutionLane::Batch,
            max_concurrent_interactive: 16,
            max_concurrent_agents: 2,
            max_concurrent_batch: max_simulations.max(1),
            max_concurrent_ambient: 1,
            allow_preemption: false, // Backtests are compute-intensive; do not cancel mid-run unless requested
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_priority_ordering() {
        assert!(ExecutionLane::Interactive > ExecutionLane::Agent);
        assert!(ExecutionLane::Agent > ExecutionLane::Batch);
        assert!(ExecutionLane::Batch > ExecutionLane::Ambient);

        assert_eq!(ExecutionLane::Interactive.priority_rank(), 100);
        assert_eq!(ExecutionLane::Ambient.priority_rank(), 25);
    }

    #[test]
    fn test_lane_preemption_rules() {
        assert!(ExecutionLane::Interactive.can_preempt(&ExecutionLane::Ambient));
        assert!(ExecutionLane::Interactive.can_preempt(&ExecutionLane::Batch));
        assert!(!ExecutionLane::Interactive.can_preempt(&ExecutionLane::Agent));
        assert!(!ExecutionLane::Interactive.can_preempt(&ExecutionLane::Interactive));

        assert!(!ExecutionLane::Agent.can_preempt(&ExecutionLane::Interactive));
        assert!(ExecutionLane::Agent.can_preempt(&ExecutionLane::Ambient));
    }

    #[test]
    fn test_lane_policy_presets() {
        let assistant = LanePolicy::interactive_focused();
        assert_eq!(assistant.default_lane, ExecutionLane::Interactive);
        assert_eq!(assistant.max_concurrent_interactive, 32);

        let coding = LanePolicy::coding_fanout(8);
        assert_eq!(coding.max_concurrency_for_lane(ExecutionLane::Agent), 8);

        let quant = LanePolicy::quant_batch(4);
        assert_eq!(quant.max_concurrency_for_lane(ExecutionLane::Batch), 4);
        assert!(!quant.allow_preemption);
    }
}
