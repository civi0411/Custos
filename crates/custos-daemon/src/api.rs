//! Custos Local API Contracts & Dispatcher
//!
//! Protocol types and dispatcher for IPC communication between CLI / UI / VS Code and custosd.

use std::sync::Arc;

pub use crate::custos_local_api::{
    AdvanceTaskRequest, ApiRequest, ApiResponse, ArchiveWorkspaceApiRequest,
    ArtifactDetailResponse, CancelHarnessRunRequest, CancelRunRequest, CancelTaskRequest,
    CompleteTaskRequest, CreateTaskRequest, CreateWorkspaceApiRequest, GetCapabilityApiRequest,
    GetHarnessRequest, GetWorkspaceApiRequest, InspectWorkspaceDirtyApiRequest,
    OAuthAuthorizeParams, OAuthExchangeParams, OAuthRefreshParams, RecoverWorkspaceApiRequest,
    RunNativeHarnessRequest, SaveNoteParams, StartRunRequest, SteerHarnessRunRequest,
    METHOD_ARTIFACTS_GET, METHOD_ARTIFACTS_LINEAGE_GRAPH, METHOD_ARTIFACTS_LIST,
    METHOD_ARTIFACTS_RECORD_LINEAGE, METHOD_AUTOMATION_JOBS_CREATE, METHOD_AUTOMATION_JOBS_LIST,
    METHOD_AUTOMATION_JOBS_RUN, METHOD_BROWSER_SESSIONS_CREATE, METHOD_BROWSER_SESSIONS_LIST,
    METHOD_BROWSER_TABS_CLOSE, METHOD_BROWSER_TABS_CREATE, METHOD_BROWSER_TABS_LIST,
    METHOD_BROWSER_TABS_NAVIGATE, METHOD_BROWSER_TABS_SNAPSHOT, METHOD_CAPABILITIES_GET,
    METHOD_CAPABILITIES_LIST, METHOD_FLEET_EXEC, METHOD_FLEET_HOSTS_LIST, METHOD_FLEET_HOSTS_PING,
    METHOD_FLEET_HOSTS_REGISTER, METHOD_HARNESS_CANCEL, METHOD_HARNESS_GET, METHOD_HARNESS_LIST,
    METHOD_HARNESS_RUN_NATIVE, METHOD_HARNESS_STEER, METHOD_MODELS_CATALOG, METHOD_MODELS_PRICING,
    METHOD_MODELS_PROBE, METHOD_NOTEBOOK_CELLS_LIST, METHOD_NOTEBOOK_CELLS_SAVE,
    METHOD_NOTEBOOK_EXECUTE, METHOD_NOTEBOOK_INTERRUPT, METHOD_NOTEBOOK_RESET,
    METHOD_NOTEBOOK_STATUS, METHOD_NOTES_GET, METHOD_NOTES_HISTORY, METHOD_NOTES_LIST,
    METHOD_NOTES_SAVE, METHOD_OAUTH_AUTHORIZE, METHOD_OAUTH_DELETE, METHOD_OAUTH_EXCHANGE,
    METHOD_OAUTH_GET, METHOD_OAUTH_REFRESH, METHOD_OAUTH_STATUS, METHOD_PROVIDERS_DELETE,
    METHOD_PROVIDERS_GET, METHOD_PROVIDERS_LIST, METHOD_PROVIDERS_SAVE,
    METHOD_RESEARCH_ANCHORS_LIST, METHOD_RESEARCH_ANCHORS_SAVE, METHOD_RESEARCH_CLAIMS_LIST,
    METHOD_RESEARCH_CLAIMS_SAVE, METHOD_RESEARCH_HANDOFF_CODING, METHOD_RESEARCH_LINEAGE_LIST,
    METHOD_RESEARCH_RUNS_LIST, METHOD_RESEARCH_RUNS_SAVE, METHOD_RESEARCH_SOURCES_LIST,
    METHOD_RESEARCH_SOURCES_SAVE, METHOD_REVIEWS_GET, METHOD_REVIEWS_LIST,
    METHOD_REVIEWS_MARK_STALE, METHOD_REVIEWS_RECORD, METHOD_SESSIONS_DELETE,
    METHOD_SYNTHESIS_HANDOFF_EXECUTE, METHOD_SYNTHESIS_PROPOSALS_GET,
    METHOD_SYNTHESIS_PROPOSALS_LIST, METHOD_SYNTHESIS_PROPOSALS_SAVE, METHOD_TERMINAL_GET,
    METHOD_TERMINAL_LIST, METHOD_TERMINAL_READ, METHOD_TERMINAL_RESIZE, METHOD_TERMINAL_SPAWN,
    METHOD_TERMINAL_TERMINATE, METHOD_TERMINAL_WRITE, METHOD_WORKFLOW_CANCEL_RUN,
    METHOD_WORKFLOW_START_RUN, METHOD_WORKSPACES_ARCHIVE, METHOD_WORKSPACES_CREATE,
    METHOD_WORKSPACES_GET, METHOD_WORKSPACES_INSPECT_DIRTY, METHOD_WORKSPACES_LIST,
    METHOD_WORKSPACES_RECOVER, METHOD_WORKSPACE_DIFF, METHOD_WORKSPACE_FILES_READ,
    METHOD_WORKSPACE_FILES_TREE, METHOD_WORKSPACE_FILES_WRITE, METHOD_WORKSPACE_FILE_DIFF,
    METHOD_WORKSPACE_GIT_DISCARD, METHOD_WORKSPACE_GIT_STAGE, METHOD_WORKSPACE_GIT_UNSTAGE,
};
use custos_adapters::harness::HarnessRegistry;
use custos_bridge::{AttachMode, BridgePort, BridgeService};
use custos_core::contracts::terminal::TerminalPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_core::{AdvanceTask, CancelTask, CreateTask, TaskService};
use custos_domain::{
    ArtifactLineageNode, BrowserSession, BrowserTab, CapabilityDescriptor, CapabilityGroup,
    ClaimGroundingLevel, ClaimHandoffSummary, CreateHeadlessJobParams, ExecuteCellParams,
    HandoffToCodingParams, HandoffToCodingResult, HeadlessAutomationJob, HeadlessJobStatus,
    NoteRecord, NotebookCell, PassageAnchor, Recipe, RecipeHandoffSummary, RecordReviewParams,
    RegisterHostParams, RemoteHostNode, ResearchClaim, ResearchExperimentRun,
    ResearchSynthesisProposal, ReviewStatus, ReviewerRecord, SaveSynthesisProposalParams,
    SessionId, SessionMode, SourceRecord, SshAuthMethod, TaskContract, TaskStatus,
};
use custos_persistence::{FleetAutomationRepository, ProviderRepository, ResearchRepository};
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::{
    CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceFilesCoordinator,
};
use custos_runtime::{PythonKernelCoordinator, TerminalCoordinator};

fn default_model_probe_base_url(provider_type: &str) -> Option<&'static str> {
    match provider_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => Some("https://api.openai.com/v1"),
        "anthropic" | "claude" => Some("https://api.anthropic.com/v1"),
        "gemini" | "google" => Some("https://generativelanguage.googleapis.com"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "local" | "ollama" => Some("http://localhost:11434"),
        _ => None,
    }
}

