//! Deterministic Workflow Compiler (RFC 004 §2C)
//!
//! Transforms an Admitted Strategy Proposal into an executable, typed `WorkflowRevision`.
//! Pure, deterministic, zero-I/O function.

use custos_domain::oi::{ExecutionTopology, NodePlacement, StrategyProposal};
use custos_domain::workflow::{RevisionNode, WorkflowRevision};

pub struct PlanCompiler;

impl PlanCompiler {
    /// Compiles an admitted StrategyProposal into an immutable WorkflowRevision.
    /// Guarantees determinism: identical inputs produce identical revisions.
    pub fn compile(proposal: &StrategyProposal, revision_number: u32) -> WorkflowRevision {
        let mut rev = WorkflowRevision::new(&proposal.task_id, &proposal.id, revision_number);
        let h = proposal.candidate_harness.as_str();
        let tid = &proposal.task_id;

        match proposal.chosen_topology {
            ExecutionTopology::T0Direct | ExecutionTopology::DirectModel => {
                rev.nodes.push(Self::node(
                    format!("{}_t0_turn", tid),
                    "Direct Synthesis",
                    "synthesizer",
                    h,
                    proposal.estimated_tokens,
                    vec![],
                    vec![],
                    vec![],
                ));
            }

            ExecutionTopology::T1SingleWorker | ExecutionTopology::NativeBaseline => {
                rev.nodes.push(Self::node(
                    format!("{}_t1_worker", tid),
                    "Governed Native Execution",
                    "worker",
                    h,
                    proposal.estimated_tokens,
                    vec!["workspace/**"],
                    vec!["workspace/**"],
                    vec!["shell", "file_write"],
                ));
                rev.obligations.push("evidence_verification".into());
            }

            ExecutionTopology::T2StagedPipeline => {
                let a_id = format!("{}_stage_1_analyze", tid);
                let e_id = format!("{}_stage_2_execute", tid);
                let v_id = format!("{}_stage_3_verify", tid);
                let b = proposal.estimated_tokens / 3;

                rev.nodes.push(Self::node(
                    &a_id,
                    "Workspace Analysis",
                    "analyzer",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec![],
                    vec!["read_file"],
                ));
                rev.nodes.push(Self::node(
                    &e_id,
                    "Patch Generation",
                    "coder",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec!["workspace/**"],
                    vec!["shell", "file_write"],
                ));
                rev.nodes.push(Self::node(
                    &v_id,
                    "Verification & Gate Proof",
                    "verifier",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec![],
                    vec!["shell"],
                ));
                rev.dependencies.push((a_id, e_id.clone()));
                rev.dependencies.push((e_id, v_id));
                rev.obligations.push("evidence_verification".into());
            }

            ExecutionTopology::T3ReadFanOut | ExecutionTopology::ParallelDag => {
                let f1 = format!("{}_fan_read_1", tid);
                let f2 = format!("{}_fan_read_2", tid);
                let m = format!("{}_merge_summary", tid);
                let b = proposal.estimated_tokens / 3;

                rev.nodes.push(Self::node(
                    &f1,
                    "Parallel Read Source",
                    "reader",
                    h,
                    b,
                    vec!["workspace/src/**"],
                    vec![],
                    vec!["read_file"],
                ));
                rev.nodes.push(Self::node(
                    &f2,
                    "Parallel Read Tests",
                    "reader",
                    h,
                    b,
                    vec!["workspace/tests/**"],
                    vec![],
                    vec!["read_file"],
                ));
                rev.nodes.push(Self::node(
                    &m,
                    "Consolidation & Merge",
                    "synthesizer",
                    h,
                    b,
                    vec![],
                    vec![],
                    vec![],
                ));
                rev.dependencies.push((f1, m.clone()));
                rev.dependencies.push((f2, m));
            }

            ExecutionTopology::T4Worktree => {
                let ba = format!("{}_branch_a", tid);
                let bb = format!("{}_branch_b", tid);
                let int = format!("{}_integrator", tid);
                let b = proposal.estimated_tokens / 3;

                rev.nodes.push(Self::node(
                    &ba,
                    "Worktree Branch A",
                    "worker",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec!["workspace/src/**"],
                    vec!["shell", "file_write"],
                ));
                rev.nodes.push(Self::node(
                    &bb,
                    "Worktree Branch B",
                    "worker",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec!["workspace/tests/**"],
                    vec!["shell", "file_write"],
                ));
                rev.nodes.push(Self::node(
                    &int,
                    "Integrator Merge & Test",
                    "integrator",
                    h,
                    b,
                    vec!["workspace/**"],
                    vec!["workspace/**"],
                    vec!["shell", "file_write"],
                ));
                rev.dependencies.push((ba, int.clone()));
                rev.dependencies.push((bb, int));
                rev.obligations.push("worktree_clean_merge".into());
                rev.obligations.push("evidence_verification".into());
            }

            ExecutionTopology::T6RepairLoop => {
                let ex = format!("{}_attempt_exec", tid);
                let ev = format!("{}_attempt_eval", tid);
                let half = proposal.estimated_tokens / 2;

                rev.nodes.push(Self::node(
                    &ex,
                    "Implementation Attempt",
                    "worker",
                    h,
                    half,
                    vec!["workspace/**"],
                    vec!["workspace/**"],
                    vec!["shell", "file_write"],
                ));
                rev.nodes.push(Self::node(
                    &ev,
                    "Evaluator Check",
                    "evaluator",
                    h,
                    half,
                    vec!["workspace/**"],
                    vec![],
                    vec!["shell"],
                ));
                rev.dependencies.push((ex, ev));
                rev.obligations.push("repair_budget_bounded".into());
            }

            _ => {
                rev.nodes.push(Self::node(
                    format!("{}_fallback_worker", tid),
                    "Sovereign Baseline",
                    "worker",
                    h,
                    proposal.estimated_tokens,
                    vec!["workspace/**"],
                    vec!["workspace/**"],
                    vec!["shell"],
                ));
            }
        }

        rev
    }

