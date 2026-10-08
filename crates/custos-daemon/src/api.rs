//! Custos Local API Contracts & Dispatcher
//!
//! Protocol types and dispatcher for IPC communication between CLI / UI / VS Code and custosd.

use std::sync::Arc;

pub use crate::custos_local_api::{
    AdvanceTaskRequest, ApiRequest, ApiResponse, ArchiveWorkspaceApiRequest, CancelRunRequest,
    CancelTaskRequest, CompleteTaskRequest, CreateTaskRequest, CreateWorkspaceApiRequest,
    GetCapabilityApiRequest, GetWorkspaceApiRequest, StartRunRequest, METHOD_CAPABILITIES_GET,
    METHOD_CAPABILITIES_LIST, METHOD_RESEARCH_ANCHORS_LIST, METHOD_RESEARCH_ANCHORS_SAVE,
    METHOD_RESEARCH_CLAIMS_LIST, METHOD_RESEARCH_CLAIMS_SAVE, METHOD_RESEARCH_HANDOFF_CODING,
    METHOD_RESEARCH_LINEAGE_LIST, METHOD_RESEARCH_RUNS_LIST, METHOD_RESEARCH_RUNS_SAVE,
    METHOD_RESEARCH_SOURCES_LIST, METHOD_RESEARCH_SOURCES_SAVE, METHOD_WORKFLOW_CANCEL_RUN,
    METHOD_WORKFLOW_START_RUN, METHOD_WORKSPACES_ARCHIVE, METHOD_WORKSPACES_CREATE,
    METHOD_WORKSPACES_GET, METHOD_WORKSPACES_LIST,
};
use custos_bridge::{AttachMode, BridgePort, BridgeService};
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::{AdvanceTask, CancelTask, CreateTask, TaskService};
use custos_domain::{
    CapabilityDescriptor, CapabilityGroup, PassageAnchor, ResearchClaim, ResearchExperimentRun,
    SessionId, SessionMode, SourceRecord, TaskContract, TaskStatus,
};
use custos_persistence::{ProviderRepository, ResearchRepository};
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::{CreateWorkspaceRequest, WorkspaceCoordinator};

/// Local API Dispatcher wrapping TaskService, SessionManager, BridgeService, WorkflowPort, ResearchRepository, and ProviderRepository for IPC callers.
pub struct LocalApiDispatcher {
    task_service: Arc<TaskService>,
    session_manager: Arc<SessionManager>,
    bridge_service: Arc<BridgeService>,
    workflow: Option<Arc<dyn WorkflowPort>>,
    workspace: Option<Arc<WorkspaceCoordinator>>,
    research: Option<Arc<ResearchRepository>>,
    providers: Option<Arc<ProviderRepository>>,
}

impl LocalApiDispatcher {
    pub fn new(
        task_service: Arc<TaskService>,
        session_manager: Arc<SessionManager>,
        bridge_service: Arc<BridgeService>,
    ) -> Self {
        Self {
            task_service,
            session_manager,
            bridge_service,
            workflow: None,
            workspace: None,
            research: None,
            providers: None,
        }
    }

    pub fn with_workflow(mut self, workflow: Arc<dyn WorkflowPort>) -> Self {
        self.workflow = Some(workflow);
        self
    }

