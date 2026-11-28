//! Bridges compiled `WorkflowRevision` + `NodePlacement` to `GraphRuntime` -> `WorkflowPort` (`TaskRuntime`).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use custos_contracts::{
    DomainError, NodePlacement, StartRunCommand, WorkflowPort, WorkflowRevision,
};

use super::super::contracts::ExecutionContext;
use super::super::dag::{DagGraph, DagNode};
use super::super::graph_runtime::{GraphExecutionReport, GraphRuntime, StepExecutor};

/// `StepExecutor` adapter delegating each DAG node to `WorkflowPort::start_run` (`TaskRuntime`).
pub struct TaskRuntimeStepExecutor {
    workflow: Arc<dyn WorkflowPort>,
    base_cmd: StartRunCommand,
    placements: HashMap<String, NodePlacement>,
}

impl TaskRuntimeStepExecutor {
    pub fn new(
        workflow: Arc<dyn WorkflowPort>,
        base_cmd: StartRunCommand,
        placements: Vec<NodePlacement>,
    ) -> Self {
        let placements = placements
            .into_iter()
            .map(|p| (p.node_id.clone(), p))
            .collect();
        Self {
            workflow,
            base_cmd,
            placements,
        }
    }
}

#[async_trait]
impl StepExecutor for TaskRuntimeStepExecutor {
    async fn execute_step(
        &self,
        ctx: &ExecutionContext,
        action_type: &str,
        _inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        let mut cmd = self.base_cmd.clone();
        cmd.node_id = Some(ctx.node_id.clone());
        if let Some(placement) = self.placements.get(&ctx.node_id) {
            cmd.harness_id = Some(placement.backend_harness.clone());
            if let Some(ref lease_id) = placement.workspace_lease_id {
                cmd.workspace_lease_id = Some(lease_id.clone());
                cmd.isolated_worktree = true;
            }
        }
        let handle = self.workflow.start_run(cmd).await?;
        Ok(serde_json::json!({
            "run_id": handle.run_id,
            "task_id": handle.task_id,
            "action_type": action_type,
            "status": format!("{:?}", handle.status),
        }))
    }
}

/// Compiles a `WorkflowRevision` into a `DagGraph` and executes all waves via `GraphRuntime`.
pub struct PersonaPlanRunner;

impl PersonaPlanRunner {
    pub fn build_dag(revision: &WorkflowRevision) -> Result<DagGraph, DomainError> {
        let mut graph = DagGraph::new(format!("revision:{}", revision.revision_id));
        graph.id = revision.revision_id.clone();
        graph.task_id = Some(revision.task_id.clone());

        let mut deps_by_node: HashMap<String, Vec<String>> = HashMap::new();
        for (from_node_id, to_node_id) in &revision.dependencies {
            deps_by_node
                .entry(to_node_id.clone())
                .or_default()
                .push(from_node_id.clone());
        }

        for node in &revision.nodes {
            let deps = deps_by_node.remove(&node.node_id).unwrap_or_default();
            graph.add_node(DagNode::new(
                node.node_id.clone(),
                node.step_name.clone(),
                node.role.clone(),
                serde_json::json!({
                    "harness_id": node.harness_id,
                    "budget_tokens": node.allocated_budget_tokens,
                }),
                deps,
            ))?;
        }
        graph.validate_dag()?;
        Ok(graph)
    }

