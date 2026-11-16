//! Execution Topology proposed by Orchestration Intelligence (RFC 003 / RFC 004)
//!
//! Categorizes execution strategy from simple direct model invocations to
//! complex staged pipelines, parallel fan-outs, and repair loops.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTopology {
    // ────────── RFC 003 Canonical Aliases (Backwards-compatible) ──────────
    /// Strong native single-agent baseline (e.g. Claude 3.5 Sonnet / Governed Worker)
    NativeBaseline,
    /// Direct model turn without tool invocation
    DirectModel,
    /// Multi-worker DAG topology with dependency waves
    ParallelDag,

    // ────────── Target Topologies T0–T8 (RFC 004 Specification) ──────────
    /// T0: Direct model turn (zero tool execution, instant response)
    T0Direct,
    /// T1: Sovereign single-worker agent with bounded tools (Default Baseline)
    T1SingleWorker,
    /// T2: Staged pipeline (sequential stages with handoff artifacts)
    T2StagedPipeline,
    /// T3: Read fan-out / fan-in (parallel discovery/analysis over read-only targets)
    T3ReadFanOut,
    /// T4: Isolated worktree workers with integrator merge
    T4Worktree,
    /// T5: Lead worker with specialized sub-workers / scouts
    T5LeadWorker,
    /// T6: Bounded repair loop (executor -> evaluator -> patcher)
    T6RepairLoop,
    /// T7: Candidate search / exploration with competitive selection
    T7CandidateSearch,
    /// T8: Event-driven reactive orchestrator
    T8EventDriven,
}

impl ExecutionTopology {
    /// Returns true if this topology utilizes concurrent workers.
    pub fn is_parallel(&self) -> bool {
        matches!(
            self,
            Self::ParallelDag
                | Self::T3ReadFanOut
                | Self::T4Worktree
                | Self::T5LeadWorker
                | Self::T7CandidateSearch
        )
    }

    /// Returns true if this topology requires strong filesystem sandbox isolation (Seatbelt/bwrap).
    pub fn requires_isolation(&self) -> bool {
        matches!(
            self,
            Self::NativeBaseline
                | Self::ParallelDag
                | Self::T1SingleWorker
                | Self::T4Worktree
                | Self::T6RepairLoop
        )
    }
}

impl std::fmt::Display for ExecutionTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NativeBaseline => write!(f, "native_baseline"),
            Self::DirectModel => write!(f, "direct_model"),
            Self::ParallelDag => write!(f, "parallel_dag"),
            Self::T0Direct => write!(f, "t0_direct"),
            Self::T1SingleWorker => write!(f, "t1_single_worker"),
            Self::T2StagedPipeline => write!(f, "t2_staged_pipeline"),
            Self::T3ReadFanOut => write!(f, "t3_read_fan_out"),
            Self::T4Worktree => write!(f, "t4_worktree"),
            Self::T5LeadWorker => write!(f, "t5_lead_worker"),
            Self::T6RepairLoop => write!(f, "t6_repair_loop"),
            Self::T7CandidateSearch => write!(f, "t7_candidate_search"),
            Self::T8EventDriven => write!(f, "t8_event_driven"),
        }
    }
}