    pub fn with_workspace(mut self, workspace: Arc<WorkspaceCoordinator>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn with_research(mut self, research: Arc<ResearchRepository>) -> Self {
        self.research = Some(research);
        self
    }

    pub fn with_providers(mut self, providers: Arc<ProviderRepository>) -> Self {
        self.providers = Some(providers);
        self
    }

    /// List all capabilities registered in the Custos ADE kernel truthfully.
    ///
    /// Workbenches project views over these capabilities; they do not maintain
    /// simulated independent backends. Unimplemented or unconfigured surfaces
    /// report `Unavailable` or `Degraded` with an explicit reason.
    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        vec![
            // Core Task Engine
            CapabilityDescriptor::available(
                "tasks.core",
                "Task Engine",
                CapabilityGroup::Coordination,
                "Kernel task lifecycle, contract enforcement, and verification proofs.",
                Some("tasks"),
                vec!["create", "get", "cancel", "advance", "complete"],
            ),
            // Workspaces (Git & Folder Execution Workspaces)
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "workspace.git",
                    "Workspaces",
                    CapabilityGroup::Code,
                    "Inspect daemon-owned folder and Git execution workspaces.",
                    Some("worktrees"),
                    vec!["create", "list", "get", "archive"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "workspace.git",
                    "Workspaces",
                    CapabilityGroup::Code,
                    "Inspect daemon-owned folder and Git execution workspaces.",
                    Some("worktrees"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            // Workflow DAG Engine
            if self.workflow.is_some() {
                CapabilityDescriptor::available(
                    "workflow.dag",
                    "Workflow",
                    CapabilityGroup::Coordination,
                    "Execution graph, dependencies, budgets, and run orchestration.",
                    Some("dag"),
                    vec!["start_run", "cancel_run"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "workflow.dag",
                    "Workflow",
                    CapabilityGroup::Coordination,
                    "Execution graph, dependencies, budgets, and run orchestration.",
                    Some("dag"),
                    "Workflow engine is not attached; graph orchestration is degraded.",
                    vec![],
                )
            },
            // Research Literature Sources
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.sources",
                    "Sources",
                    CapabilityGroup::Evidence,
                    "Versioned literature, passages, DOI anchors, and corpus coverage.",
                    Some("literature"),
                    vec!["list", "save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.sources",
                    "Sources",
                    CapabilityGroup::Evidence,
                    "Versioned literature, passages, DOI anchors, and corpus coverage.",
                    Some("literature"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Claims Matrix
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.claims",
                    "Claims",
                    CapabilityGroup::Evidence,
                    "Atomic claims with support, contradiction, or unknown evidence grounding.",
                    Some("claims"),
                    vec!["list", "save", "handoff_coding"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.claims",
                    "Claims",
                    CapabilityGroup::Evidence,
                    "Atomic claims with support, contradiction, or unknown evidence grounding.",
                    Some("claims"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Methods / Recipes & Executions
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.methods",
                    "Methods",
                    CapabilityGroup::Evidence,
                    "Reproduction recipes separated from observed execution records.",
                    Some("knowledge"),
                    vec!["recipes.list", "recipes.get", "executions.list", "executions.get"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.methods",
                    "Methods",
                    CapabilityGroup::Evidence,
                    "Reproduction recipes separated from observed execution records.",
                    Some("knowledge"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Runs Ledger
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.runs",
                    "Runs",
                    CapabilityGroup::Compute,
                    "Experiment attempts, environments, outputs, and execution receipts.",
                    Some("synthesis"),
                    vec!["list", "save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.runs",
                    "Runs",
                    CapabilityGroup::Compute,
                    "Experiment attempts, environments, outputs, and execution receipts.",
                    Some("synthesis"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Artifacts & Lineage
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.artifacts",
                    "Artifacts",
                    CapabilityGroup::Evidence,
                    "Version lineage, annotations, provenance DAG, and review findings.",
                    Some("artifacts"),
                    vec!["lineage.list", "annotations.list", "annotations.save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.artifacts",
                    "Artifacts",
                    CapabilityGroup::Evidence,
                    "Version lineage, annotations, provenance DAG, and review findings.",
                    Some("artifacts"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Providers (LLM backend)
            if self.providers.is_some() {
                CapabilityDescriptor::available(
                    "providers.llm",
                    "Providers",
                    CapabilityGroup::Conversation,
                    "Model provider credentials, client keys, and health probes.",
                    Some("providers"),
                    vec!["list", "save", "check_status", "keys.list", "keys.generate"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "providers.llm",
                    "Providers",
                    CapabilityGroup::Conversation,
                    "Model provider credentials, client keys, and health probes.",
                    Some("providers"),
                    "Provider repository not configured on daemon instance.",
                    vec![],
                )
            },
            // Roadmap Capabilities (truthfully reported as Unavailable with roadmap milestone)
            CapabilityDescriptor::unavailable(
                "compute.pty",
                "Terminal",
                CapabilityGroup::Compute,
                "Bounded PTY streams scoped to an execution workspace.",
                Some("terminal"),
                "Bounded PTY streams scoped to an execution workspace scheduled in Roadmap Step 3.",
            ),
            CapabilityDescriptor::unavailable(
                "code.files",
                "Files",
                CapabilityGroup::Code,
                "Repository tree, file buffers, and source anchor inspection.",
                Some("files"),
                "Safe Git workspace file/editor/diff API scheduled in Roadmap Step 4.",
            ),
            CapabilityDescriptor::unavailable(
                "code.changes",
                "Changes",
                CapabilityGroup::Code,
                "Diff review, patch proposal, annotations, and approval receipts.",
                Some("changes"),
                "Interactive diff review & patch approval API scheduled in Roadmap Step 4.",
            ),
            CapabilityDescriptor::unavailable(
                "compute.notebook",
                "Notebook",
                CapabilityGroup::Compute,
                "Authorized kernels, code cells, and reproducible compute epochs.",
                Some("experiments"),
                "Authorized kernels, code cells and reproducible compute scheduled in Roadmap Step 7.",
            ),
            CapabilityDescriptor::unavailable(
                "evidence.criteria",
                "Evidence",
                CapabilityGroup::Evidence,
                "Criteria, receipts, verifier records, and freshness status.",
                Some("evidence"),
                "Criteria verifier records and receipts scheduled in Roadmap Step 8.",
            ),
            CapabilityDescriptor::unavailable(
                "coordination.kanban",
                "Agents",
                CapabilityGroup::Coordination,
                "Worker runs, attention state, and delegated multi-agent dispatch.",
                Some("kanban"),
                "Worker runs, attention state and delegated task topology scheduled in Roadmap Step 10.",
            ),
            CapabilityDescriptor::unavailable(
                "browser.tabs",
                "Browser",
                CapabilityGroup::Browser,
                "Scoped browsing, DOM snapshots, network inspector, and page capture.",
                Some("browser"),
                "Scoped browsing and page capture scheduled in Roadmap Step 10.",
            ),
            CapabilityDescriptor::unavailable(
                "personal.notes",
                "Notes",
                CapabilityGroup::Personal,
                "Task notes, scratchpads, and Markdown knowledge capture.",
                Some("notes"),
                "Task notes and Markdown artifacts scheduled in Roadmap Step 6.",
            ),
        ]
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let response = match serde_json::from_str::<ApiRequest>(raw) {
            Ok(request) => self.handle_request(request).await,
            Err(error) => ApiResponse::error("", format!("Invalid request JSON: {error}")),
        };
        serde_json::to_string(&response).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    pub async fn handle_request(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            "v1.tasks.create" => {
                let params: CreateTaskRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let cmd = CreateTask {
                    title: params.title,
                    metadata: params.metadata,
                    contract: params.contract,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.tasks.get" => {
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.task_service.get_task(task_id).await {
                    Ok(Some(task)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Task {task_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.tasks.cancel" => {
                let params: CancelTaskRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let task = match self.task_service.get_task(&params.task_id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Task {} not found", params.task_id),
                        );
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let cmd = CancelTask {
                    task_id: params.task_id,
                    expected_epoch: task.epoch,
                    reason: params
                        .reason
                        .unwrap_or_else(|| "User cancelled via API".into()),
                };

                match self.task_service.execute_cancel(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.tasks.advance" => {
                let params: AdvanceTaskRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                if params.target_status == TaskStatus::Succeeded {
                    return ApiResponse::error(
                        req.id,
                        "Cannot advance directly to Succeeded. Use v1.tasks.complete with verification proof.",
                    );
                }

                let task = match self.task_service.get_task(&params.task_id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Task {} not found", params.task_id),
                        );
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let cmd = AdvanceTask {
                    task_id: params.task_id,
                    expected_epoch: task.epoch,
                    next_status: params.target_status,
                    rationale: Some("Advance requested via Local API".into()),
                };

                match self.task_service.execute_advance(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.tasks.complete" => {
                let params: CompleteTaskRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let task = match self.task_service.get_task(&params.task_id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Task {} not found", params.task_id),
                        );
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let cmd = custos_core::CompleteTask {
                    task_id: params.task_id,
                    expected_epoch: task.epoch,
                    summary: params
                        .summary
                        .unwrap_or_else(|| "Completed via Local API".to_string()),
                    evidence_claims: params.evidence_claims,
                };

                match self.task_service.execute_complete(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.tasks.list" => match self.task_service.list_tasks().await {
                Ok(tasks) => match serde_json::to_value(&tasks) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                },
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "v1.tasks.spans" => {
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.task_service.list_spans(task_id).await {
                    Ok(spans) => match serde_json::to_value(&spans) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.create" => {
                let mode: SessionMode = match req.params.get("mode").and_then(|v| v.as_str()) {
                    Some("assisted") => SessionMode::Assisted,
                    _ => SessionMode::Bare,
                };

                let session = self.session_manager.create_session(mode).await;
                match serde_json::to_value(&session) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.get" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.get_session(&session_id).await {
                    Some(s) => match serde_json::to_value(&s) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    None => ApiResponse::error(req.id, format!("Session {session_id} not found")),
                }
            }
            "v1.sessions.promote" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                let contract: TaskContract = match req.params.get("contract") {
                    Some(c) => match serde_json::from_value(c.clone()) {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid contract: {e}"))
                        }
                    },
                    None => return ApiResponse::error(req.id, "Missing contract param"),
                };

                match self.bridge_service.promote(session_id, contract).await {
                    Ok(task) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.bridge.steer" | "v1.sessions.steer" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) => m.to_string(),
                    None => return ApiResponse::error(req.id, "Missing message param"),
                };

                match self
                    .bridge_service
                    .steer(task_id, session_id, message)
                    .await
                {
                    Ok(receipt) => match serde_json::to_value(&receipt) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.list" => match self.session_manager.list_sessions().await {
                Ok(sessions) => match serde_json::to_value(&sessions) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                },
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "v1.sessions.journal" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.get_journal(&session_id).await {
                    Some(journal) => match serde_json::to_value(&journal) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    None => ApiResponse::error(req.id, format!("Session {session_id} not found")),
                }
            }
            "v1.sessions.message" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let role = req
                    .params
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing content param"),
                };

                let res = if role == "assistant" {
                    self.session_manager
                        .append_assistant_message(&session_id, content)
                        .await
                } else {
                    self.session_manager
                        .append_user_message(&session_id, content)
                        .await
                };

                match res {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "status": "appended" }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.bridge.attach" | "v1.sessions.attach" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self
                    .bridge_service
                    .attach(session_id, task_id, AttachMode::Steer)
                    .await
                {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "status": "attached" }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.oi.explain" => {
                let params: crate::custos_local_api::ExplainPlanRequest =
                    match serde_json::from_value(req.params) {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid params: {e}"))
                        }
                    };

                let task_id = params
                    .task_id
                    .unwrap_or_else(|| custos_domain::new_id("task"));
                let mut snapshot = custos_domain::oi::DecisionSnapshot::new(&task_id);
                if let Some(budget) = params.budget_limit_tokens {
                    snapshot.remaining_budget_tokens = budget;
                }
                if let Some(ref contract) = params.contract {
                    snapshot.required_capabilities = contract.required_capabilities.clone();
                }

                match custos_runtime::oi::ExplainService::explain(&snapshot) {
                    Ok(report) => match serde_json::to_value(&report) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKFLOW_START_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: StartRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let mut cmd = custos_domain::StartRunCommand::new(
                    params.task_id,
                    params.actor.unwrap_or_else(|| "daemon_user".into()),
                );
                if let Some(rev) = params.workflow_revision {
                    cmd = cmd.with_workflow_revision(rev);
                }
                if let Some(mode) = params.preferred_mode {
                    cmd = cmd.with_preferred_mode(mode);
                }
                if let Some(harness) = params.harness_id {
                    cmd = cmd.with_harness_id(harness);
                }
                if let Some(root) = params.workspace_root {
                    cmd = cmd.with_workspace_root(root);
                }

                match workflow.start_run(cmd).await {
                    Ok(handle) => match serde_json::to_value(&handle) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKFLOW_CANCEL_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: CancelRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let reason = params
                    .reason
                    .unwrap_or_else(|| "User requested cancellation".into());

                match workflow.request_cancel(&params.run_id, &reason).await {
                    Ok(receipt) => match serde_json::to_value(&receipt) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_CREATE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let params: CreateWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let create_req = CreateWorkspaceRequest {
                    name: params.name,
                    kind: params.kind,
                    path: params.path,
                    lineage: params.lineage,
                    metadata: params.metadata,
                    setup_script: params.setup_script,
                };
                match coordinator.create_workspace(create_req).await {
                    Ok(ws) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_GET => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) => custos_domain::WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id param"),
                };
                match coordinator.get_workspace(&ws_id).await {
                    Ok(Some(ws)) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Workspace {ws_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_LIST => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                match coordinator.list_workspaces().await {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_ARCHIVE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let params: ArchiveWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let ws_id = custos_domain::WorkspaceId::new(params.workspace_id);
                match coordinator
                    .archive_workspace(&ws_id, params.delete_physical)
                    .await
                {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "archived": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_SOURCES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_sources() {
                    Ok(sources) => match serde_json::to_value(&sources) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_SOURCES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let src: SourceRecord = match serde_json::from_value(req.params) {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid source payload: {e}"))
                    }
                };
                let src = match custos_core::research_ingress::source_draft(src) {
                    Ok(src) => src,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_source(&src) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": src.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let source_id = match req.params.get("source_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing source_id parameter"),
                };
                match research.list_anchors_for_source(source_id) {
                    Ok(anchors) => match serde_json::to_value(&anchors) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let anchor: PassageAnchor = match serde_json::from_value(req.params) {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid anchor payload: {e}"))
                    }
                };
                let anchor = match custos_core::research_ingress::passage_draft(anchor) {
                    Ok(anchor) => anchor,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_sources() {
                    Ok(sources) if sources.iter().any(|s| s.id == anchor.source_id) => {}
                    Ok(_) => return ApiResponse::error(req.id, "Passage source does not exist"),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_anchor(&anchor) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anchor.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_claims() {
                    Ok(claims) => match serde_json::to_value(&claims) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim: ResearchClaim = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid claim payload: {e}"))
                    }
                };
                let claim = match custos_core::research_ingress::claim_draft(claim) {
                    Ok(claim) => claim,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_claim(&claim) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": claim.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_runs() {
                    Ok(runs) => match serde_json::to_value(&runs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let run: ResearchExperimentRun = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid run payload: {e}"))
                    }
                };
                let run = match custos_core::research_ingress::experiment_draft(run) {
                    Ok(run) => run,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_runs() {
                    Ok(runs) if runs.iter().any(|existing| existing.run_id == run.run_id) => {
                        return ApiResponse::error(req.id, "Experiment draft id already exists");
                    }
                    Ok(_) => {}
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_run(&run) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "run_id": run.run_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_LINEAGE_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Missing artifact_path parameter"),
                };
                match research.list_artifact_lineage(artifact_path) {
                    Ok(lineage) => match serde_json::to_value(&lineage) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_HANDOFF_CODING => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim_id = match req.params.get("claim_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing claim_id parameter"),
                };
                let claim = match research.list_claims() {
                    Ok(claims) => match claims.into_iter().find(|c| c.id == claim_id) {
                        Some(claim) => claim,
                        None => return ApiResponse::error(req.id, "Claim does not exist"),
                    },
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let title = format!("Investigate research claim: {}", claim.statement);

                let cmd = CreateTask {
                    title,
                    metadata: Some(serde_json::json!({
                        "source": "research_workbench",
                        "claim_id": claim_id,
                        "claim_statement": claim.statement,
                        "claim_grounding": "unverified_client_draft",
                    })),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "handoff_status": "task_created",
                                "task": val,
                            }),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                match research.list_recipes() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe: custos_domain::Recipe = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid recipe payload: {e}"))
                    }
                };
                match research.save_recipe(&recipe) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": recipe.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.get" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.get_recipe(recipe_id) {
                    Ok(Some(recipe)) => match serde_json::to_value(&recipe) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Recipe {recipe_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.executions.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.list_execution_records_for_recipe(recipe_id) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.executions.get" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let execution_id = match req.params.get("execution_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing execution_id parameter"),
                };
                match research.get_execution_record(execution_id) {
                    Ok(Some(record)) => match serde_json::to_value(&record) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("Execution record {execution_id} not found"),
                    ),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.annotations.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let artifact_id = req
                    .params
                    .get("artifact_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let version = req
                    .params
                    .get("version")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                match research.list_annotations_for_artifact(artifact_id, version) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.annotations.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let anno: custos_domain::AnnotationRecord = match serde_json::from_value(req.params)
                {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid annotation payload: {e}"),
                        )
                    }
                };
                match research.save_annotation(&anno) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anno.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_providers() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.save" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("p_custom")
                    .to_string();
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&id)
                    .to_string();
                let service_type = req
                    .params
                    .get("service_type")
                    .or_else(|| req.params.get("provider_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("anthropic")
                    .to_string();
                let raw_api_key = req.params.get("api_key").and_then(|v| v.as_str());
                let masked = if let Some(key) = raw_api_key {
                    if key.len() > 8 {
                        format!("{}••••••••", &key[..6])
                    } else if !key.is_empty() {
                        "••••••••".to_string()
                    } else {
                        "none".to_string()
                    }
                } else {
                    req.params
                        .get("api_key_masked")
                        .and_then(|v| v.as_str())
                        .unwrap_or("none")
                        .to_string()
                };
                let status = req
                    .params
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or(if masked != "none" {
                        "configured"
                    } else {
                        "unconfigured"
                    })
                    .to_string();
                let endpoint_url = req
                    .params
                    .get("endpoint_url")
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let now = chrono::Utc::now().timestamp_millis();

                let cfg = custos_domain::ProviderConfig {
                    id,
                    name,
                    service_type,
                    api_key_masked: masked,
                    status,
                    endpoint_url,
                    created_at: now,
                    updated_at: now,
                };
                match providers_repo.save_provider(&cfg) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": cfg.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_client_keys() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.generate" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Client API Token");
                let key = custos_domain::ClientApiKeyRecord {
                    id: custos_domain::new_id("key"),
                    name: name.to_string(),
                    token: format!("custos_live_sec_{}", custos_domain::new_id("tok")),
                    created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    revoked: false,
                };
                match providers_repo.create_client_key(&key) {
                    Ok(()) => match serde_json::to_value(&key) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.revoke" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let key_id = match req.params.get("key_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing key_id parameter"),
                };
                match providers_repo.revoke_client_key(key_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "revoked": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.llm.status" => {
                let has_env_key = std::env::var("ANTHROPIC_API_KEY").is_ok()
                    || std::env::var("OPENAI_API_KEY").is_ok()
                    || std::env::var("GEMINI_API_KEY").is_ok();

                let configured_provider = if let Some(ref repo) = self.providers {
                    repo.list_providers().ok().and_then(|list| {
                        list.into_iter().find(|p| {
                            (p.status == "configured"
                                || p.status == "active"
                                || p.status == "primary")
                                && p.api_key_masked != "none"
                        })
                    })
                } else {
                    None
                };

                if has_env_key || configured_provider.is_some() {
                    let prov_name = configured_provider
                        .map(|p| p.name)
                        .unwrap_or_else(|| "Environment API Key".to_string());
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": prov_name,
                            "message": "LLM sẵn sàng thực thi",
                        }),
                    )
                } else {
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": false,
                            "active_provider": serde_json::Value::Null,
                            "message": "Chưa có LLM nào được cấu hình hiện tại (No LLMs currently available)",
                        }),
                    )
                }
            }
            METHOD_CAPABILITIES_LIST => {
                let caps = self.list_capabilities();
                match serde_json::to_value(&caps) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_CAPABILITIES_GET => {
                let target_id = req
                    .params
                    .get("capability_id")
                    .or_else(|| req.params.get("id"))
                    .or_else(|| req.params.get("resource_id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let caps = self.list_capabilities();
                        if let Some(c) = caps
                            .into_iter()
                            .find(|cap| cap.id == id || cap.resource_id.as_deref() == Some(id))
                        {
                            match serde_json::to_value(&c) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            }
                        } else {
                            ApiResponse::error(req.id, format!("Capability '{id}' not found"))
                        }
                    }
                    None => ApiResponse::error(req.id, "Missing capability_id parameter"),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}

#[async_trait::async_trait]
impl crate::custos_local_api::ApiTransport for LocalApiDispatcher {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        Ok(self.handle_request(req).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_core::{TaskEvent, TaskStore};
    use custos_domain::{ContinuationPacket, DomainError, Span, Task};
    use std::sync::Mutex;

    struct MockStore {
        tasks: Mutex<Vec<Task>>,
    }

    #[async_trait]
    impl TaskStore for MockStore {
        async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
            let list = self.tasks.lock().unwrap();
            Ok(list.clone())
        }
        async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
            let mut list = self.tasks.lock().unwrap();
            if let Some(pos) = list.iter().position(|t| t.id == task.id) {
                list[pos] = task.clone();
            } else {
                list.push(task.clone());
            }
            Ok(())
        }
        async fn commit_task_event(
            &self,
            _event: &TaskEvent,
            task: &Task,
        ) -> Result<(), DomainError> {
            self.save_task(task).await
        }
        async fn get_task_events(&self, _task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
            Ok(Vec::new())
        }

        async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
            let list = self.tasks.lock().unwrap();
            Ok(list.iter().find(|t| t.id == task_id).cloned())
        }

        async fn get_span(&self, _span_id: &str) -> Result<Option<Span>, DomainError> {
            Ok(None)
        }

        async fn save_span(&self, _span: &Span) -> Result<(), DomainError> {
            Ok(())
        }

        async fn list_spans(&self, _task_id: &str) -> Result<Vec<Span>, DomainError> {
            Ok(Vec::new())
        }

        async fn save_continuation(&self, _packet: &ContinuationPacket) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_continuation(
            &self,
            _task_id: &str,
            _to_span: u32,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            Ok(None)
        }

        async fn get_latest_continuation(
            &self,
            _task_id: &str,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_local_api_dispatcher_create_and_get() {
        let store = Arc::new(MockStore {
            tasks: Mutex::new(Vec::new()),
        });
        let task_service = Arc::new(TaskService::new(store));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service);

        // 1. Create task via API
        let create_req = ApiRequest {
            id: "req_1".into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({
                "title": "Build Local API",
                "metadata": {"source": "cli"}
            }),
        };

        let create_res = dispatcher.handle_request(create_req).await;
        assert!(create_res.error.is_none());
        let created_task: Task = serde_json::from_value(create_res.result.unwrap()).unwrap();
        assert_eq!(created_task.title, "Build Local API");
        assert_eq!(created_task.status, TaskStatus::Draft);

        // 2. Get task via API
        let get_req = ApiRequest {
            id: "req_2".into(),
            method: "v1.tasks.get".into(),
            params: serde_json::json!({
                "task_id": created_task.id
            }),
        };

        let get_res = dispatcher.handle_request(get_req).await;
        assert!(get_res.error.is_none());
        let fetched_task: Task = serde_json::from_value(get_res.result.unwrap()).unwrap();
        assert_eq!(fetched_task.id, created_task.id);
        assert_eq!(fetched_task.title, "Build Local API");

        // 3. Advance task to Queued via API
        let advance_req = ApiRequest {
            id: "req_adv".into(),
            method: "v1.tasks.advance".into(),
            params: serde_json::json!({
                "task_id": created_task.id,
                "target_status": "queued"
            }),
        };
        let adv_res = dispatcher.handle_request(advance_req).await;
        assert!(adv_res.error.is_none());
        let advanced_task: Task = serde_json::from_value(adv_res.result.unwrap()).unwrap();
        assert_eq!(advanced_task.status, TaskStatus::Queued);

        // 3b. Verify advancing directly to Succeeded is REJECTED
        let bad_advance_req = ApiRequest {
            id: "req_bad_adv".into(),
            method: "v1.tasks.advance".into(),
            params: serde_json::json!({
                "task_id": created_task.id,
                "target_status": "succeeded"
            }),
        };
        let bad_adv_res = dispatcher.handle_request(bad_advance_req).await;
        assert!(bad_adv_res.error.is_some());
        assert!(bad_adv_res
            .error
            .unwrap()
            .contains("Cannot advance directly to Succeeded"));

        // 3c. Advance to Running
        let run_req = ApiRequest {
            id: "req_run".into(),
            method: "v1.tasks.advance".into(),
            params: serde_json::json!({
                "task_id": created_task.id,
                "target_status": "running"
            }),
        };
        let run_res = dispatcher.handle_request(run_req).await;
        assert!(run_res.error.is_none());

        // 3d. Complete task via v1.tasks.complete
        let complete_req = ApiRequest {
            id: "req_comp".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": created_task.id,
                "summary": "Finished with proof closure"
            }),
        };
        let comp_res = dispatcher.handle_request(complete_req).await;
        assert!(comp_res.error.is_none());
        let completed_task: Task = serde_json::from_value(comp_res.result.unwrap()).unwrap();
        assert_eq!(completed_task.status, TaskStatus::Succeeded);

        // 3e. List tasks via API
        let list_req = ApiRequest {
            id: "req_list".into(),
            method: "v1.tasks.list".into(),
            params: serde_json::json!({}),
        };
        let list_res = dispatcher.handle_request(list_req).await;
        assert!(list_res.error.is_none());
        let tasks_list: Vec<Task> = serde_json::from_value(list_res.result.unwrap()).unwrap();
        assert_eq!(tasks_list.len(), 1);
        assert_eq!(tasks_list[0].id, created_task.id);

        // 4. Create and Get session via API
        let session_create_req = ApiRequest {
            id: "req_sess".into(),
            method: "v1.sessions.create".into(),
            params: serde_json::json!({"mode": "bare"}),
        };
        let session_res = dispatcher.handle_request(session_create_req).await;
        assert!(session_res.error.is_none());

        // 5. Unknown method
        let invalid_req = ApiRequest {
            id: "req_3".into(),
            method: "v1.unknown.action".into(),
            params: serde_json::json!({}),
        };
        let invalid_res = dispatcher.handle_request(invalid_req).await;
        assert!(invalid_res.error.is_some());
        assert!(invalid_res.error.unwrap().contains("Unknown method"));
    }

    #[tokio::test]
    async fn test_proof_closure_evidence_gating_via_local_api() {
        use custos_domain::{ContractEvidence, EvidenceKind, TaskContract, VerificationClaim};

        let store = Arc::new(MockStore {
            tasks: Mutex::new(Vec::new()),
        });
        let task_service = Arc::new(TaskService::new(store));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service);

        // 1. Create a task with strict contract requiring TestResult and Diff evidence
        let contract = TaskContract {
            pack_id: "engineering".into(),
            name: "Verified Fix".into(),
            description: "Must pass tests and produce verified patch diff".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![
                ContractEvidence {
                    kind: EvidenceKind::TestResult,
                    required: true,
                },
                ContractEvidence {
                    kind: EvidenceKind::Diff,
                    required: true,
                },
            ],
        };

        let create_req = ApiRequest {
            id: "req_c1".into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({
                "title": "Fix memory leak",
                "contract": contract,
            }),
        };

        let create_res = dispatcher.handle_request(create_req).await;
        assert!(create_res.error.is_none());
        let task: Task = serde_json::from_value(create_res.result.unwrap()).unwrap();

        // 2. Advance Draft -> Queued -> Running
        let adv1 = dispatcher
            .handle_request(ApiRequest {
                id: "adv_1".into(),
                method: "v1.tasks.advance".into(),
                params: serde_json::json!({ "task_id": task.id, "target_status": "queued" }),
            })
            .await;
        assert!(adv1.error.is_none());

        let adv2 = dispatcher
            .handle_request(ApiRequest {
                id: "adv_2".into(),
                method: "v1.tasks.advance".into(),
                params: serde_json::json!({ "task_id": task.id, "target_status": "running" }),
            })
            .await;
        assert!(adv2.error.is_none());

        // 3. Attempt completion WITHOUT claims -> MUST FAIL (Proof-closure violation)
        let fail_comp1 = dispatcher
            .handle_request(ApiRequest {
                id: "comp_fail1".into(),
                method: "v1.tasks.complete".into(),
                params: serde_json::json!({
                    "task_id": task.id,
                    "summary": "Try complete without proof",
                    "evidence_claims": []
                }),
            })
            .await;
        assert!(fail_comp1.error.is_some());
        assert!(fail_comp1
            .error
            .unwrap()
            .contains("Proof-closure violation"));

        // 4. Attempt completion with failing claim -> MUST FAIL
        let failing_claim = VerificationClaim::new(
            task.id.clone(),
            "Tests failed".into(),
            "command_exit_code".into(),
            false,
            serde_json::json!({"exit_code": 1}),
        );
        let fail_comp2 = dispatcher
            .handle_request(ApiRequest {
                id: "comp_fail2".into(),
                method: "v1.tasks.complete".into(),
                params: serde_json::json!({
                    "task_id": task.id,
                    "summary": "Try complete with failing claim",
                    "evidence_claims": [failing_claim]
                }),
            })
            .await;
        assert!(fail_comp2.error.is_some());
        assert!(fail_comp2
            .error
            .unwrap()
            .contains("Proof-closure violation"));

        // 5. Complete WITH passing claims for all required evidence -> MUST SUCCEED
        let test_claim = VerificationClaim::new(
            task.id.clone(),
            "Unit tests passed with exit code 0".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        );
        let diff_claim = VerificationClaim::new(
            task.id.clone(),
            "Simulated patch preview generated".into(),
            "patch_preview".into(),
            true,
            serde_json::json!({"simulated": true, "diff_digest": "sha256:abc12345"}),
        );

        let success_comp = dispatcher
            .handle_request(ApiRequest {
                id: "comp_ok".into(),
                method: "v1.tasks.complete".into(),
                params: serde_json::json!({
                    "task_id": task.id,
                    "summary": "All proof closed successfully",
                    "evidence_claims": [test_claim, diff_claim]
                }),
            })
            .await;
        assert!(
            success_comp.error.is_none(),
            "Error: {:?}",
            success_comp.error
        );
        let completed: Task = serde_json::from_value(success_comp.result.unwrap()).unwrap();
        assert_eq!(completed.status, TaskStatus::Succeeded);
    }

    #[tokio::test]
    async fn test_workflow_start_and_cancel_run_api() {
        let store = Arc::new(MockStore {
            tasks: Mutex::new(Vec::new()),
        });
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let workflow = Arc::new(custos_runtime::workflow::TaskRuntime::new());

        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_workflow(workflow);

        // 1. Start run via API
        let start_req = ApiRequest {
            id: "req_run_1".into(),
            method: "v1.workflow.start_run".into(),
            params: serde_json::json!({
                "task_id": "task_api_workflow_1",
                "actor": "tester",
                "preferred_mode": "model"
            }),
        };
        let start_resp = dispatcher.handle_request(start_req).await;
        assert!(start_resp.error.is_none(), "Error: {:?}", start_resp.error);
        let handle: custos_domain::RunHandle =
            serde_json::from_value(start_resp.result.unwrap()).unwrap();
        assert_eq!(handle.task_id, "task_api_workflow_1");
        assert_eq!(handle.status, custos_domain::RunStatus::Active);

        // 2. Double-dispatch via API on same task must fail with Conflict
        let start_dup_req = ApiRequest {
            id: "req_run_dup".into(),
            method: "v1.workflow.start_run".into(),
            params: serde_json::json!({
                "task_id": "task_api_workflow_1",
                "actor": "second_caller"
            }),
        };
        let dup_resp = dispatcher.handle_request(start_dup_req).await;
        assert!(dup_resp.error.is_some());
        assert!(dup_resp.error.unwrap().contains("already claimed"));

        // 3. Cancel run via API
        let cancel_req = ApiRequest {
            id: "req_cancel_1".into(),
            method: "v1.workflow.cancel_run".into(),
            params: serde_json::json!({
                "run_id": handle.run_id,
                "reason": "Test cancel via API"
            }),
        };
        let cancel_resp = dispatcher.handle_request(cancel_req).await;
        assert!(
            cancel_resp.error.is_none(),
            "Error: {:?}",
            cancel_resp.error
        );
        let receipt: custos_domain::CancelReceipt =
            serde_json::from_value(cancel_resp.result.unwrap()).unwrap();
        assert_eq!(receipt.run_id, handle.run_id);
        assert_eq!(receipt.reason, "Test cancel via API");
    }

    #[tokio::test]
    async fn test_workspace_api_dispatch_lifecycle() {
        use custos_adapters::workspace::LocalWorkspaceProvider;
        use custos_domain::{ExecutionWorkspace, WorkspaceStatus};
        use custos_persistence::SqliteTaskStore;
        use custos_runtime::workspace::WorkspaceCoordinator;

        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let workspace_provider = Arc::new(LocalWorkspaceProvider::new());
        let coordinator = Arc::new(WorkspaceCoordinator::new(store.clone(), workspace_provider));

        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_workspace(coordinator);

        let tmp = tempfile::tempdir().unwrap();
        let ws_path = tmp.path().join("api_test_ws");
        let path_str = ws_path.to_str().unwrap().to_string();

        // 1. Create Workspace
        let create_req = ApiRequest {
            id: "req_ws_1".into(),
            method: METHOD_WORKSPACES_CREATE.into(),
            params: serde_json::json!({
                "name": "api-workspace",
                "kind": {
                    "type": "folder",
                    "path": path_str
                },
                "path": path_str,
                "metadata": {
                    "domain": "assistant"
                }
            }),
        };
        let create_resp = dispatcher.handle_request(create_req).await;
        assert!(
            create_resp.error.is_none(),
            "Error: {:?}",
            create_resp.error
        );
        let created_ws: ExecutionWorkspace =
            serde_json::from_value(create_resp.result.unwrap()).unwrap();
        assert_eq!(created_ws.name, "api-workspace");
        assert_eq!(created_ws.status, WorkspaceStatus::Ready);
        assert!(ws_path.exists());

        // 2. Get Workspace
        let get_req = ApiRequest {
            id: "req_ws_2".into(),
            method: METHOD_WORKSPACES_GET.into(),
            params: serde_json::json!({
                "workspace_id": created_ws.id.as_str()
            }),
        };
        let get_resp = dispatcher.handle_request(get_req).await;
        assert!(get_resp.error.is_none());
        let loaded_ws: ExecutionWorkspace =
            serde_json::from_value(get_resp.result.unwrap()).unwrap();
        assert_eq!(loaded_ws.id, created_ws.id);

        // 3. List Workspaces
        let list_req = ApiRequest {
            id: "req_ws_3".into(),
            method: METHOD_WORKSPACES_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_resp = dispatcher.handle_request(list_req).await;
        assert!(list_resp.error.is_none());
        let workspaces: Vec<ExecutionWorkspace> =
            serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert_eq!(workspaces.len(), 1);

        // 4. Archive Workspace (with delete_physical: true)
        let archive_req = ApiRequest {
            id: "req_ws_4".into(),
            method: METHOD_WORKSPACES_ARCHIVE.into(),
            params: serde_json::json!({
                "workspace_id": created_ws.id.as_str(),
                "delete_physical": true
            }),
        };
        let archive_resp = dispatcher.handle_request(archive_req).await;
        assert!(archive_resp.error.is_none());
        assert!(!ws_path.exists());
    }

    #[tokio::test]
    async fn test_research_api_dispatch_lifecycle() {
        use custos_adapters::workspace::LocalWorkspaceProvider;
        use custos_persistence::SqliteTaskStore;
        use custos_runtime::workspace::WorkspaceCoordinator;

        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));

        let workspace = Arc::new(WorkspaceCoordinator::new(
            store.clone(),
            Arc::new(LocalWorkspaceProvider::new()),
        ));
        let dispatcher = LocalApiDispatcher::new(
            task_service.clone(),
            session_manager.clone(),
            bridge_service.clone(),
        )
        .with_workspace(workspace)
        .with_research(Arc::new(store.research().clone()));

        // Research records and OrCa-style workspace lifecycle share one daemon,
        // but neither owns the other's canonical state.
        let temp = tempfile::tempdir().unwrap();
        let research_dir = temp.path().join("study");
        let create_workspace = dispatcher
            .handle_request(ApiRequest {
                id: "req_research_ws".into(),
                method: METHOD_WORKSPACES_CREATE.into(),
                params: serde_json::json!({
                    "name": "study",
                    "kind": { "type": "folder", "path": research_dir },
                    "path": research_dir,
                    "metadata": { "domain": "research" }
                }),
            })
            .await;
        assert!(
            create_workspace.is_success(),
            "Workspace failed: {:?}",
            create_workspace.error
        );
        assert!(research_dir.exists());

        // 1. Save & List Source
        let save_src_req = ApiRequest {
            id: "req_s1".into(),
            method: METHOD_RESEARCH_SOURCES_SAVE.into(),
            params: serde_json::json!({
                "id": "src_paper_01",
                "source_type": "paper",
                "title": "Quantum Error Mitigation",
                "doi": "10.1038/s41586-023-06096-3",
                "authors": ["Kim et al."],
                "year": 2023,
                "content_hash": "hash_qem",
                "local_path": "/papers/qem.pdf",
                "verified": true,
                "abstract_text": "Evidence for the utility of quantum computing before fault tolerance.",
                "created_at": 1700000000
            }),
        };
        let save_src_resp = dispatcher.handle_request(save_src_req).await;
        assert!(
            save_src_resp.is_success(),
            "Failed to save source: {:?}",
            save_src_resp.error
        );

        let list_src_req = ApiRequest {
            id: "req_s2".into(),
            method: METHOD_RESEARCH_SOURCES_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_src_resp = dispatcher.handle_request(list_src_req).await;
        assert!(list_src_resp.is_success());
        let sources: Vec<SourceRecord> =
            serde_json::from_value(list_src_resp.result.unwrap()).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Quantum Error Mitigation");
        assert!(!sources[0].verified, "client cannot certify a source");

        // 2. Save & List Passage Anchor
        let save_anc_req = ApiRequest {
            id: "req_a1".into(),
            method: METHOD_RESEARCH_ANCHORS_SAVE.into(),
            params: serde_json::json!({
                "id": "anc_01",
                "source_id": "src_paper_01",
                "source_title": "Quantum Error Mitigation",
                "section_title": "Zero-Noise Extrapolation",
                "page_number": 4,
                "start_offset": 500,
                "end_offset": 620,
                "exact_text": "ZNE scales circuit noise artificially by pulse stretching.",
                "passage_hash": "hash_anchor_zne"
            }),
        };
        let save_anc_resp = dispatcher.handle_request(save_anc_req).await;
        assert!(save_anc_resp.is_success());

        let list_anc_req = ApiRequest {
            id: "req_a2".into(),
            method: METHOD_RESEARCH_ANCHORS_LIST.into(),
            params: serde_json::json!({ "source_id": "src_paper_01" }),
        };
        let list_anc_resp = dispatcher.handle_request(list_anc_req).await;
        assert!(list_anc_resp.is_success());
        let anchors: Vec<PassageAnchor> =
            serde_json::from_value(list_anc_resp.result.unwrap()).unwrap();
        assert_eq!(anchors.len(), 1);
        assert_eq!(
            anchors[0].exact_text,
            "ZNE scales circuit noise artificially by pulse stretching."
        );
        assert!(anchors[0].passage_hash.starts_with("sha256:"));
        assert_ne!(anchors[0].passage_hash, "hash_anchor_zne");

        // 3. Save & List Claims
        let save_claim_req = ApiRequest {
            id: "req_c1".into(),
            method: METHOD_RESEARCH_CLAIMS_SAVE.into(),
            params: serde_json::json!({
                "id": "claim_zne_01",
                "statement": "Zero noise extrapolation bounds expectation value bias within 2%",
                "level": "l1_cited",
                "confidence_score": 0.92,
                "invariants": ["abs(bias) <= 0.02"],
                "evidence_links": [{
                    "passage_anchor_id": "anc_01",
                    "source_title": "Quantum Error Mitigation",
                    "exact_text": "ZNE scales circuit noise artificially by pulse stretching.",
                    "relation": "supports",
                    "rationale": "Empirical curve fitting verified",
                    "verified_by": "expert_review"
                }],
                "created_at": 1700000100,
                "sealed_proof_uri": null
            }),
        };
        let save_claim_resp = dispatcher.handle_request(save_claim_req).await;
        assert!(
            save_claim_resp.is_success(),
            "Failed to save claim: {:?}",
            save_claim_resp.error
        );

        let list_claim_req = ApiRequest {
            id: "req_c2".into(),
            method: METHOD_RESEARCH_CLAIMS_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_claim_resp = dispatcher.handle_request(list_claim_req).await;
        assert!(list_claim_resp.is_success());
        let claims: Vec<ResearchClaim> =
            serde_json::from_value(list_claim_resp.result.unwrap()).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, "claim_zne_01");
        assert_eq!(
            claims[0].level,
            custos_domain::ClaimGroundingLevel::L0Ungrounded
        );
        assert_eq!(
            claims[0].evidence_links[0].verified_by,
            "unreviewed_client_proposal"
        );

        // 4. Recipe definitions and observed executions stay separate. The API
        // exposes the ledger read-only; it does not execute the recipe command.
        let recipe = custos_domain::Recipe::new(
            "QEM reproduction",
            "python reproduce.py --seed 7",
            custos_domain::EnvironmentSpec::new(),
        );
        store.research().save_recipe(&recipe).unwrap();
        let mut execution =
            custos_domain::ExecutionRecord::new(&recipe.id).with_session_id("research_session_01");
        execution.transition(custos_domain::ExecutionRecordStatus::Completed);
        execution.exit_code = Some(0);
        store.research().save_execution_record(&execution).unwrap();

        let list_executions = dispatcher
            .handle_request(ApiRequest {
                id: "req_exec_list".into(),
                method: "v1.research.executions.list".into(),
                params: serde_json::json!({ "recipe_id": recipe.id }),
            })
            .await;
        assert!(list_executions.is_success());
        let records: Vec<custos_domain::ExecutionRecord> =
            serde_json::from_value(list_executions.result.unwrap()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, execution.id);

        let get_execution = dispatcher
            .handle_request(ApiRequest {
                id: "req_exec_get".into(),
                method: "v1.research.executions.get".into(),
                params: serde_json::json!({ "execution_id": execution.id }),
            })
            .await;
        assert!(get_execution.is_success());

        // 5. Handoff Claim to Coding Task
        let handoff_req = ApiRequest {
            id: "req_h1".into(),
            method: METHOD_RESEARCH_HANDOFF_CODING.into(),
            params: serde_json::json!({
                "claim_id": "claim_zne_01",
                "statement": claims[0].statement
            }),
        };
        let handoff_resp = dispatcher.handle_request(handoff_req).await;
        assert!(
            handoff_resp.is_success(),
            "Failed handoff: {:?}",
            handoff_resp.error
        );
        let handoff_val = handoff_resp.result.unwrap();
        assert_eq!(handoff_val["handoff_status"], "task_created");
        assert!(handoff_val["task"]["id"].is_string());

        let forged_handoff = dispatcher.handle_request(ApiRequest {
            id: "req_h2".into(),
            method: METHOD_RESEARCH_HANDOFF_CODING.into(),
            params: serde_json::json!({"claim_id": "missing", "statement": "Trusted by client"}),
        }).await;
        assert!(forged_handoff.error.is_some());
    }

    #[tokio::test]
    async fn test_capabilities_api_dispatch() {
        let store = Arc::new(MockStore {
            tasks: Mutex::new(Vec::new()),
        });
        let task_service = Arc::new(TaskService::new(store));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service);

        // 1. List all capabilities
        let list_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_caps_1".into(),
                method: METHOD_CAPABILITIES_LIST.into(),
                params: serde_json::Value::Null,
            })
            .await;
        assert!(list_resp.is_success());

        let caps: Vec<custos_domain::CapabilityDescriptor> =
            serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert!(!caps.is_empty());

        // Kernel task engine is available
        let tasks_cap = caps.iter().find(|c| c.id == "tasks.core").unwrap();
        assert!(tasks_cap.status.is_available());

        // PTY terminal is truthfully unavailable with roadmap rationale
        let pty_cap = caps.iter().find(|c| c.id == "compute.pty").unwrap();
        assert!(!pty_cap.status.is_available());
        assert_eq!(pty_cap.resource_id.as_deref(), Some("terminal"));
        assert!(pty_cap.status.reason().unwrap().contains("Roadmap Step 3"));

        // 2. Query capability by capability_id
        let get_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_caps_2".into(),
                method: METHOD_CAPABILITIES_GET.into(),
                params: serde_json::json!({ "capability_id": "compute.pty" }),
            })
            .await;
        assert!(get_resp.is_success());
        let cap: custos_domain::CapabilityDescriptor =
            serde_json::from_value(get_resp.result.unwrap()).unwrap();
        assert_eq!(cap.id, "compute.pty");

        // 3. Query capability by resource_id
        let get_res_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_caps_3".into(),
                method: METHOD_CAPABILITIES_GET.into(),
                params: serde_json::json!({ "resource_id": "terminal" }),
            })
            .await;
        assert!(get_res_resp.is_success());
        let res_cap: custos_domain::CapabilityDescriptor =
            serde_json::from_value(get_res_resp.result.unwrap()).unwrap();
        assert_eq!(res_cap.id, "compute.pty");

        // 4. Missing capability returns error
        let missing_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_caps_4".into(),
                method: METHOD_CAPABILITIES_GET.into(),
                params: serde_json::json!({ "capability_id": "nonexistent" }),
            })
            .await;
        assert!(missing_resp.error.is_some());
    }
}