    /// Compiles both WorkflowRevision and corresponding NodePlacements,
    /// assigning isolated workspace leases for parallel worktree nodes.
    pub fn compile_with_placements(
        proposal: &StrategyProposal,
        revision_number: u32,
    ) -> (WorkflowRevision, Vec<NodePlacement>) {
        let revision = Self::compile(proposal, revision_number);
        let mut placements = Vec::new();

        for node in &revision.nodes {
            let mut placement = NodePlacement::new(
                &node.node_id,
                &node.role,
                &node.harness_id,
                node.allocated_budget_tokens,
            );

            if proposal.chosen_topology == ExecutionTopology::T4Worktree && node.role == "worker" {
                placement.workspace_lease_id =
                    Some(format!("lease_{}_{}", proposal.task_id, node.node_id));
            }

            placements.push(placement);
        }

        (revision, placements)
    }

    #[allow(clippy::too_many_arguments)]
    fn node(
        id: impl Into<String>,
        name: &str,
        role: &str,
        harness: &str,
        budget: u64,
        reads: Vec<&str>,
        writes: Vec<&str>,
        caps: Vec<&str>,
    ) -> RevisionNode {
        RevisionNode {
            node_id: id.into(),
            step_name: name.into(),
            role: role.into(),
            harness_id: harness.into(),
            allocated_budget_tokens: budget,
            read_set: reads.into_iter().map(String::from).collect(),
            write_set: writes.into_iter().map(String::from).collect(),
            required_capabilities: caps.into_iter().map(String::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_t0_direct() {
        let mut proposal = StrategyProposal::native_baseline("task_t0", "claude-code");
        proposal.chosen_topology = ExecutionTopology::T0Direct;
        let rev = PlanCompiler::compile(&proposal, 1);
        assert_eq!(rev.nodes.len(), 1);
        assert_eq!(rev.nodes[0].role, "synthesizer");
    }

    #[test]
    fn test_compile_t1_single_worker() {
        let proposal = StrategyProposal::native_baseline("task_t1", "claude-code");
        let rev = PlanCompiler::compile(&proposal, 1);
        assert_eq!(rev.nodes.len(), 1);
        assert_eq!(rev.nodes[0].role, "worker");
        assert!(rev.obligations.contains(&"evidence_verification".into()));
    }

    #[test]
    fn test_compile_t2_staged_pipeline() {
        let mut proposal = StrategyProposal::native_baseline("task_t2", "claude-code");
        proposal.chosen_topology = ExecutionTopology::T2StagedPipeline;
        proposal.estimated_tokens = 9_000;
        let rev = PlanCompiler::compile(&proposal, 1);
        assert_eq!(rev.nodes.len(), 3);
        assert_eq!(rev.dependencies.len(), 2);
    }

    #[test]
    fn test_compile_t3_fan_out() {
        let mut proposal = StrategyProposal::native_baseline("task_t3", "claude-code");
        proposal.chosen_topology = ExecutionTopology::T3ReadFanOut;
        let rev = PlanCompiler::compile(&proposal, 1);
        assert_eq!(rev.nodes.len(), 3);
        assert_eq!(rev.dependencies.len(), 2);
    }

    #[test]
    fn test_compile_t4_worktree_integrator() {
        let mut proposal = StrategyProposal::native_baseline("task_t4", "claude-code");
        proposal.chosen_topology = ExecutionTopology::T4Worktree;
        proposal.estimated_tokens = 6_000;
        let (rev, placements) = PlanCompiler::compile_with_placements(&proposal, 1);
        assert_eq!(rev.nodes.len(), 3);
        assert_eq!(rev.dependencies.len(), 2);
        assert!(rev.obligations.contains(&"worktree_clean_merge".into()));
        assert_eq!(placements.len(), 3);
        assert!(placements[0].workspace_lease_id.is_some());
        assert!(placements[1].workspace_lease_id.is_some());
    }
}