    pub async fn execute_plan(
        workflow: Arc<dyn WorkflowPort>,
        base_cmd: StartRunCommand,
        revision: &WorkflowRevision,
        placements: Vec<NodePlacement>,
    ) -> Result<GraphExecutionReport, DomainError> {
        let dag = Self::build_dag(revision)?;
        let executor = Arc::new(TaskRuntimeStepExecutor::new(workflow, base_cmd, placements));
        let runtime = GraphRuntime::new(dag, executor);
        runtime.execute_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::oi::{
        DecisionSnapshotExtractor, OiEngine, PersonaDispatcher, PlanCompiler,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use custos_contracts::authority::action::RiskLevel;
    use custos_contracts::oi::{
        ActsForTrustPolicy, CallerIdentity, ExecutionTopology, MemoryScopeId, WhoCanAskPolicy,
    };
    use custos_contracts::{
        AgentPersonaProfile, CancelReceipt, ExecutorDescriptor, ExecutorRegistryPort,
        RunHandle, RunStatus, Task, WorkbenchId,
    };
    use tokio::sync::Mutex;

    struct StaticExecutors;
    impl ExecutorRegistryPort for StaticExecutors {
        fn list_available_executors(&self) -> Vec<ExecutorDescriptor> {
            vec![ExecutorDescriptor {
                harness_id: "claude-code".into(),
                supports_worktree: true,
                supports_read_only: true,
            }]
        }
    }

    struct RecordingWorkflowPort {
        dispatched: Mutex<Vec<StartRunCommand>>,
    }

    #[async_trait]
    impl WorkflowPort for RecordingWorkflowPort {
        async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
            let task_id = cmd.task_id.clone();
            self.dispatched.lock().await.push(cmd);
            Ok(RunHandle::new("run-1", task_id, RunStatus::Completed, Utc::now()))
        }

        async fn request_cancel(
            &self,
            run_id: &str,
            reason: &str,
        ) -> Result<CancelReceipt, DomainError> {
            Ok(CancelReceipt {
                run_id: run_id.into(),
                cancelled_at: Utc::now(),
                reason: reason.into(),
                uncertain_effects_count: 0,
            })
        }

        async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError> {
            Ok(RunHandle::new(
                run_id,
                "task-persona-e2e",
                RunStatus::Completed,
                Utc::now(),
            ))
        }

        async fn checkpoint(
            &self,
            _run_id: &str,
        ) -> Result<custos_contracts::ContinuationPacket, DomainError> {
            Err(DomainError::Validation("Not implemented in mock".into()))
        }
    }

    #[tokio::test]
    async fn persona_plan_compiles_and_dispatches_via_graph_runtime() {
        let profile = AgentPersonaProfile {
            id: "persona-coding".into(),
            name: "Coding".into(),
            role: "Engineer".into(),
            default_workbench: WorkbenchId::new(WorkbenchId::CODING),
            memory_scope: MemoryScopeId::new("persona:coding"),
            allowed_triggers: WhoCanAskPolicy::OwnerOnly,
            acts_for: ActsForTrustPolicy {
                allowed_workbenches: vec![WorkbenchId::new(WorkbenchId::CODING)],
                allowed_capabilities: vec!["fs_read".into()],
                max_autonomous_risk: RiskLevel::Low,
                require_biometric_for_mutations: true,
                allow_mobile_dispatch: true,
            },
            sovereign_instructions: String::new(),
            look_around_job_id: None,
            max_up_next_items: 5,
            enabled: true,
        };
        let caller = CallerIdentity {
            sender_id: "owner".into(),
            scope: "mobile".into(),
            is_paired_owner: true,
        };
        let task = Task::new("task-persona-e2e".into(), "Inspect repo".into());
        let snapshot = DecisionSnapshotExtractor::extract_snapshot_with_executors(
            &task,
            None,
            Some(&StaticExecutors),
            None,
            50_000,
            5.0,
        )
        .await
        .expect("snapshot");
        assert_eq!(snapshot.available_executors, vec!["claude-code".to_string()]);

        let oi = OiEngine::new();
        let (mut proposal, _auth) = PersonaDispatcher::plan_for_persona(
            &profile,
            &caller,
            &WorkbenchId::new(WorkbenchId::CODING),
            RiskLevel::Low,
            &snapshot,
            &oi,
        )
        .await
        .expect("plan");
        proposal.chosen_topology = ExecutionTopology::T4Worktree;

        let (revision, placements) = PlanCompiler::compile_with_placements(&proposal, 1);
        let workflow = Arc::new(RecordingWorkflowPort {
            dispatched: Mutex::new(Vec::new()),
        });
        let mut base_cmd = StartRunCommand::new("task-persona-e2e", "persona:persona-coding");
        base_cmd.persona_id = Some("persona-coding".into());

        let report = PersonaPlanRunner::execute_plan(
            workflow.clone(),
            base_cmd,
            &revision,
            placements,
        )
        .await
        .expect("execute_plan");

        assert!(report.success);
        let calls = workflow.dispatched.lock().await;
        assert_eq!(calls.len(), revision.nodes.len());
        assert!(calls.iter().any(|c| c.isolated_worktree));
        assert!(calls.iter().all(|c| c.node_id.is_some()));
    }
}