/// Local API Dispatcher wrapping TaskService, SessionManager, BridgeService, WorkflowPort, ResearchRepository, ProviderRepository, TerminalCoordinator, and WorkspaceFilesPort for IPC callers.
pub struct LocalApiDispatcher {
    task_service: Arc<TaskService>,
    session_manager: Arc<SessionManager>,
    bridge_service: Arc<BridgeService>,
    workflow: Option<Arc<dyn WorkflowPort>>,
    workspace: Option<Arc<WorkspaceCoordinator>>,
    research: Option<Arc<ResearchRepository>>,
    providers: Option<Arc<ProviderRepository>>,
    terminal: Arc<TerminalCoordinator>,
    files: Arc<dyn WorkspaceFilesPort>,
    harnesses: Arc<HarnessRegistry>,
    python_kernel: Option<Arc<PythonKernelCoordinator>>,
    fleet_automation: Option<Arc<FleetAutomationRepository>>,
    model_catalog: custos_adapters::providers::catalog::ModelCatalogService,
    oauth_callback_server: Option<Arc<crate::oauth_callback_server::OAuthCallbackServer>>,
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
            terminal: Arc::new(TerminalCoordinator::new()),
            files: Arc::new(WorkspaceFilesCoordinator::new()),
            harnesses: Arc::new(HarnessRegistry::new()),
            python_kernel: Some(Arc::new(PythonKernelCoordinator::new())),
            fleet_automation: None,
            model_catalog: custos_adapters::providers::catalog::ModelCatalogService::new(),
            oauth_callback_server: None,
        }
    }

    pub fn with_python_kernel(mut self, python_kernel: Arc<PythonKernelCoordinator>) -> Self {
        self.python_kernel = Some(python_kernel);
        self
    }

    pub fn with_harnesses(mut self, harnesses: Arc<HarnessRegistry>) -> Self {
        self.harnesses = harnesses;
        self
    }

    pub fn with_files(mut self, files: Arc<dyn WorkspaceFilesPort>) -> Self {
        self.files = files;
        self
    }

    pub fn with_terminal(mut self, terminal: Arc<TerminalCoordinator>) -> Self {
        self.terminal = terminal;
        self
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

    pub fn with_fleet_automation(
        mut self,
        fleet_automation: Arc<FleetAutomationRepository>,
    ) -> Self {
        self.fleet_automation = Some(fleet_automation);
        self
    }

    pub fn with_oauth_callback_server(
        mut self,
        oauth_callback_server: Arc<crate::oauth_callback_server::OAuthCallbackServer>,
    ) -> Self {
        self.oauth_callback_server = Some(oauth_callback_server);
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
                    vec!["create", "list", "get", "archive", "inspect_dirty", "recover"],
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
                    vec!["list", "get", "record_lineage", "lineage.list", "lineage.graph", "annotations.list", "annotations.save"],
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
            // Terminal PTY (Native PTY stream scoped to execution workspace)
            CapabilityDescriptor::available(
                "compute.pty",
                "Terminal",
                CapabilityGroup::Compute,
                "Bounded PTY streams scoped to an execution workspace.",
                Some("terminal"),
                vec!["spawn", "write", "resize", "read", "terminate", "list", "get"],
            ),
            CapabilityDescriptor::available(
                "code.files",
                "Files",
                CapabilityGroup::Code,
                "Repository tree, file buffers, and source anchor inspection.",
                Some("files"),
                vec!["tree", "read", "write"],
            ),
            CapabilityDescriptor::available(
                "code.changes",
                "Changes",
                CapabilityGroup::Code,
                "Diff review, patch proposal, annotations, and Git stage decisions.",
                Some("changes"),
                vec!["diff", "file_diff", "stage", "unstage", "discard"],
            ),
            if self.python_kernel.is_some() {
                CapabilityDescriptor::available(
                    "compute.notebook",
                    "Notebook",
                    CapabilityGroup::Compute,
                    "Authorized kernels, code cells, and reproducible compute epochs.",
                    Some("experiments"),
                    vec!["cells.list", "cells.save", "execute", "interrupt", "reset", "status"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "compute.notebook",
                    "Notebook",
                    CapabilityGroup::Compute,
                    "Authorized kernels, code cells, and reproducible compute epochs.",
                    Some("experiments"),
                    "PythonKernelCoordinator is not configured on this daemon instance.",
                )
            },
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "evidence.criteria",
                    "Evidence",
                    CapabilityGroup::Evidence,
                    "Criteria, receipts, verifier records, and freshness status.",
                    Some("evidence"),
                    vec!["reviews.list", "reviews.record", "reviews.get", "reviews.mark_stale"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "evidence.criteria",
                    "Evidence",
                    CapabilityGroup::Evidence,
                    "Criteria, receipts, verifier records, and freshness status.",
                    Some("evidence"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            CapabilityDescriptor::available(
                "coordination.kanban",
                "Agents",
                CapabilityGroup::Coordination,
                "Worker runs, native coding harnesses (Claude Code, Codex, Goose), and agent loop dispatch.",
                Some("kanban"),
                vec!["list_harnesses", "get_harness", "run_native", "dispatch", "cancel", "steer"],
            ),
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "synthesis.handoff",
                    "Synthesis Handoff",
                    CapabilityGroup::Coordination,
                    "Research claim synthesis, proposal drafting, recipe conversion, and Coding handoff.",
                    Some("synthesis"),
                    vec!["proposals.list", "proposals.save", "proposals.get", "handoff.execute"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "synthesis.handoff",
                    "Synthesis Handoff",
                    CapabilityGroup::Coordination,
                    "Research claim synthesis, proposal drafting, recipe conversion, and Coding handoff.",
                    Some("synthesis"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                CapabilityDescriptor::degraded(
                    "browser.tabs",
                    "Browser",
                    CapabilityGroup::Browser,
                    "Persisted browser session and tab intents.",
                    Some("browser"),
                    "Browser execution adapter is not connected; navigation and snapshots are refused.",
                    vec!["sessions.list", "sessions.create", "tabs.list", "tabs.create", "tabs.close"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "browser.tabs",
                    "Browser",
                    CapabilityGroup::Browser,
                    "Scoped browsing, DOM snapshots, network inspector, and page capture.",
                    Some("browser"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                CapabilityDescriptor::degraded(
                    "remote.fleet",
                    "Remote Fleet",
                    CapabilityGroup::Compute,
                    "Persisted SSH host inventory.",
                    Some("fleet"),
                    "SSH transport is not connected; probes and remote execution are refused.",
                    vec!["hosts.list", "hosts.register"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "remote.fleet",
                    "Remote Fleet",
                    CapabilityGroup::Compute,
                    "SSH remote host inventory, ping latency probes, and distributed node execution.",
                    Some("fleet"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                CapabilityDescriptor::degraded(
                    "automation.headless",
                    "Automation",
                    CapabilityGroup::Coordination,
                    "Persisted headless job definitions.",
                    Some("automation"),
                    "Automation executor and standing-grant checks are not connected; job execution is refused.",
                    vec!["jobs.list", "jobs.create"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "automation.headless",
                    "Automation",
                    CapabilityGroup::Coordination,
                    "Headless job schedules, batch tasks, and verification execution receipts.",
                    Some("automation"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "personal.notes",
                    "Notes",
                    CapabilityGroup::Personal,
                    "Task notes, scratchpads, and Markdown knowledge capture with version history.",
                    Some("notes"),
                    vec!["list", "get", "save", "history"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "personal.notes",
                    "Notes",
                    CapabilityGroup::Personal,
                    "Task notes, scratchpads, and Markdown knowledge capture with version history.",
                    Some("notes"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
        ]
    }

    async fn resolve_workspace_for_dispatch(
        &self,
        req_id: &str,
        ws_id_str: &str,
    ) -> Result<custos_domain::ExecutionWorkspace, ApiResponse> {
        let coordinator = match self.workspace.as_ref() {
            Some(c) => c,
            None => {
                return Err(ApiResponse::error(
                    req_id,
                    "WorkspaceCoordinator not configured on daemon",
                ))
            }
        };
        let ws_id = custos_domain::WorkspaceId::new(ws_id_str);
        match coordinator.get_workspace(&ws_id).await {
            Ok(Some(ws)) => Ok(ws),
            Ok(None) => Err(ApiResponse::error(
                req_id,
                format!("Workspace {ws_id_str} not found"),
            )),
            Err(e) => Err(ApiResponse::error(
                req_id,
                format!("Workspace {ws_id_str} lookup failed: {e}"),
            )),
        }
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
            METHOD_SESSIONS_DELETE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.delete_session(&session_id).await {
                    Ok(deleted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "deleted": deleted,
                            "session_id": session_id.0,
                        }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
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

                if let Some(ref sid) = params.session_id {
                    let sess_opt = self.session_manager.get_session(&SessionId(sid.to_string())).await;
                    if sess_opt.is_none() {
                        return ApiResponse::error(
                            req.id,
                            format!("Admission gate refused unverified session_id '{sid}': session does not exist"),
                        );
                    }
                }

                let actor = match params.actor {
                    Some(ref a) if !a.trim().is_empty() => a.trim().to_string(),
                    _ => "daemon_user".into(),
                };

                let mut cmd = custos_domain::StartRunCommand::new(params.task_id, actor);
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
                if let Some(ref session_id) = params.session_id {
                    cmd = cmd.with_session_id(session_id.clone());
                }
                if let Some(ref turn_id) = params.turn_id {
                    cmd = cmd.with_turn_id(turn_id.clone());
                }
                if let Some(ref m) = params.model {
                    cmd = cmd.with_model(m.clone());
                }

                let prompt = if let Some(p) = params.prompt {
                    Some(p)
                } else if let Some(ref sid) = params.session_id {
                    if let Some(journal) = self.session_manager.get_journal(&SessionId(sid.to_string())).await {
                        journal.entries.iter().rev()
                            .find(|e| e.entry_type == "user_message")
                            .map(|e| e.entry_data.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(p) = prompt {
                    cmd = cmd.with_prompt(p);
                }

                match workflow.start_run(cmd).await {
                    Ok(handle) => {
                        if let (Some(ref sid), Some(ref out)) = (params.session_id.as_ref(), handle.output.as_ref()) {
                            let _ = self
                                .session_manager
                                .append_assistant_message(&SessionId(sid.to_string()), out)
                                .await;
                        }
                        match serde_json::to_value(&handle) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
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
                let owner_task_id = req
                    .params
                    .get("owner_task_id")
                    .or_else(|| req.params.get("task_id"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let params: CreateWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let create_req = CreateWorkspaceRequest {
                    name: params.name,
                    kind: params.kind,
                    path: params.path,
                    lineage: params.lineage,
                    owner_task_id,
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
                    .archive_workspace_with_force(&ws_id, params.delete_physical, params.force)
                    .await
                {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "archived": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_INSPECT_DIRTY => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) => custos_domain::WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                match coordinator.inspect_dirty(&ws_id).await {
                    Ok(dirty) => match serde_json::to_value(&dirty) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_RECOVER => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let target_id = req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let ws_id = custos_domain::WorkspaceId::new(id);
                        match coordinator.recover_workspace(&ws_id).await {
                            Ok(ws) => match serde_json::to_value(&ws) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            },
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => {
                        match coordinator.reconcile_all().await {
                            Ok(recovered) => match serde_json::to_value(&recovered) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            },
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
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
            METHOD_ARTIFACTS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_all_artifacts() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing or empty artifact_path parameter"),
                };
                let versions = match research.list_artifact_lineage(artifact_path) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let annotations = match research.list_annotations_for_artifact(artifact_path, None) {
                    Ok(a) => a,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let (latest_version, latest_content_hash) = if let Some(latest) = versions.last() {
                    (latest.version, latest.content_hash.clone())
                } else {
                    (0, String::new())
                };

                let mut content: Option<String> = None;
                if artifact_path.starts_with("notes/") {
                    let note_id = artifact_path.trim_start_matches("notes/").trim_end_matches(".md");
                    if let Ok(Some(note)) = research.get_note(note_id) {
                        content = Some(note.content);
                    }
                }

                if content.is_none() {
                    if let Ok(data) = tokio::fs::read_to_string(artifact_path).await {
                        content = Some(data);
                    }
                }

                let resp = ArtifactDetailResponse {
                    artifact_path: artifact_path.to_string(),
                    latest_version,
                    latest_content_hash,
                    versions,
                    annotations,
                    content,
                };
                match serde_json::to_value(&resp) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_RECORD_LINEAGE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let node: ArtifactLineageNode = match serde_json::from_value(req.params) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid artifact lineage payload: {e}")),
                };
                if node.artifact_path.trim().is_empty() || node.content_hash.trim().is_empty() {
                    return ApiResponse::error(req.id, "Artifact path and content hash cannot be empty");
                }
                match research.record_artifact_lineage(&node) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "recorded": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_LINEAGE_GRAPH => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let filter_path = req.params.get("artifact_path").and_then(|v| v.as_str());
                match research.get_artifact_lineage_graph(filter_path) {
                    Ok(graph) => match serde_json::to_value(&graph) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let session_id = req.params.get("session_id").and_then(|v| v.as_str());
                match research.list_notes(session_id) {
                    Ok(notes) => match serde_json::to_value(&notes) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match research.get_note(id) {
                    Ok(Some(note)) => match serde_json::to_value(&note) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, "Note not found"),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: SaveNoteParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid save note params: {e}")),
                };

                let note = if let Some(ref note_id) = params.id {
                    match research.get_note(note_id) {
                        Ok(Some(mut existing)) => {
                            match existing.update(Some(params.title), params.content, params.tags) {
                                Ok(prev_ver) => {
                                    if let Err(e) = research.save_note(&existing) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    if let Err(e) = research.save_note_version(&prev_ver) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", existing.id),
                                        version: existing.version,
                                        content_hash: existing.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: Some(prev_ver.content_hash),
                                        timestamp: existing.updated_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    existing
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Ok(None) => {
                            match NoteRecord::new(params.title, params.content, params.session_id, params.task_id, params.tags.unwrap_or_default()) {
                                Ok(mut n) => {
                                    n.id = note_id.clone();
                                    if let Err(e) = research.save_note(&n) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", n.id),
                                        version: 1,
                                        content_hash: n.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: None,
                                        timestamp: n.created_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    n
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    match NoteRecord::new(params.title, params.content, params.session_id, params.task_id, params.tags.unwrap_or_default()) {
                        Ok(n) => {
                            if let Err(e) = research.save_note(&n) {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            let node = ArtifactLineageNode {
                                artifact_path: format!("notes/{}.md", n.id),
                                version: 1,
                                content_hash: n.content_hash.clone(),
                                produced_by_run_id: None,
                                parent_version_hash: None,
                                timestamp: n.created_at,
                            };
                            let _ = research.record_artifact_lineage(&node);
                            n
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                };

                match serde_json::to_value(&note) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_HISTORY => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let note_id = match req.params.get("note_id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing note_id parameter"),
                };
                match research.list_note_versions(note_id) {
                    Ok(vers) => match serde_json::to_value(&vers) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_notebook_cells(session_id) {
                    Ok(cells) => match serde_json::to_value(&cells) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_SAVE => {
                let _session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let cells: Vec<NotebookCell> = match req.params.get("cells").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing or invalid cells parameter"),
                };
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.save_notebook_cells(&cells) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "saved": true, "count": cells.len() })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let params: ExecuteCellParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid execute cell params: {e}")),
                };

                // Admission Gate: Ensure requested cwd is valid and not a restricted root/system directory
                if let Some(ref cwd) = params.cwd {
                    let path = std::path::Path::new(cwd);
                    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                    let s = canon.to_string_lossy();
                    if s == "/" || s.starts_with("/etc") || s.starts_with("/dev") || s.starts_with("/proc") || s.starts_with("/sys") {
                        return ApiResponse::error(
                            req.id,
                            format!("Admission gate refused restricted execution directory: '{s}'"),
                        );
                    }
                    if !path.exists() || !path.is_dir() {
                        return ApiResponse::error(
                            req.id,
                            format!("Execution cwd does not exist or is not a directory: '{cwd}'"),
                        );
                    }
                }

                let repo_ref = self.research.as_deref();
                match kernel.execute_cell(params, repo_ref).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_INTERRUPT => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                match kernel.interrupt(session_id).await {
                    Ok(interrupted) => ApiResponse::success(req.id, serde_json::json!({ "interrupted": interrupted })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_RESET => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let repo_ref = self.research.as_deref();
                match kernel.reset(session_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_STATUS => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let repo_ref = self.research.as_deref();
                match kernel.get_status(session_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let target_type = req.params.get("target_type").and_then(|v| serde_json::from_value(v.clone()).ok());
                let target_id = req.params.get("target_id").and_then(|v| v.as_str());
                match research.list_reviews(target_type, target_id) {
                    Ok(reviews) => match serde_json::to_value(&reviews) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("review_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.get_review(id) {
                    Ok(Some(rev)) => match serde_json::to_value(&rev) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_RECORD => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: RecordReviewParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid record review params: {e}")),
                };
                let record = match ReviewerRecord::new(
                    params.target_type,
                    params.target_id,
                    params.reviewer,
                    params.method,
                    params.status,
                    params.evidence_summary,
                    params.evidence_digest,
                    params.findings.unwrap_or_default(),
                ) {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                if let Err(e) = research.save_review(&record) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                match serde_json::to_value(&record) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_MARK_STALE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("review_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.mark_review_stale(id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "marked_stale": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_synthesis_proposals() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("proposal_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty proposal id parameter"),
                };
                match research.get_synthesis_proposal(id) {
                    Ok(Some(prop)) => match serde_json::to_value(&prop) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: SaveSynthesisProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid save proposal params: {e}")),
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                let claims: Vec<ClaimHandoffSummary> = params.claim_ids.iter().filter_map(|cid| {
                    all_claims.iter().find(|c| &c.id == cid).map(|c| {
                        let has_fresh_review = all_reviews.iter().any(|r| {
                            r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                        });
                        ClaimHandoffSummary {
                            claim_id: c.id.clone(),
                            statement: c.statement.clone(),
                            level: c.level,
                            confidence_score: c.confidence_score,
                            has_fresh_review,
                            sealed_proof_uri: c.sealed_proof_uri.clone(),
                        }
                    })
                }).collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let recipes: Vec<RecipeHandoffSummary> = params.recipe_ids.iter().filter_map(|rid| {
                    all_recipes.iter().find(|r| &r.id == rid).map(|r| {
                        RecipeHandoffSummary {
                            recipe_id: r.id.clone(),
                            name: r.name.clone(),
                            command: r.command.clone(),
                            inputs_count: r.inputs.len(),
                            outputs: r.outputs.clone(),
                        }
                    })
                }).collect();

                let mut proposal = match ResearchSynthesisProposal::new(
                    params.title,
                    params.summary,
                    claims,
                    recipes,
                    params.artifact_paths.unwrap_or_default(),
                    params.workspace_id,
                    params.target_branch,
                ) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                if let Some(existing_id) = params.id {
                    proposal.id = existing_id;
                }

                if let Err(e) = research.save_synthesis_proposal(&proposal) {
                    return ApiResponse::error(req.id, e.to_string());
                }

                match serde_json::to_value(&proposal) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: HandoffToCodingParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid handoff params: {e}")),
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                // Invariant: Verify all requested claim IDs actually exist in the repository
                for cid in &params.claim_ids {
                    if !all_claims.iter().any(|c| &c.id == cid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Claim '{cid}' not found in research repository"),
                        );
                    }
                }
                let selected_claims: Vec<&ResearchClaim> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| all_claims.iter().find(|c| &c.id == cid))
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Invariant: Verify all requested recipe IDs actually exist in the repository
                for rid in &params.recipe_ids {
                    if !all_recipes.iter().any(|r| &r.id == rid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Recipe '{rid}' not found in research repository"),
                        );
                    }
                }
                let selected_recipes: Vec<&Recipe> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| all_recipes.iter().find(|r| &r.id == rid))
                    .collect();

                // Fail-closed verification gate: EVERY claim must be grounded/verified
                if params.enforce_verification && !selected_claims.is_empty() {
                    let unverified = selected_claims.iter().find(|c| {
                        !(c.level >= ClaimGroundingLevel::L2Verified
                            || all_reviews.iter().any(|r| {
                                r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                            }))
                    });
                    if let Some(bad_claim) = unverified {
                        return ApiResponse::error(
                            req.id,
                            format!(
                                "Fail-closed verification gate: claim '{}' is ungrounded (L0/unverified) and lacks approved active verifier record or L2+ proof",
                                bad_claim.id
                            ),
                        );
                    }
                }

                let claim_summaries: Vec<ClaimHandoffSummary> = selected_claims.iter().map(|c| {
                    let has_fresh_review = all_reviews.iter().any(|r| {
                        r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                    });
                    ClaimHandoffSummary {
                        claim_id: c.id.clone(),
                        statement: c.statement.clone(),
                        level: c.level,
                        confidence_score: c.confidence_score,
                        has_fresh_review,
                        sealed_proof_uri: c.sealed_proof_uri.clone(),
                    }
                }).collect();

                let recipe_summaries: Vec<RecipeHandoffSummary> = selected_recipes.iter().map(|r| {
                    RecipeHandoffSummary {
                        recipe_id: r.id.clone(),
                        name: r.name.clone(),
                        command: r.command.clone(),
                        inputs_count: r.inputs.len(),
                        outputs: r.outputs.clone(),
                    }
                }).collect();

                let caveats = ResearchSynthesisProposal::generate_caveats(&claim_summaries, &recipe_summaries);
                let mut created_task_ids = Vec::new();

                // 1. Create coding tasks for claims
                for claim in &selected_claims {
                    let cmd = CreateTask {
                        title: format!("[Research Claim] {}", claim.statement),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "claim_id": claim.id,
                            "claim_statement": claim.statement,
                            "claim_level": claim.level,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                            "caveats": caveats,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "engineering_verification".into(),
                            name: format!("Verify: {}", claim.statement),
                            description: format!("Empirically implement and produce diff / test verification for claim: {}", claim.statement),
                            required_capabilities: vec!["code.files".into(), "code.changes".into(), "compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::Diff,
                                    required: true,
                                },
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to child task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(req.id, format!("Failed to create task: {e}"));
                        }
                    }
                }

                // 2. Create coding tasks for recipes
                for recipe in &selected_recipes {
                    let cmd = CreateTask {
                        title: format!("[Recipe Reproduction] {}", recipe.name),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "recipe_id": recipe.id,
                            "recipe_name": recipe.name,
                            "recipe_command": recipe.command,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "recipe_execution".into(),
                            name: format!("Execute: {}", recipe.name),
                            description: format!("Execute recipe command: {}", recipe.command),
                            required_capabilities: vec!["compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to recipe task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(req.id, format!("Failed to create recipe task: {e}"));
                        }
                    }
                }

                // If proposal_id is provided, mark it completed
                if let Some(ref prop_id) = params.proposal_id {
                    if let Ok(Some(mut prop)) = research.get_synthesis_proposal(prop_id) {
                        prop.mark_completed();
                        let _ = research.save_synthesis_proposal(&prop);
                    }
                }

                let handoff_res = HandoffToCodingResult {
                    handoff_id: custos_domain::new_id("handoff"),
                    proposal_id: params.proposal_id,
                    task_ids: created_task_ids,
                    workspace_id: params.workspace_id,
                    target_branch: params.target_branch,
                    verified_claims_count: selected_claims.len(),
                    converted_recipes_count: selected_recipes.len(),
                    caveats,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                match serde_json::to_value(&handoff_res) {
                    Ok(val) => ApiResponse::success(req.id, val),
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
            "v1.providers.get" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match providers_repo.get_provider(id) {
                    Ok(Some(cfg)) => match serde_json::to_value(&cfg) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Provider {id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.delete" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match providers_repo.delete_provider(id) {
                    Ok(deleted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": deleted, "id": id }),
                    ),
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
                let default_model = req
                    .params
                    .get("default_model")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let context_window = req
                    .params
                    .get("context_window")
                    .and_then(|v| v.as_u64());
                let fast_mode = req
                    .params
                    .get("fast_mode")
                    .and_then(|v| v.as_bool());

                let now = chrono::Utc::now().timestamp_millis();

                if let Some(key) = raw_api_key {
                    if !key.trim().is_empty() {
                        let tok = custos_domain::OAuthTokenRecord {
                            provider_id: id.clone(),
                            service_type: service_type.clone(),
                            access_token: key.trim().to_string(),
                            refresh_token: None,
                            expires_at: i64::MAX / 2,
                            token_type: "bearer".to_string(),
                            scope: None,
                            created_at: now,
                            updated_at: now,
                        };
                        let _ = providers_repo.save_oauth_token(&tok);
                        if service_type != id {
                            let mut tok_alias = tok.clone();
                            tok_alias.provider_id = service_type.clone();
                            let _ = providers_repo.save_oauth_token(&tok_alias);
                        }
                    }
                }

                let cfg = custos_domain::ProviderConfig {
                    id,
                    name,
                    service_type,
                    api_key_masked: masked,
                    status,
                    endpoint_url,
                    default_model,
                    context_window,
                    fast_mode,
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
            "v1.models.probe" => {
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string());

                let provider_record = if let (Some(repo), Some(pid)) =
                    (self.providers.as_ref(), provider_id.as_deref())
                {
                    repo.get_provider(pid).ok().flatten()
                } else {
                    None
                };

                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| provider_record.as_ref().map(|p| p.service_type.as_str()))
                    .unwrap_or("local");

                let base_url = req
                    .params
                    .get("base_url")
                    .or_else(|| req.params.get("endpoint_url"))
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        provider_record
                            .as_ref()
                            .and_then(|p| p.endpoint_url.as_deref())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    })
                    .or_else(|| default_model_probe_base_url(provider_type).map(str::to_string))
                    .unwrap_or_default();

                if base_url.trim().is_empty() {
                    return ApiResponse::error(req.id, "Missing base_url parameter for models probe");
                }

                let api_key_owned = req
                    .params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        self.providers.as_ref().and_then(|repo| {
                            provider_id
                                .as_deref()
                                .and_then(|pid| repo.get_oauth_token(pid).ok().flatten())
                                .or_else(|| repo.get_oauth_token(provider_type).ok().flatten())
                                .map(|tok| tok.access_token)
                        })
                    });
                let api_key = api_key_owned.as_deref();

                match custos_adapters::providers::probe::probe_endpoint_models(&base_url, api_key, provider_type).await {
                    Ok(models) => {
                        if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id.as_deref()) {
                            let catalog_result = self.model_catalog.merge_probed_models(provider_type, models.clone());
                            let _ = repo.save_catalog_models(pid, &catalog_result.models);
                        }
                        match serde_json::to_value(&models) {
                            Ok(val) => ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "models": val,
                                    "count": models.len(),
                                    "base_url": base_url,
                                }),
                            ),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Probe failed: {e}")),
                }
            }
            "v1.models.catalog" => {
                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str());
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id) {
                    if let Ok(cached) = repo.list_catalog_models(Some(pid)) {
                        if !cached.is_empty() {
                            return ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "origin": "cached",
                                    "models": cached,
                                    "fetched_at": chrono::Utc::now().timestamp_millis(),
                                }),
                            );
                        }
                    }
                }

                let catalog = self.model_catalog.get_catalog(provider_type);
                match serde_json::to_value(&catalog) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.models.pricing" => {
                let model_id = req
                    .params
                    .get("model_id")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if model_id.trim().is_empty() {
                    return ApiResponse::error(req.id, "Missing model_id parameter");
                }

                let pricing = custos_adapters::providers::pricing::lookup_model_pricing(model_id);
                let input_tokens = req.params.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let output_tokens = req.params.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_read_tokens = req.params.get("cache_read_tokens").and_then(|v| v.as_u64());
                let cache_write_tokens = req.params.get("cache_write_tokens").and_then(|v| v.as_u64());

                let estimated_cost = pricing.as_ref().map(|p| {
                    custos_adapters::providers::pricing::calculate_token_cost(
                        p,
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_write_tokens,
                    )
                });

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "model_id": model_id,
                        "pricing": pricing,
                        "estimated_cost_usd": estimated_cost,
                    }),
                )
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
                    || std::env::var("GEMINI_API_KEY").is_ok()
                    || std::env::var("CUSTOS_PROVIDER").is_ok();

                let oauth_token = if let Some(ref repo) = self.providers {
                    repo.get_oauth_token("openai").ok().flatten()
                } else {
                    None
                };

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

                if let Some(tok) = oauth_token {
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": "OpenAI (OAuth PKCE)",
                            "message": "OAuth 2.0 PKCE authentication active",
                            "expires_at": tok.expires_at,
                        }),
                    )
                } else if has_env_key || configured_provider.is_some() {
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
            METHOD_OAUTH_AUTHORIZE => {
                let params: OAuthAuthorizeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let auth_url = params
                    .auth_endpoint
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_TOKEN_URL);
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_CLIENT_ID);
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI);
                let scope = params.scope.as_deref();

                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(auth_url, token_url);
                let challenge = custos_adapters::providers::oauth_pkce::OAuthPkceManager::generate_pkce_challenge();

                match manager.build_authorization_url(client_id, redirect_uri, scope, &challenge) {
                    Ok(res) => {
                        // Start the loopback callback server on port 1455 if available
                        if let Some(ref cb_server) = self.oauth_callback_server {
                            let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                            let service_type = params.service_type.as_deref().unwrap_or("openai");
                            let pending = crate::oauth_callback_server::PendingOAuthContext {
                                provider_id: provider_id.to_string(),
                                service_type: service_type.to_string(),
                                client_id: client_id.to_string(),
                                code_verifier: challenge.code_verifier.clone(),
                                state: challenge.state.clone(),
                                redirect_uri: redirect_uri.to_string(),
                            };
                            let _ = cb_server.start_listening(pending).await;
                        }

                        match serde_json::to_value(&res) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_EXCHANGE => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let params: OAuthExchangeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                let service_type = params.service_type.as_deref().unwrap_or("openai");
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_CLIENT_ID);
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_TOKEN_URL);

                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager
                    .exchange_code(
                        provider_id,
                        service_type,
                        client_id,
                        &params.code_or_url,
                        redirect_uri,
                        &params.code_verifier,
                    )
                    .await
                {
                    Ok(token_record) => {
                        // Persist in provider_oauth_tokens
                        if let Err(e) = providers_repo.save_oauth_token(&token_record) {
                            return ApiResponse::error(req.id, format!("Failed to save OAuth token: {e}"));
                        }

                        // Upsert matching provider config so it appears in UI as active
                        let now = chrono::Utc::now().timestamp_millis();
                        let masked = if token_record.access_token.len() > 8 {
                            format!("{}••••••••", &token_record.access_token[..6])
                        } else {
                            "••••••••".to_string()
                        };
                        let prov_cfg = custos_domain::ProviderConfig {
                            id: provider_id.to_string(),
                            name: format!("{} (OAuth)", if provider_id == "openai" { "OpenAI" } else { provider_id }),
                            service_type: service_type.to_string(),
                            api_key_masked: masked,
                            status: "active".to_string(),
                            endpoint_url: default_model_probe_base_url(service_type).map(str::to_string),
                            default_model: None,
                            context_window: None,
                            fast_mode: Some(false),
                            created_at: now,
                            updated_at: now,
                        };
                        let _ = providers_repo.save_provider(&prov_cfg);

                        match serde_json::to_value(&token_record) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_REFRESH => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let params: OAuthRefreshParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let existing = match providers_repo.get_oauth_token(&params.provider_id) {
                    Ok(Some(tok)) => tok,
                    Ok(None) => return ApiResponse::error(req.id, format!("No OAuth token found for {}", params.provider_id)),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let client_id = params.client_id.as_deref().unwrap_or("custos-openai-desktop");
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_TOKEN_URL);
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager.refresh_token(client_id, &existing).await {
                    Ok(refreshed) => {
                        if let Err(e) = providers_repo.save_oauth_token(&refreshed) {
                            return ApiResponse::error(req.id, format!("Failed to update token: {e}"));
                        }
                        match serde_json::to_value(&refreshed) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_GET => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match providers_repo.get_oauth_token(provider_id) {
                    Ok(Some(tok)) => {
                        let sanitized = serde_json::json!({
                            "provider_id": tok.provider_id,
                            "service_type": tok.service_type,
                            "connected": !tok.access_token.is_empty(),
                            "expires_at": tok.expires_at,
                            "token_type": tok.token_type,
                            "scope": tok.scope,
                            "has_refresh_token": tok.refresh_token.is_some(),
                        });
                        ApiResponse::success(req.id, sanitized)
                    }
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_DELETE => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match providers_repo.delete_oauth_token(provider_id) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "provider_id": provider_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_STATUS => {
                if let Some(ref cb_server) = self.oauth_callback_server {
                    let status_val = match cb_server.status() {
                        crate::oauth_callback_server::CallbackServerStatus::Idle => {
                            serde_json::json!({ "status": "idle" })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Listening { port } => {
                            serde_json::json!({ "status": "listening", "port": port })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Exchanging { code } => {
                            serde_json::json!({ "status": "exchanging", "code": code })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Completed { provider_id } => {
                            serde_json::json!({ "status": "completed", "provider_id": provider_id })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Failed { error } => {
                            serde_json::json!({ "status": "failed", "error": error })
                        }
                    };
                    ApiResponse::success(req.id, status_val)
                } else {
                    ApiResponse::success(req.id, serde_json::json!({ "status": "idle" }))
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
            METHOD_TERMINAL_SPAWN => {
                let workspace_id = match req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };

                let working_dir_str = match req.params.get("working_dir").and_then(|v| v.as_str()) {
                    Some(dir) => dir.to_string(),
                    None => {
                        let coordinator = match self.workspace.as_ref() {
                            Some(c) => c,
                            None => {
                                return ApiResponse::error(
                                    req.id,
                                    "WorkspaceCoordinator not configured on daemon",
                                )
                            }
                        };
                        let ws_id = custos_domain::WorkspaceId::new(workspace_id);
                        match coordinator.get_workspace(&ws_id).await {
                            Ok(Some(ws)) => ws.path,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Workspace {workspace_id} not found"),
                                );
                            }
                            Err(e) => {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Workspace {workspace_id} lookup failed: {e}"),
                                );
                            }
                        }
                    }
                };

                let command = req.params.get("command").and_then(|v| v.as_str());
                let cols = req
                    .params
                    .get("cols")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(80) as u16;
                let rows = req
                    .params
                    .get("rows")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(24) as u16;

                match self
                    .terminal
                    .spawn(
                        workspace_id,
                        std::path::Path::new(&working_dir_str),
                        command,
                        cols,
                        rows,
                    )
                    .await
                {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_WRITE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let data = match req.params.get("data").and_then(|v| v.as_str()) {
                    Some(d) => d,
                    None => return ApiResponse::error(req.id, "Missing data parameter"),
                };

                match self.terminal.write(session_id, data.as_bytes()).await {
                    Ok(written) => {
                        ApiResponse::success(req.id, serde_json::json!({ "written": written }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_RESIZE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let cols = match req.params.get("cols").and_then(|v| v.as_u64()) {
                    Some(c) => c as u16,
                    None => return ApiResponse::error(req.id, "Missing cols parameter"),
                };
                let rows = match req.params.get("rows").and_then(|v| v.as_u64()) {
                    Some(r) => r as u16,
                    None => return ApiResponse::error(req.id, "Missing rows parameter"),
                };

                match self.terminal.resize(session_id, cols, rows).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "ok": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_READ => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let from_seq = req
                    .params
                    .get("from_seq")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let max_bytes = req
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(65536) as usize;

                match self
                    .terminal
                    .read_output(session_id, from_seq, max_bytes)
                    .await
                {
                    Ok(chunk) => match serde_json::to_value(&chunk) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_TERMINATE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.terminal.terminate(session_id).await {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "terminated": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_LIST => {
                let workspace_id = req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str());

                match self.terminal.list_sessions(workspace_id).await {
                    Ok(sessions) => match serde_json::to_value(&sessions) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_GET => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.terminal.get_session(session_id).await {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_TREE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };
                let relative_dir = req.params.get("relative_dir").and_then(|v| v.as_str());
                let max_depth = req.params.get("max_depth").and_then(|v| v.as_u64()).map(|d| d as usize);

                match self.files.get_file_tree(&ws, relative_dir, max_depth).await {
                    Ok(tree) => match serde_json::to_value(&tree) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_READ => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let max_bytes = req.params.get("max_bytes").and_then(|v| v.as_u64()).map(|b| b as usize);
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.read_file(&ws, path, max_bytes).await {
                    Ok(content) => match serde_json::to_value(&content) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_WRITE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c.to_string(),
                    None => return ApiResponse::error(req.id, "Missing content parameter"),
                };
                let create_parents = req.params.get("create_parents").and_then(|v| v.as_bool()).unwrap_or(true);
                let overwrite = req.params.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(true);
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                let mut params = custos_domain::WriteWorkspaceFileParams::new(ws.id.clone(), path, content);
                params.create_parents = create_parents;
                params.overwrite = overwrite;

                match self.files.write_file(&ws, params).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_diff(&ws, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_file_diff(&ws, path, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_STAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.stage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "staged": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_UNSTAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.unstage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "unstaged": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_DISCARD => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.discard_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "discarded": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_LIST => {
                let harnesses = self.harnesses.list();
                match serde_json::to_value(&harnesses) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_GET => {
                let harness_id = req
                    .params
                    .get("harness_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match harness_id {
                    Some(id) => match self.harnesses.get_descriptor(id) {
                        Some(desc) => match serde_json::to_value(&desc) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        },
                        None => ApiResponse::error(req.id, format!("Harness '{id}' not found")),
                    },
                    None => ApiResponse::error(req.id, "Missing harness_id parameter"),
                }
            }
            METHOD_HARNESS_RUN_NATIVE => {
                let params: RunNativeHarnessRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid run_native parameters: {e}")),
                };

                let cwd_path = if let Some(ws_id) = &params.workspace_id {
                    match self.resolve_workspace_for_dispatch(&req.id, ws_id).await {
                        Ok(ws) => std::path::PathBuf::from(ws.path),
                        Err(resp) => return resp,
                    }
                } else if let Some(c) = &params.cwd {
                    std::path::PathBuf::from(c)
                } else {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                };

                match self.harnesses.run_native(&params.harness_id, &params.instruction, &cwd_path).await {
                    Ok(result) => match serde_json::to_value(&result) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(err) => ApiResponse::error(req.id, format!("Harness execution failed: {err}")),
                }
            }
            METHOD_HARNESS_CANCEL => {
                let params: CancelHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid cancel parameters: {e}")),
                };
                match self.harnesses.cancel(&params.harness_id, &params.run_id).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "cancelled": true })),
                    Err(err) => ApiResponse::error(req.id, format!("Failed to cancel harness: {err}")),
                }
            }
            METHOD_HARNESS_STEER => {
                let params: SteerHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid steer parameters: {e}")),
                };
                match self.harnesses.steer(&params.harness_id, &params.run_id, &params.guidance).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "steered": true })),
                    Err(err) => ApiResponse::error(req.id, format!("Failed to steer harness: {err}")),
                }
            }
            METHOD_BROWSER_SESSIONS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let ws_id = req.params.get("workspace_id").and_then(|v| v.as_str());
                match fleet.list_browser_sessions() {
                    Ok(sessions) => {
                        let filtered = if let Some(ws) = ws_id {
                            sessions.into_iter().filter(|s| s.workspace_id.as_deref() == Some(ws)).collect()
                        } else {
                            sessions
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_SESSIONS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("Default Session");
                let ws_id = req.params.get("workspace_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let session = BrowserSession::new(name, ws_id);
                match fleet.save_browser_session(&session) {
                    Ok(()) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                match fleet.list_browser_tabs(Some(session_id)) {
                    Ok(tabs) => match serde_json::to_value(&tabs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = req.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let tab = match BrowserTab::new(session_id, url) {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_NAVIGATE => {
                ApiResponse::error(
                    req.id,
                    "Browser navigation is unavailable until a scoped browser adapter is connected",
                )
            }
            METHOD_BROWSER_TABS_SNAPSHOT => {
                ApiResponse::error(
                    req.id,
                    "Browser snapshots are unavailable until an authenticated browser adapter supplies them",
                )
            }
            METHOD_BROWSER_TABS_CLOSE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                match fleet.delete_browser_tab(tab_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "closed": true, "tab_id": tab_id })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                match fleet.list_remote_hosts() {
                    Ok(hosts) => match serde_json::to_value(&hosts) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_REGISTER => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let params: RegisterHostParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid register host params: {e}")),
                };
                let auth_method = if let Some(key_path) = &params.private_key_path {
                    SshAuthMethod::KeyPair {
                        private_key_path: key_path.clone(),
                        passphrase: None,
                    }
                } else {
                    SshAuthMethod::Agent
                };
                let mut node = match RemoteHostNode::new(params.name, params.host, params.port, params.user, auth_method) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                node.labels = params.labels;
                match fleet.save_remote_host(&node) {
                    Ok(()) => match serde_json::to_value(&node) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_PING => {
                ApiResponse::error(
                    req.id,
                    "Remote host probing is unavailable until a fleet transport adapter is connected",
                )
            }
            METHOD_FLEET_EXEC => {
                ApiResponse::error(
                    req.id,
                    "Remote execution is unavailable until transport, authority and receipt verification are connected",
                )
            }
            METHOD_AUTOMATION_JOBS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let status_filter = req.params.get("status")
                    .and_then(|v| serde_json::from_value::<HeadlessJobStatus>(v.clone()).ok());
                match fleet.list_headless_jobs() {
                    Ok(jobs) => {
                        let filtered: Vec<HeadlessAutomationJob> = if let Some(filter) = status_filter {
                            jobs.into_iter().filter(|j| j.status == filter).collect()
                        } else {
                            jobs
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let params: CreateHeadlessJobParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid job parameters: {e}")),
                };
                let trigger = params.trigger.unwrap_or(custos_domain::HeadlessTrigger::Manual);
                let job = match HeadlessAutomationJob::new(params.name, params.spec, trigger) {
                    Ok(j) => j,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_headless_job(&job) {
                    Ok(()) => match serde_json::to_value(&job) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_RUN => {
                ApiResponse::error(
                    req.id,
                    "Headless execution is unavailable until an executor and scoped authority checks are connected",
                )
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
    use custos_domain::{
        ContinuationPacket, DomainError, ExecuteCellResult, NotebookKernelState, Session, Span,
        Task,
    };
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
        let created_session: Session = serde_json::from_value(session_res.result.unwrap()).unwrap();

        let delete_session_req = ApiRequest {
            id: "req_sess_delete".into(),
            method: METHOD_SESSIONS_DELETE.into(),
            params: serde_json::json!({ "session_id": created_session.id.0 }),
        };
        let delete_session_res = dispatcher.handle_request(delete_session_req).await;
        assert!(delete_session_res.error.is_none());
        assert_eq!(
            delete_session_res
                .result
                .unwrap()
                .get("deleted")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

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

        // 4. Start run with unverified session_id must be refused by admission gate
        let invalid_sess_req = ApiRequest {
            id: "req_invalid_sess".into(),
            method: "v1.workflow.start_run".into(),
            params: serde_json::json!({
                "task_id": "task_api_workflow_invalid_sess",
                "session_id": "non_existent_sess_999",
                "actor": "tester"
            }),
        };
        let invalid_resp = dispatcher.handle_request(invalid_sess_req).await;
        assert!(invalid_resp.error.is_some());
        assert!(invalid_resp
            .error
            .unwrap()
            .contains("Admission gate refused unverified session_id"));
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

        // 3b. Inspect dirty manifest
        let inspect_req = ApiRequest {
            id: "req_ws_inspect".into(),
            method: METHOD_WORKSPACES_INSPECT_DIRTY.into(),
            params: serde_json::json!({
                "workspace_id": created_ws.id.as_str()
            }),
        };
        let inspect_resp = dispatcher.handle_request(inspect_req).await;
        assert!(inspect_resp.is_success());
        let dirty: custos_domain::DirtyManifest =
            serde_json::from_value(inspect_resp.result.unwrap()).unwrap();
        assert!(!dirty.is_dirty);

        // 3c. Recover workspace
        let recover_req = ApiRequest {
            id: "req_ws_recover".into(),
            method: METHOD_WORKSPACES_RECOVER.into(),
            params: serde_json::json!({
                "workspace_id": created_ws.id.as_str()
            }),
        };
        let recover_resp = dispatcher.handle_request(recover_req).await;
        assert!(recover_resp.is_success());

        // 4. Archive Workspace (with delete_physical: true)
        let archive_req = ApiRequest {
            id: "req_ws_4".into(),
            method: METHOD_WORKSPACES_ARCHIVE.into(),
            params: serde_json::json!({
                "workspace_id": created_ws.id.as_str(),
                "delete_physical": true,
                "force": false
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

        // PTY terminal is truthfully available with supported operations
        let pty_cap = caps.iter().find(|c| c.id == "compute.pty").unwrap();
        assert!(pty_cap.status.is_available());
        assert_eq!(pty_cap.resource_id.as_deref(), Some("terminal"));
        assert!(pty_cap.supported_operations.contains(&"spawn".to_string()));

        // Files and Changes capabilities are truthfully available with supported operations
        let files_cap = caps.iter().find(|c| c.id == "code.files").unwrap();
        assert!(files_cap.status.is_available());
        assert!(files_cap.supported_operations.contains(&"tree".to_string()));
        assert!(files_cap.supported_operations.contains(&"read".to_string()));
        assert!(files_cap
            .supported_operations
            .contains(&"write".to_string()));

        let changes_cap = caps.iter().find(|c| c.id == "code.changes").unwrap();
        assert!(changes_cap.status.is_available());
        assert!(changes_cap
            .supported_operations
            .contains(&"diff".to_string()));
        assert!(changes_cap
            .supported_operations
            .contains(&"stage".to_string()));

        // Notebook capability is truthfully available when PythonKernelCoordinator is configured
        let notebook_cap = caps.iter().find(|c| c.id == "compute.notebook").unwrap();
        assert!(notebook_cap.status.is_available());
        assert!(notebook_cap
            .supported_operations
            .contains(&"execute".to_string()));
        assert!(notebook_cap
            .supported_operations
            .contains(&"reset".to_string()));

        // Evidence criteria verifier capability is truthfully unavailable without ResearchRepository
        let evidence_cap = caps.iter().find(|c| c.id == "evidence.criteria").unwrap();
        assert!(!evidence_cap.status.is_available());
        assert_eq!(
            evidence_cap.status.reason(),
            Some("ResearchRepository is not configured on this daemon instance.")
        );

        // Synthesis handoff capability is truthfully unavailable without ResearchRepository
        let synth_cap = caps.iter().find(|c| c.id == "synthesis.handoff").unwrap();
        assert!(!synth_cap.status.is_available());
        assert_eq!(
            synth_cap.status.reason(),
            Some("ResearchRepository is not configured on this daemon instance.")
        );

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

    #[tokio::test]
    #[cfg(unix)]
    async fn test_terminal_api_dispatch_lifecycle() {
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

        let temp = tempfile::tempdir().unwrap();

        // 1. Spawn terminal session
        let spawn_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t1".into(),
                method: METHOD_TERMINAL_SPAWN.into(),
                params: serde_json::json!({
                    "workspace_id": "ws-mock",
                    "working_dir": temp.path().to_str().unwrap(),
                    "command": "echo test_terminal_stream",
                    "cols": 80,
                    "rows": 24,
                }),
            })
            .await;

        assert!(
            spawn_resp.is_success(),
            "Failed to spawn: {:?}",
            spawn_resp.error
        );
        let session: custos_domain::TerminalSession =
            serde_json::from_value(spawn_resp.result.unwrap()).unwrap();
        assert_eq!(session.workspace_id, "ws-mock");

        // 2. Poll read
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        let read_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t2".into(),
                method: METHOD_TERMINAL_READ.into(),
                params: serde_json::json!({
                    "session_id": session.id,
                    "from_seq": 0,
                    "max_bytes": 4096,
                }),
            })
            .await;

        assert!(read_resp.is_success());
        let chunk: custos_domain::TerminalOutputChunk =
            serde_json::from_value(read_resp.result.unwrap()).unwrap();
        assert!(chunk.data.contains("test_terminal_stream"));

        // 3. Resize
        let resize_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t3".into(),
                method: METHOD_TERMINAL_RESIZE.into(),
                params: serde_json::json!({
                    "session_id": session.id,
                    "cols": 120,
                    "rows": 30,
                }),
            })
            .await;
        assert!(resize_resp.is_success());

        // 4. List terminals
        let list_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t4".into(),
                method: METHOD_TERMINAL_LIST.into(),
                params: serde_json::json!({ "workspace_id": "ws-mock" }),
            })
            .await;
        assert!(list_resp.is_success());
        let sessions: Vec<custos_domain::TerminalSession> =
            serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert_eq!(sessions.len(), 1);

        // 5. Terminate
        let term_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t5".into(),
                method: METHOD_TERMINAL_TERMINATE.into(),
                params: serde_json::json!({ "session_id": session.id }),
            })
            .await;
        assert!(term_resp.is_success());

        // 6. Get terminal
        let get_resp = dispatcher
            .handle_request(ApiRequest {
                id: "req_t6".into(),
                method: METHOD_TERMINAL_GET.into(),
                params: serde_json::json!({ "session_id": session.id }),
            })
            .await;
        assert!(get_resp.is_success());
        let got_sess: custos_domain::TerminalSession =
            serde_json::from_value(get_resp.result.unwrap()).unwrap();
        assert_eq!(
            got_sess.status,
            custos_domain::TerminalSessionStatus::Terminated
        );
    }

    #[tokio::test]
    async fn test_workspace_files_api_dispatch() {
        use custos_persistence::SqliteTaskStore;

        let temp_dir =
            std::env::temp_dir().join(format!("custos_ws_api_files_{}", std::process::id()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));

        let ws_provider = Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
        let ws_coord = Arc::new(WorkspaceCoordinator::new(store, ws_provider));
        let ws = ws_coord
            .create_workspace(CreateWorkspaceRequest {
                name: "files_test_ws".into(),
                kind: custos_domain::WorkspaceKind::Folder {
                    path: temp_dir.to_string_lossy().to_string(),
                },
                path: temp_dir.to_string_lossy().to_string(),
                lineage: None,
                owner_task_id: None,
                metadata: None,
                setup_script: None,
            })
            .await
            .expect("create workspace");

        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_workspace(ws_coord);

        // 1. Write file
        let write_resp = dispatcher
            .handle_request(ApiRequest {
                id: "wf_1".into(),
                method: METHOD_WORKSPACE_FILES_WRITE.into(),
                params: serde_json::json!({
                    "workspace_id": ws.id.to_string(),
                    "path": "src/hello.rs",
                    "content": "pub fn hello() -> &'static str { \"world\" }\n",
                }),
            })
            .await;
        assert!(write_resp.is_success());
        let written: custos_domain::WorkspaceFileContent =
            serde_json::from_value(write_resp.result.unwrap()).unwrap();
        assert_eq!(written.path, "src/hello.rs");
        assert_eq!(written.line_count, 1);

        // 2. Read file
        let read_resp = dispatcher
            .handle_request(ApiRequest {
                id: "wf_2".into(),
                method: METHOD_WORKSPACE_FILES_READ.into(),
                params: serde_json::json!({
                    "workspace_id": ws.id.to_string(),
                    "path": "src/hello.rs",
                }),
            })
            .await;
        assert!(read_resp.is_success());
        let read: custos_domain::WorkspaceFileContent =
            serde_json::from_value(read_resp.result.unwrap()).unwrap();
        assert!(read.content.contains("pub fn hello"));

        // 3. Tree
        let tree_resp = dispatcher
            .handle_request(ApiRequest {
                id: "wf_3".into(),
                method: METHOD_WORKSPACE_FILES_TREE.into(),
                params: serde_json::json!({
                    "workspace_id": ws.id.to_string(),
                }),
            })
            .await;
        assert!(tree_resp.is_success());
        let tree: custos_domain::WorkspaceFileTree =
            serde_json::from_value(tree_resp.result.unwrap()).unwrap();
        assert!(tree.entries.iter().any(|e| e.path == "src/hello.rs"));

        // 4. Diff (folder ws is not git repo, returns clean summary)
        let diff_resp = dispatcher
            .handle_request(ApiRequest {
                id: "wf_4".into(),
                method: METHOD_WORKSPACE_DIFF.into(),
                params: serde_json::json!({
                    "workspace_id": ws.id.to_string(),
                }),
            })
            .await;
        assert!(diff_resp.is_success());
        let diff: custos_domain::WorkspaceDiffSummary =
            serde_json::from_value(diff_resp.result.unwrap()).unwrap();
        assert!(diff.is_clean);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_artifact_and_notes_api_lifecycle() {
        use custos_domain::{ArtifactSummary, NoteVersionRecord};

        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let db = custos_persistence::DbConnection::open_in_memory().unwrap();
        let research = Arc::new(ResearchRepository::new(db));

        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_research(research);

        // 1. Save a Note
        let save_resp = dispatcher
            .handle_request(ApiRequest {
                id: "note_save_1".into(),
                method: METHOD_NOTES_SAVE.into(),
                params: serde_json::json!({
                    "title": "Quantum Attention Hypothesis",
                    "content": "# Theory\nEvaluating linear attention scaling.",
                    "session_id": "sess_101",
                    "tags": ["quantum", "scaling"]
                }),
            })
            .await;
        assert!(save_resp.is_success());
        let note: NoteRecord = serde_json::from_value(save_resp.result.unwrap()).unwrap();
        assert_eq!(note.title, "Quantum Attention Hypothesis");
        assert_eq!(note.version, 1);

        // 2. Update Note (version 2)
        let update_resp = dispatcher
            .handle_request(ApiRequest {
                id: "note_save_2".into(),
                method: METHOD_NOTES_SAVE.into(),
                params: serde_json::json!({
                    "id": note.id,
                    "title": "Quantum Attention Hypothesis (Revised)",
                    "content": "# Theory\nUpdated with ablation empirical data.",
                    "session_id": "sess_101",
                    "tags": ["quantum", "scaling", "ablation"]
                }),
            })
            .await;
        assert!(update_resp.is_success());
        let updated_note: NoteRecord = serde_json::from_value(update_resp.result.unwrap()).unwrap();
        assert_eq!(updated_note.version, 2);

        // 3. List Note History
        let hist_resp = dispatcher
            .handle_request(ApiRequest {
                id: "note_hist_1".into(),
                method: METHOD_NOTES_HISTORY.into(),
                params: serde_json::json!({
                    "note_id": note.id,
                }),
            })
            .await;
        assert!(hist_resp.is_success());
        let history: Vec<NoteVersionRecord> =
            serde_json::from_value(hist_resp.result.unwrap()).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 1);

        // 4. Artifact list includes note
        let art_list_resp = dispatcher
            .handle_request(ApiRequest {
                id: "art_list_1".into(),
                method: METHOD_ARTIFACTS_LIST.into(),
                params: serde_json::json!({}),
            })
            .await;
        assert!(art_list_resp.is_success());
        let artifacts: Vec<ArtifactSummary> =
            serde_json::from_value(art_list_resp.result.unwrap()).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_path, format!("notes/{}.md", note.id));
        assert_eq!(artifacts[0].latest_version, 2);

        // 5. Lineage Graph DAG
        let dag_resp = dispatcher
            .handle_request(ApiRequest {
                id: "art_dag_1".into(),
                method: METHOD_ARTIFACTS_LINEAGE_GRAPH.into(),
                params: serde_json::json!({}),
            })
            .await;
        assert!(dag_resp.is_success());
        let graph: custos_domain::ArtifactLineageGraph =
            serde_json::from_value(dag_resp.result.unwrap()).unwrap();
        assert_eq!(graph.nodes.len(), 2); // v1 and v2
        assert_eq!(graph.edges.len(), 1); // edge from v1 to v2
    }

    #[tokio::test]
    async fn test_notebook_kernel_api_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("custos_test_nb.db");
        let store =
            Arc::new(custos_persistence::SqliteTaskStore::new(&db_path.to_string_lossy()).unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let research = Arc::new(store.research().clone());
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_research(research);
        let session_id = "test_nb_session_42";

        // 1. Initial kernel status
        let status_req = ApiRequest {
            id: "nb_status_1".into(),
            method: METHOD_NOTEBOOK_STATUS.into(),
            params: serde_json::json!({ "session_id": session_id }),
        };
        let status_resp = dispatcher.handle_request(status_req).await;
        assert!(status_resp.is_success());
        let initial_state: NotebookKernelState =
            serde_json::from_value(status_resp.result.unwrap()).unwrap();
        assert_eq!(initial_state.epoch, 1);
        assert_eq!(initial_state.execution_counter, 0);

        // 2. Save cells
        let cell = NotebookCell {
            id: custos_domain::new_id("cell"),
            session_id: session_id.to_string(),
            cell_type: custos_domain::NotebookCellType::Code,
            source: "print(6 * 7)".to_string(),
            cell_index: 0,
            execution_count: None,
            status: custos_domain::CellExecutionStatus::Idle,
            stdout: None,
            stderr: None,
            output_image: None,
            wall_ms: None,
            epoch: 0,
            updated_at: chrono::Utc::now().timestamp(),
        };
        let save_cells_req = ApiRequest {
            id: "nb_save_1".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cells": [cell.clone()]
            }),
        };
        let save_resp = dispatcher.handle_request(save_cells_req).await;
        assert!(
            save_resp.is_success(),
            "Failed to save cells: {:?}",
            save_resp.error
        );

        // 3. List cells
        let list_req = ApiRequest {
            id: "nb_list_1".into(),
            method: METHOD_NOTEBOOK_CELLS_LIST.into(),
            params: serde_json::json!({ "session_id": session_id }),
        };
        let list_resp = dispatcher.handle_request(list_req).await;
        assert!(list_resp.is_success());
        let loaded_cells: Vec<NotebookCell> =
            serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert_eq!(loaded_cells.len(), 1);
        assert_eq!(loaded_cells[0].source, "print(6 * 7)");

        // 4. Execute cell
        let exec_req = ApiRequest {
            id: "nb_exec_1".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": "print(6 * 7)",
            }),
        };
        let exec_resp = dispatcher.handle_request(exec_req).await;
        assert!(
            exec_resp.is_success(),
            "Execution failed: {:?}",
            exec_resp.error
        );
        let exec_result: ExecuteCellResult =
            serde_json::from_value(exec_resp.result.unwrap()).unwrap();
        assert_eq!(
            exec_result.status,
            custos_domain::CellExecutionStatus::Success
        );
        assert_eq!(exec_result.stdout.trim(), "42");
        assert_eq!(exec_result.execution_count, 1);
        assert_eq!(exec_result.epoch, 1);

        // 5. Reset kernel
        let reset_req = ApiRequest {
            id: "nb_reset_1".into(),
            method: METHOD_NOTEBOOK_RESET.into(),
            params: serde_json::json!({ "session_id": session_id }),
        };
        let reset_resp = dispatcher.handle_request(reset_req).await;
        assert!(reset_resp.is_success());
        let reset_state: NotebookKernelState =
            serde_json::from_value(reset_resp.result.unwrap()).unwrap();
        assert_eq!(reset_state.epoch, 2);
        assert_eq!(reset_state.execution_counter, 0);
    }

    #[tokio::test]
    async fn test_reviewer_records_api_lifecycle() {
        let store = Arc::new(MockStore {
            tasks: Mutex::new(Vec::new()),
        });
        let task_service = Arc::new(TaskService::new(store));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let mem_conn = custos_persistence::DbConnection::open_in_memory().expect("open memory db");
        let research_repo = Arc::new(custos_persistence::ResearchRepository::new(mem_conn));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_research(research_repo);

        // 1. Record a review
        let record_req = ApiRequest {
            id: "rev_record_1".into(),
            method: METHOD_REVIEWS_RECORD.into(),
            params: serde_json::json!({
                "target_type": "artifact",
                "target_id": "artifacts/report.md",
                "reviewer": "custos-verifier",
                "method": "automated_verifier",
                "status": "approved",
                "evidence_summary": "Verified markdown schema and signatures",
                "evidence_digest": "sha256:5678",
                "findings": [
                    {
                        "severity": "info",
                        "criterion": "schema_conformant",
                        "message": "Valid schema",
                        "file_path": "artifacts/report.md",
                        "line_number": 1
                    }
                ]
            }),
        };
        let record_resp = dispatcher.handle_request(record_req).await;
        assert!(
            record_resp.is_success(),
            "Failed to record review: {:?}",
            record_resp.error
        );
        let record: ReviewerRecord = serde_json::from_value(record_resp.result.unwrap()).unwrap();
        assert_eq!(record.reviewer, "custos-verifier");
        assert_eq!(record.status, custos_domain::ReviewStatus::Approved);
        assert!(record.is_fresh);
        assert_eq!(record.findings.len(), 1);

        // 2. Get review by id
        let get_req = ApiRequest {
            id: "rev_get_1".into(),
            method: METHOD_REVIEWS_GET.into(),
            params: serde_json::json!({ "review_id": record.id }),
        };
        let get_resp = dispatcher.handle_request(get_req).await;
        assert!(get_resp.is_success());
        let fetched: Option<ReviewerRecord> =
            serde_json::from_value(get_resp.result.unwrap()).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, record.id);

        // 3. List reviews by target
        let list_req = ApiRequest {
            id: "rev_list_1".into(),
            method: METHOD_REVIEWS_LIST.into(),
            params: serde_json::json!({
                "target_type": "artifact",
                "target_id": "artifacts/report.md"
            }),
        };
        let list_resp = dispatcher.handle_request(list_req).await;
        assert!(list_resp.is_success());
        let list: Vec<ReviewerRecord> = serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, record.id);

        // 4. Mark review stale
        let stale_req = ApiRequest {
            id: "rev_stale_1".into(),
            method: METHOD_REVIEWS_MARK_STALE.into(),
            params: serde_json::json!({ "review_id": record.id }),
        };
        let stale_resp = dispatcher.handle_request(stale_req).await;
        assert!(stale_resp.is_success());
        assert_eq!(
            stale_resp
                .result
                .unwrap()
                .get("marked_stale")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        // 5. Verify fetched record is now stale
        let verify_get_req = ApiRequest {
            id: "rev_get_2".into(),
            method: METHOD_REVIEWS_GET.into(),
            params: serde_json::json!({ "review_id": record.id }),
        };
        let verify_get_resp = dispatcher.handle_request(verify_get_req).await;
        assert!(verify_get_resp.is_success());
        let verified: Option<ReviewerRecord> =
            serde_json::from_value(verify_get_resp.result.unwrap()).unwrap();
        assert!(!verified.unwrap().is_fresh);
    }

    #[tokio::test]
    async fn test_synthesis_handoff_api_lifecycle() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let mem_conn = custos_persistence::DbConnection::open_in_memory().expect("open memory db");
        let research_repo = Arc::new(custos_persistence::ResearchRepository::new(mem_conn));

        // Seed a verified claim (L2) and an ungrounded claim (L0)
        let verified_claim = custos_domain::ResearchClaim {
            id: "claim_v1".into(),
            statement: "Attention layers exhibit $O(N^2)$ quadratic complexity".into(),
            level: custos_domain::ClaimGroundingLevel::L2Verified,
            confidence_score: 0.98,
            invariants: vec!["quadratic_scaling".into()],
            evidence_links: vec![],
            created_at: chrono::Utc::now().timestamp(),
            sealed_proof_uri: Some("cas://bafy_proof_1".into()),
        };
        research_repo.save_claim(&verified_claim).unwrap();

        let ungrounded_claim = custos_domain::ResearchClaim {
            id: "claim_u1".into(),
            statement: "Unverified speculative optimization".into(),
            level: custos_domain::ClaimGroundingLevel::L0Ungrounded,
            confidence_score: 0.1,
            invariants: vec![],
            evidence_links: vec![],
            created_at: chrono::Utc::now().timestamp(),
            sealed_proof_uri: None,
        };
        research_repo.save_claim(&ungrounded_claim).unwrap();

        // Seed a recipe
        let recipe = custos_domain::Recipe::new(
            "Benchmark Attention",
            "cargo bench --bench attention",
            custos_domain::EnvironmentSpec {
                python_version: None,
                requirements: vec![],
                container_image: None,
                hardware: None,
            },
        );
        let recipe_id = recipe.id.clone();
        research_repo.save_recipe(&recipe).unwrap();

        let dispatcher =
            LocalApiDispatcher::new(task_service.clone(), session_manager, bridge_service)
                .with_research(research_repo.clone());

        // 1. Save synthesis proposal
        let save_req = ApiRequest {
            id: "prop_save_1".into(),
            method: METHOD_SYNTHESIS_PROPOSALS_SAVE.into(),
            params: serde_json::json!({
                "title": "Attention Quadratic Optimization",
                "summary": "Synthesize benchmarks and handoff invariant tasks to coding",
                "claim_ids": ["claim_v1", "claim_u1"],
                "recipe_ids": [recipe_id],
                "artifact_paths": ["artifacts/benchmark.csv"],
                "workspace_id": "ws_bench",
                "target_branch": "perf/attention-quad",
            }),
        };
        let save_resp = dispatcher.handle_request(save_req).await;
        assert!(
            save_resp.is_success(),
            "Save proposal failed: {:?}",
            save_resp.error
        );
        let prop: custos_domain::ResearchSynthesisProposal =
            serde_json::from_value(save_resp.result.unwrap()).unwrap();
        assert_eq!(prop.title, "Attention Quadratic Optimization");
        assert_eq!(prop.claims.len(), 2);
        assert_eq!(prop.recipes.len(), 1);
        assert!(
            !prop.caveats.is_empty(),
            "Should generate caveats for ungrounded claim"
        );

        // 2. Get and List proposal
        let get_req = ApiRequest {
            id: "prop_get_1".into(),
            method: METHOD_SYNTHESIS_PROPOSALS_GET.into(),
            params: serde_json::json!({ "id": prop.id }),
        };
        let get_resp = dispatcher.handle_request(get_req).await;
        assert!(get_resp.is_success());
        let fetched: Option<custos_domain::ResearchSynthesisProposal> =
            serde_json::from_value(get_resp.result.unwrap()).unwrap();
        assert_eq!(fetched.unwrap().id, prop.id);

        let list_req = ApiRequest {
            id: "prop_list_1".into(),
            method: METHOD_SYNTHESIS_PROPOSALS_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_resp = dispatcher.handle_request(list_req).await;
        assert!(list_resp.is_success());
        let props: Vec<custos_domain::ResearchSynthesisProposal> =
            serde_json::from_value(list_resp.result.unwrap()).unwrap();
        assert_eq!(props.len(), 1);

        // 3. Test Fail-Closed Rejection: Handoff only ungrounded claim with enforce_verification = true
        let reject_req = ApiRequest {
            id: "handoff_rej_1".into(),
            method: METHOD_SYNTHESIS_HANDOFF_EXECUTE.into(),
            params: serde_json::json!({
                "title": "Premature Handoff",
                "claim_ids": ["claim_u1"],
                "recipe_ids": [],
                "enforce_verification": true,
            }),
        };
        let reject_resp = dispatcher.handle_request(reject_req).await;
        assert!(
            reject_resp.error.is_some(),
            "Fail-closed check should reject unverified L0 claims"
        );
        assert!(reject_resp
            .error
            .unwrap()
            .contains("Fail-closed verification gate"));

        // 4. Test Successful Handoff: includes verified claim and recipe
        let exec_req = ApiRequest {
            id: "handoff_exec_1".into(),
            method: METHOD_SYNTHESIS_HANDOFF_EXECUTE.into(),
            params: serde_json::json!({
                "proposal_id": prop.id,
                "title": "Execute Attention Handoff",
                "claim_ids": ["claim_v1"],
                "recipe_ids": [recipe_id],
                "workspace_id": "ws_bench",
                "target_branch": "perf/attention-quad",
                "enforce_verification": true,
            }),
        };
        let exec_resp = dispatcher.handle_request(exec_req).await;
        assert!(
            exec_resp.is_success(),
            "Handoff execution failed: {:?}",
            exec_resp.error
        );
        let handoff_res: custos_domain::HandoffToCodingResult =
            serde_json::from_value(exec_resp.result.unwrap()).unwrap();
        assert_eq!(handoff_res.verified_claims_count, 1);
        assert_eq!(handoff_res.converted_recipes_count, 1);
        assert_eq!(handoff_res.task_ids.len(), 2);

        // 5. Verify created tasks exist in TaskService
        let task_0 = task_service
            .get_task(&handoff_res.task_ids[0])
            .await
            .unwrap();
        assert!(task_0.is_some());
        let t0 = task_0.unwrap();
        assert!(t0.title.contains("[Research Claim]"));
        assert!(
            t0.contract.is_some(),
            "Task must have contract with evidence requirements"
        );

        let task_1 = task_service
            .get_task(&handoff_res.task_ids[1])
            .await
            .unwrap();
        assert!(task_1.is_some());
        let t1 = task_1.unwrap();
        assert!(t1.title.contains("[Recipe Reproduction]"));

        // 6. Verify proposal marked completed
        let check_prop_req = ApiRequest {
            id: "prop_check_1".into(),
            method: METHOD_SYNTHESIS_PROPOSALS_GET.into(),
            params: serde_json::json!({ "id": prop.id }),
        };
        let check_resp = dispatcher.handle_request(check_prop_req).await;
        let completed_prop: Option<custos_domain::ResearchSynthesisProposal> =
            serde_json::from_value(check_resp.result.unwrap()).unwrap();
        assert_eq!(
            completed_prop.unwrap().status,
            custos_domain::SynthesisProposalStatus::HandoffCompleted
        );
    }

    #[tokio::test]
    async fn test_browser_fleet_automation_api() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let fleet_repo = Arc::new(store.fleet_automation().clone());
        let dispatcher = LocalApiDispatcher::new(
            task_service.clone(),
            session_manager.clone(),
            bridge_service.clone(),
        )
        .with_fleet_automation(fleet_repo);

        // 1. Browser metadata is durable, but effectful adapter operations fail closed.
        let s_req = ApiRequest {
            id: "bs_1".into(),
            method: METHOD_BROWSER_SESSIONS_CREATE.into(),
            params: serde_json::json!({ "name": "Test Session", "workspace_id": "ws_1" }),
        };
        let s_res = dispatcher.handle_request(s_req).await;
        assert!(s_res.is_success());
        let session: BrowserSession = serde_json::from_value(s_res.result.unwrap()).unwrap();
        assert_eq!(session.name, "Test Session");

        let t_req = ApiRequest {
            id: "bt_1".into(),
            method: METHOD_BROWSER_TABS_CREATE.into(),
            params: serde_json::json!({ "session_id": session.id, "url": "https://example.com" }),
        };
        let t_res = dispatcher.handle_request(t_req).await;
        assert!(t_res.is_success());
        let tab: BrowserTab = serde_json::from_value(t_res.result.unwrap()).unwrap();
        assert_eq!(tab.url, "https://example.com");
        assert_eq!(tab.status, custos_domain::BrowserTabStatus::Idle);

        let list_tabs_req = ApiRequest {
            id: "bt_list_1".into(),
            method: METHOD_BROWSER_TABS_LIST.into(),
            params: serde_json::json!({ "session_id": session.id }),
        };
        let list_tabs_res = dispatcher.handle_request(list_tabs_req).await;
        assert!(list_tabs_res.is_success());
        let tabs: Vec<BrowserTab> = serde_json::from_value(list_tabs_res.result.unwrap()).unwrap();
        assert_eq!(tabs.len(), 1);

        let nav_req = ApiRequest {
            id: "bt_nav_1".into(),
            method: METHOD_BROWSER_TABS_NAVIGATE.into(),
            params: serde_json::json!({ "tab_id": tab.id, "url": "https://example.com/docs" }),
        };
        let nav_res = dispatcher.handle_request(nav_req).await;
        assert!(!nav_res.is_success());
        assert!(nav_res.error.unwrap().contains("browser adapter"));

        let snap_req = ApiRequest {
            id: "bt_snap_1".into(),
            method: METHOD_BROWSER_TABS_SNAPSHOT.into(),
            params: serde_json::json!({
                "tab_id": tab.id,
                "url": "https://example.com/docs",
                "title": "Example Docs",
                "dom_tree_summary": "<main><h1>Documentation</h1></main>",
                "text_content": "Documentation contents",
                "links": ["https://example.com/about"],
                "viewport_width": 1280,
                "viewport_height": 800,
                "screenshot_uri": null,
                "timestamp": chrono::Utc::now().timestamp_millis()
            }),
        };
        let snap_res = dispatcher.handle_request(snap_req).await;
        assert!(!snap_res.is_success());
        assert!(snap_res.error.unwrap().contains("browser adapter"));

        let close_req = ApiRequest {
            id: "bt_close_1".into(),
            method: METHOD_BROWSER_TABS_CLOSE.into(),
            params: serde_json::json!({ "tab_id": tab.id }),
        };
        let close_res = dispatcher.handle_request(close_req).await;
        assert!(close_res.is_success());

        // 2. Fleet inventory is durable, while probe/exec require a real transport.
        let reg_host_req = ApiRequest {
            id: "host_reg_1".into(),
            method: METHOD_FLEET_HOSTS_REGISTER.into(),
            params: serde_json::json!({
                "name": "Cluster Node 01",
                "host": "127.0.0.1",
                "port": 22,
                "user": "ubuntu",
                "labels": { "gpu": "h100" }
            }),
        };
        let reg_host_res = dispatcher.handle_request(reg_host_req).await;
        assert!(reg_host_res.is_success());
        let host: RemoteHostNode = serde_json::from_value(reg_host_res.result.unwrap()).unwrap();
        assert_eq!(host.name, "Cluster Node 01");

        let ping_req = ApiRequest {
            id: "host_ping_1".into(),
            method: METHOD_FLEET_HOSTS_PING.into(),
            params: serde_json::json!({ "host_id": host.id }),
        };
        let ping_res = dispatcher.handle_request(ping_req).await;
        assert!(!ping_res.is_success());
        assert!(ping_res.error.unwrap().contains("transport adapter"));

        let exec_fleet_req = ApiRequest {
            id: "fleet_exec_1".into(),
            method: METHOD_FLEET_EXEC.into(),
            params: serde_json::json!({
                "host_id": host.id,
                "command": "nvidia-smi --query-gpu=name --format=csv"
            }),
        };
        let exec_fleet_res = dispatcher.handle_request(exec_fleet_req).await;
        assert!(!exec_fleet_res.is_success());
        assert!(exec_fleet_res.error.unwrap().contains("Remote execution"));

        // 3. Automation definitions are durable, while running needs an executor.
        let create_job_req = ApiRequest {
            id: "job_create_1".into(),
            method: METHOD_AUTOMATION_JOBS_CREATE.into(),
            params: serde_json::json!({
                "name": "Nightly Regression Invariants",
                "spec": {
                    "target_type": "local_process",
                    "command_or_script": "cargo test --workspace --offline",
                    "timeout_secs": 600,
                    "env": {},
                    "required_evidence": ["INV-STABILITY-01"]
                },
                "trigger": { "type": "manual" }
            }),
        };
        let create_job_res = dispatcher.handle_request(create_job_req).await;
        assert!(create_job_res.is_success());
        let job: HeadlessAutomationJob =
            serde_json::from_value(create_job_res.result.unwrap()).unwrap();
        assert_eq!(job.name, "Nightly Regression Invariants");
        assert_eq!(job.status, HeadlessJobStatus::Pending);

        let run_job_req = ApiRequest {
            id: "job_run_1".into(),
            method: METHOD_AUTOMATION_JOBS_RUN.into(),
            params: serde_json::json!({ "job_id": job.id }),
        };
        let run_job_res = dispatcher.handle_request(run_job_req).await;
        assert!(!run_job_res.is_success());
        assert!(run_job_res.error.unwrap().contains("executor"));

        let persisted = store
            .fleet_automation()
            .get_headless_job(&job.id)
            .unwrap()
            .unwrap();
        assert_eq!(persisted.status, HeadlessJobStatus::Pending);
        assert_eq!(persisted.exit_code, None);
    }

    #[tokio::test]
    async fn test_provider_and_model_catalog_api_lifecycle() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_providers(Arc::new(store.providers().clone()));

        // 1. Save provider with full protocol fields
        let save_req = ApiRequest {
            id: "p_save_1".into(),
            method: METHOD_PROVIDERS_SAVE.into(),
            params: serde_json::json!({
                "id": "p_anthropic_custom",
                "name": "Custom Anthropic Claude",
                "service_type": "anthropic",
                "api_key": "sk-ant-secret-key-123456789",
                "default_model": "claude-3-7-sonnet",
                "context_window": 200000,
                "fast_mode": true
            }),
        };
        let save_res = dispatcher.handle_request(save_req).await;
        assert!(save_res.is_success());

        // 2. Get provider
        let get_req = ApiRequest {
            id: "p_get_1".into(),
            method: METHOD_PROVIDERS_GET.into(),
            params: serde_json::json!({ "id": "p_anthropic_custom" }),
        };
        let get_res = dispatcher.handle_request(get_req).await;
        assert!(get_res.is_success());
        let cfg: custos_domain::ProviderConfig =
            serde_json::from_value(get_res.result.unwrap()).unwrap();
        assert_eq!(cfg.id, "p_anthropic_custom");
        assert_eq!(cfg.default_model, Some("claude-3-7-sonnet".into()));
        assert_eq!(cfg.context_window, Some(200000));
        assert_eq!(cfg.fast_mode, Some(true));

        // 3. List providers
        let list_req = ApiRequest {
            id: "p_list_1".into(),
            method: METHOD_PROVIDERS_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_res = dispatcher.handle_request(list_req).await;
        assert!(list_res.is_success());
        let providers: Vec<custos_domain::ProviderConfig> =
            serde_json::from_value(list_res.result.unwrap()).unwrap();
        assert!(providers.iter().any(|p| p.id == "p_anthropic_custom"));

        // 4. Test Model Catalog
        let catalog_req = ApiRequest {
            id: "cat_1".into(),
            method: METHOD_MODELS_CATALOG.into(),
            params: serde_json::json!({ "provider_type": "anthropic" }),
        };
        let catalog_res = dispatcher.handle_request(catalog_req).await;
        assert!(catalog_res.is_success());
        let catalog: custos_domain::ModelCatalogResult =
            serde_json::from_value(catalog_res.result.unwrap()).unwrap();
        assert!(!catalog.models.is_empty());
        assert!(catalog.models.iter().any(|m| m.id == "claude-3-7-sonnet"));

        // 5. Test Model Pricing
        let pricing_req = ApiRequest {
            id: "price_1".into(),
            method: METHOD_MODELS_PRICING.into(),
            params: serde_json::json!({
                "model_id": "claude-3-7-sonnet",
                "input_tokens": 100000,
                "output_tokens": 10000,
                "cache_read_tokens": 50000
            }),
        };
        let pricing_res = dispatcher.handle_request(pricing_req).await;
        assert!(pricing_res.is_success());
        let price_body = pricing_res.result.unwrap();
        let cost = price_body
            .get("estimated_cost_usd")
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((cost - 0.465).abs() < 1e-4);

        // 6. Delete Provider
        let del_req = ApiRequest {
            id: "del_1".into(),
            method: METHOD_PROVIDERS_DELETE.into(),
            params: serde_json::json!({ "id": "p_anthropic_custom" }),
        };
        let del_res = dispatcher.handle_request(del_req).await;
        assert!(del_res.is_success());
        assert_eq!(
            del_res.result.unwrap().get("deleted").unwrap().as_bool(),
            Some(true)
        );
    }

    #[tokio::test]
    async fn test_oauth_api_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("custos_oauth_test.db");
        let store =
            Arc::new(custos_persistence::SqliteTaskStore::new(&db_path.to_string_lossy()).unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_providers(Arc::new(store.providers().clone()));

        // 1. Authorize URL generation
        let auth_req = ApiRequest {
            id: "oa_1".into(),
            method: METHOD_OAUTH_AUTHORIZE.into(),
            params: serde_json::json!({
                "provider_id": "openai",
                "client_id": "custos-test-client",
                "redirect_uri": "http://localhost:1420/auth/callback"
            }),
        };
        let auth_res = dispatcher.handle_request(auth_req).await;
        assert!(
            auth_res.is_success(),
            "auth_res failed: {:?}",
            auth_res.error
        );
        let auth_val = auth_res.result.unwrap();
        let auth_url = auth_val.get("authorization_url").unwrap().as_str().unwrap();
        assert!(auth_url.contains("client_id=custos-test-client"));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
        assert!(
            auth_val
                .get("code_verifier")
                .unwrap()
                .as_str()
                .unwrap()
                .len()
                >= 43
        );

        // 2. Query empty OAuth token
        let get_req = ApiRequest {
            id: "oa_2".into(),
            method: METHOD_OAUTH_GET.into(),
            params: serde_json::json!({ "provider_id": "openai" }),
        };
        let get_res = dispatcher.handle_request(get_req).await;
        assert!(get_res.is_success());
        assert!(get_res.result.unwrap().is_null());

        // 3. Save mock OAuth token into database directly to simulate successful exchange
        let now = chrono::Utc::now().timestamp_millis();
        let token_record = custos_domain::OAuthTokenRecord {
            provider_id: "openai".into(),
            service_type: "openai".into(),
            access_token: "sk-proj-test-oauth-access-token".into(),
            refresh_token: Some("rt-test-refresh-token".into()),
            expires_at: now + 3600_000,
            token_type: "Bearer".into(),
            scope: Some("openid profile email model.request".into()),
            created_at: now,
            updated_at: now,
        };
        store.providers().save_oauth_token(&token_record).unwrap();

        // 4. Query token via v1.oauth.get
        let get_req_2 = ApiRequest {
            id: "oa_3".into(),
            method: METHOD_OAUTH_GET.into(),
            params: serde_json::json!({ "provider_id": "openai" }),
        };
        let get_res_2 = dispatcher.handle_request(get_req_2).await;
        assert!(get_res_2.is_success());
        let tok_val = get_res_2.result.unwrap();
        assert_eq!(tok_val.get("connected").unwrap().as_bool(), Some(true));
        assert_eq!(
            tok_val.get("has_refresh_token").unwrap().as_bool(),
            Some(true)
        );
        assert!(tok_val.get("access_token").is_none());
        assert!(tok_val.get("refresh_token").is_none());

        // 5. Query v1.llm.status -> should report OAuth PKCE active
        let llm_req = ApiRequest {
            id: "oa_4".into(),
            method: "v1.llm.status".into(),
            params: serde_json::json!({}),
        };
        let llm_res = dispatcher.handle_request(llm_req).await;
        assert!(llm_res.is_success());
        let llm_body = llm_res.result.unwrap();
        assert_eq!(llm_body.get("configured").unwrap().as_bool(), Some(true));
        assert_eq!(
            llm_body.get("active_provider").unwrap().as_str(),
            Some("OpenAI (OAuth PKCE)")
        );

        // 6. Delete OAuth token via v1.oauth.delete
        let del_req = ApiRequest {
            id: "oa_5".into(),
            method: METHOD_OAUTH_DELETE.into(),
            params: serde_json::json!({ "provider_id": "openai" }),
        };
        let del_res = dispatcher.handle_request(del_req).await;
        assert!(del_res.is_success());
        assert_eq!(
            del_res.result.unwrap().get("deleted").unwrap().as_bool(),
            Some(true)
        );

        // 7. Verify token is gone
        let get_req_3 = ApiRequest {
            id: "oa_6".into(),
            method: METHOD_OAUTH_GET.into(),
            params: serde_json::json!({ "provider_id": "openai" }),
        };
        let get_res_3 = dispatcher.handle_request(get_req_3).await;
        assert!(get_res_3.is_success());
        assert!(get_res_3.result.unwrap().is_null());
    }
}
