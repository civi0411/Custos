//! Custos Local API Contracts & Dispatcher
//!
//! Protocol types and dispatcher for IPC communication between CLI / UI / VS Code and custosd.

use std::sync::Arc;
use base64::Engine;
use custos_domain::DomainError;

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
    METHOD_BROWSER_TABS_CLICK, METHOD_BROWSER_TABS_CLOSE, METHOD_BROWSER_TABS_CREATE,
    METHOD_BROWSER_TABS_FILL, METHOD_BROWSER_TABS_LIST, METHOD_BROWSER_TABS_NAVIGATE,
    METHOD_BROWSER_TABS_SCREENSHOT, METHOD_BROWSER_TABS_SNAPSHOT, METHOD_CAPABILITIES_GET,
    METHOD_CAPABILITIES_LIST, METHOD_EMULATOR_DEVICES_LIST, METHOD_EMULATOR_LAUNCH,
    METHOD_EMULATOR_NAVIGATE, METHOD_EMULATOR_SCREENSHOT, METHOD_EMULATOR_SNAPSHOT,
    METHOD_EMULATOR_TAP, METHOD_FLEET_EXEC, METHOD_FLEET_HOSTS_LIST, METHOD_FLEET_HOSTS_PING,
    METHOD_FLEET_HOSTS_REGISTER, METHOD_HARNESS_CANCEL, METHOD_HARNESS_GET, METHOD_HARNESS_LIST,
    METHOD_HARNESS_RUN_NATIVE, METHOD_HARNESS_STEER, METHOD_MODELS_CATALOG, METHOD_MODELS_PRICING,
    METHOD_MODELS_PROBE, METHOD_NOTEBOOK_CELLS_LIST, METHOD_NOTEBOOK_CELLS_SAVE,
    METHOD_NOTEBOOK_EXECUTE, METHOD_NOTEBOOK_INTERRUPT, METHOD_NOTEBOOK_RESET,
    METHOD_NOTEBOOK_STATUS, METHOD_NOTES_GET, METHOD_NOTES_HISTORY, METHOD_NOTES_LIST,
    METHOD_NOTES_SAVE, METHOD_OAUTH_AUTHORIZE, METHOD_OAUTH_CANCEL, METHOD_OAUTH_DELETE,
    METHOD_OAUTH_EXCHANGE, METHOD_OAUTH_GET, METHOD_OAUTH_REFRESH, METHOD_OAUTH_STATUS,
    METHOD_PROVIDERS_DELETE, METHOD_PROVIDERS_GET, METHOD_PROVIDERS_LIST, METHOD_PROVIDERS_SAVE,
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
    METHOD_WORKSPACES_RECOVER, METHOD_WORKSPACES_DETECT, METHOD_WORKSPACES_PICK_FOLDER,
    METHOD_WORKSPACES_CANDIDATES_SPAWN, METHOD_WORKSPACES_CANDIDATES_COMPARE,
    METHOD_WORKSPACES_INTEGRATE, METHOD_SETTINGS_GET, METHOD_SETTINGS_SET,
    METHOD_WORKSPACE_DIFF, METHOD_WORKSPACE_FILES_READ,
    METHOD_WORKSPACE_FILES_TREE, METHOD_WORKSPACE_FILES_WRITE, METHOD_WORKSPACE_FILE_DIFF,
    METHOD_WORKSPACE_GIT_COMMIT, METHOD_WORKSPACE_GIT_DISCARD, METHOD_WORKSPACE_GIT_STAGE, METHOD_WORKSPACE_GIT_UNSTAGE,
    JobOperationalStatus, PacksJobCatalogItem, METHOD_PACKS_JOBS_GET, METHOD_PACKS_JOBS_LIST,
    METHOD_PACKS_JOBS_RUN, METHOD_PACKS_MANIFESTS_LIST, METHOD_PACKS_ROLES_LIST,
    METHOD_ASSISTANT_DRAFTS_DELETE, METHOD_ASSISTANT_DRAFTS_GET, METHOD_ASSISTANT_DRAFTS_LIST,
    METHOD_ASSISTANT_DRAFTS_SAVE, METHOD_ASSISTANT_OUTBOX_CREATE, METHOD_ASSISTANT_OUTBOX_APPROVE, METHOD_ASSISTANT_OUTBOX_DISPATCH,
    METHOD_ASSISTANT_OUTBOX_LIST, METHOD_RESOURCES_CAPABILITIES, METHOD_WORKSPACES_AGENT_STEER,
    METHOD_GITHUB_AUTH_STATUS, METHOD_GITHUB_VIEWER, METHOD_GITHUB_LOGIN,
    METHOD_GITHUB_ACCOUNTS, METHOD_GIT_REPOSITORY, METHOD_GIT_SWITCH, METHOD_GIT_FETCH, METHOD_GIT_ACCOUNT, METHOD_GIT_GITHUB,
};
use custos_adapters::harness::HarnessRegistry;
use custos_bridge::{AttachMode, BridgePort, BridgeService};
use custos_core::contracts::storage::OutboxPort;
use custos_core::contracts::terminal::TerminalPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_core::{AdvanceTask, CancelTask, CreateTask, TaskService};
use custos_domain::{
    new_id, validate_browser_url, ArtifactLineageNode, AssistantDraft, BrowserConsoleEntry,
    BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab, BrowserTabStatus,
    CapabilityDescriptor, CapabilityGroup, CellExecutionStatus, ClaimGroundingLevel, ClaimHandoffSummary,
    CreateHeadlessJobParams, ExecuteCellParams, FleetExecReceipt, HandoffToCodingParams,
    HandoffToCodingResult, HeadlessAutomationJob, HeadlessJobStatus, NoteRecord, NotebookCell,
    OutboxActionItem, PassageAnchor, Recipe, RecipeHandoffSummary, RecordReviewParams,
    RegisterHostParams, RemoteHostNode, RemoteHostStatus, ResearchClaim, ResearchExperimentRun,
    ResearchSynthesisProposal, ResourceActionCapability, ReviewStatus, ReviewerRecord,
    SaveSynthesisProposalParams, SessionId, SessionMode, SourceRecord, SshAuthMethod, TaskContract,
    TaskStatus,
};
use custos_persistence::repositories::{AssistantRepository, OutboxRepository};
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
        "copilot" => Some("https://api.githubcopilot.com"),
        "antigravity" => Some("https://cloudcode-pa.googleapis.com"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "local" | "ollama" => Some("http://localhost:11434"),
        "lmstudio" | "lm-studio" => Some("http://127.0.0.1:1234/v1"),
        _ => None,
    }
}

static CAFFEINATE_STATE: std::sync::Mutex<(String, Option<std::process::Child>)> =
    std::sync::Mutex::new((String::new(), None));

pub fn resolve_agent_browser_binary() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("AGENT_BROWSER_BINARY") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/agent-browser");
        if local_bin.exists() {
            return Some(local_bin);
        }
        let orca_bin = home.join(
            "Project/AgentHub/orca/node_modules/agent-browser/bin/agent-browser-darwin-arm64",
        );
        if orca_bin.exists() {
            return Some(orca_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("agent-browser")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = std::path::PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_ssh_binary() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("SSH_BINARY") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in [
        "/usr/bin/ssh",
        "/usr/local/bin/ssh",
        "/opt/homebrew/bin/ssh",
    ] {
        let p = std::path::PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("ssh")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = std::path::PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

static GITHUB_LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn github_login_command(path: &std::path::Path) -> String {
    let executable = path.to_string_lossy().replace('\'', "'\\''");
    format!("exec '{executable}' auth login --hostname github.com --git-protocol https --web --skip-ssh-key")
}

fn github_auth_metadata(success: bool, stdout: &[u8]) -> serde_json::Value {
    let user = serde_json::from_slice::<serde_json::Value>(stdout).ok();
    if success && user.as_ref().and_then(|u| u.get("login")).and_then(|v| v.as_str()).is_some_and(|login| !login.trim().is_empty()) {
        // Whitelist profile metadata; never forward arbitrary CLI output.
        let user = user.as_ref();
        return serde_json::json!({"authenticated": true, "source": "gh_cli", "installed": true, "user": {
            "login": user.and_then(|u| u.get("login")),
            "name": user.and_then(|u| u.get("name")),
            "avatar_url": user.and_then(|u| u.get("avatar_url")),
            "html_url": user.and_then(|u| u.get("html_url"))
        }});
    }
    serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true,
        "error": "GitHub account could not be verified. Connect with gh, or check network access and retry."})
}

pub fn resolve_gh_binary() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("GH_BINARY") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in [
        "/opt/homebrew/bin/gh",
        "/usr/local/bin/gh",
        "/usr/bin/gh",
    ] {
        let p = std::path::PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/gh");
        if local_bin.exists() {
            return Some(local_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("gh")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = std::path::PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_git_binary() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("GIT_BINARY") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    for candidate in [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/git",
    ] {
        let p = std::path::PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    std::path::PathBuf::from("git")
}

/// Parses a Git remote URL into (host, owner, repo, avatar_url)
/// Supports SSH, HTTPS, git://, and custom enterprise GitHub hosts.
pub fn parse_github_remote(remote_url: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return (None, None, None, None);
    }

    let s = raw.strip_suffix(".git").unwrap_or(raw);

    // Case 1: SCP-like syntax: [user@]host:owner/repo (only when no scheme :// is present)
    if !s.contains("://") {
        if let Some((left, path)) = s.split_once(':') {
            if !left.contains('/') {
                let host = left.split('@').last().unwrap_or(left).trim();
                let path = path.trim().trim_start_matches('/');
                let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
                if parts.len() >= 2 {
                    let owner = parts[0].to_string();
                    let repo = parts[1..].join("/");
                    let avatar = if host == "github.com" {
                        Some(format!("https://github.com/{}.png", owner))
                    } else {
                        None
                    };
                    return (Some(host.to_string()), Some(owner), Some(repo), avatar);
                }
            }
        }
    }

    // Case 2: URL syntax: [scheme://][user[:pass]@]host[:port]/owner/repo
    let after_scheme = if let Some(idx) = s.find("://") {
        &s[idx + 3..]
    } else {
        s
    };

    let after_auth = if let Some((_, rest)) = after_scheme.split_once('@') {
        rest
    } else {
        after_scheme
    };

    if let Some((host_part, path_part)) = after_auth.split_once('/') {
        let host = host_part.split(':').next().unwrap_or(host_part).trim();
        let parts: Vec<&str> = path_part.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            let owner = parts[0].to_string();
            let repo = parts[1..].join("/");
            let avatar = if host == "github.com" {
                Some(format!("https://github.com/{}.png", owner))
            } else {
                None
            };
            return (Some(host.to_string()), Some(owner), Some(repo), avatar);
        }
    }

    (None, None, None, None)
}

/// Resolves actual gitdir and common gitdir for a path (handling both .git directory and .git file for worktrees/submodules)
pub async fn resolve_deep_git_paths(path: &std::path::Path) -> Option<(std::path::PathBuf, std::path::PathBuf, bool)> {
    let dot_git = path.join(".git");
    if !dot_git.exists() {
        return None;
    }

    if dot_git.is_dir() {
        // Standard repository
        return Some((dot_git.clone(), dot_git, false));
    }

    if dot_git.is_file() {
        // Linked worktree or submodule
        if let Ok(content) = tokio::fs::read_to_string(&dot_git).await {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("gitdir:") {
                    let gitdir_str = rest.trim();
                    let gitdir_path = if std::path::Path::new(gitdir_str).is_absolute() {
                        std::path::PathBuf::from(gitdir_str)
                    } else {
                        path.join(gitdir_str)
                    };
                    let gitdir_canonical = gitdir_path.canonicalize().unwrap_or(gitdir_path);

                    let commondir_file = gitdir_canonical.join("commondir");
                    let commondir_canonical = if commondir_file.is_file() {
                        if let Ok(common_content) = tokio::fs::read_to_string(&commondir_file).await {
                            let common_str = common_content.trim();
                            let common_path = if std::path::Path::new(common_str).is_absolute() {
                                std::path::PathBuf::from(common_str)
                            } else {
                                gitdir_canonical.join(common_str)
                            };
                            common_path.canonicalize().unwrap_or(common_path)
                        } else {
                            gitdir_canonical.clone()
                        }
                    } else {
                        gitdir_canonical.clone()
                    };

                    return Some((gitdir_canonical, commondir_canonical, true));
                }
            }
        }
    }

    None
}

/// Lists all worktrees for a Git repository via `git worktree list --porcelain`
pub async fn list_git_worktrees(git_bin: &std::path::Path, repo_path: &str) -> Vec<serde_json::Value> {
    let mut worktrees = Vec::new();
    let output = match tokio::process::Command::new(git_bin)
        .current_dir(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .await
    {
        Ok(out) if out.status.success() => out,
        _ => return worktrees,
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;
    let mut current_detached = false;
    let mut is_first = true;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(p) = current_path.take() {
                worktrees.push(serde_json::json!({
                    "path": p,
                    "head": current_head.take(),
                    "branch": current_branch.take(),
                    "is_main": is_first,
                    "is_bare": current_bare,
                    "is_detached": current_detached,
                }));
                is_first = false;
            }
            current_bare = false;
            current_detached = false;
            continue;
        }

        if let Some(p) = trimmed.strip_prefix("worktree ") {
            current_path = Some(p.trim().to_string());
        } else if let Some(h) = trimmed.strip_prefix("HEAD ") {
            current_head = Some(h.trim().to_string());
        } else if let Some(b) = trimmed.strip_prefix("branch ") {
            let b_ref = b.trim();
            let short = b_ref.strip_prefix("refs/heads/").unwrap_or(b_ref);
            current_branch = Some(short.to_string());
        } else if trimmed == "bare" {
            current_bare = true;
        } else if trimmed == "detached" {
            current_detached = true;
        }
    }

    if let Some(p) = current_path.take() {
        worktrees.push(serde_json::json!({
            "path": p,
            "head": current_head.take(),
            "branch": current_branch.take(),
            "is_main": is_first,
            "is_bare": current_bare,
            "is_detached": current_detached,
        }));
    }

    worktrees
}

// Effectful Browser and Remote Fleet operations stay fail-closed until their
// transports are composed behind governed capability ports. Keeping this
// decision in one place prevents persisted inventory/session metadata from
// being mistaken for an executable capability.
fn browser_execution_adapter_connected() -> bool {
    std::env::var("CUSTOS_BROWSER_CONNECTED")
        .map(|v| v != "0" && v != "false")
        .unwrap_or_else(|_| resolve_agent_browser_binary().is_some())
}

fn remote_execution_adapter_connected() -> bool {
    std::env::var("CUSTOS_REMOTE_FLEET_CONNECTED")
        .map(|v| v != "0" && v != "false")
        .unwrap_or_else(|_| resolve_ssh_binary().is_some())
}

/// Local API Dispatcher wrapping TaskService, SessionManager, BridgeService, WorkflowPort, ResearchRepository, ProviderRepository, TerminalCoordinator, and WorkspaceFilesPort for IPC callers.
pub struct LocalApiDispatcher {
    task_service: Arc<TaskService>,
    session_manager: Arc<SessionManager>,
    bridge_service: Arc<BridgeService>,
    workflow: Option<Arc<dyn WorkflowPort>>,
    usage_runs: Option<Arc<dyn custos_core::contracts::storage::RunPort>>,
    workspace: Option<Arc<WorkspaceCoordinator>>,
    candidate_mutation: tokio::sync::Mutex<()>,
    research: Option<Arc<ResearchRepository>>,
    providers: Option<Arc<ProviderRepository>>,
    credentials: Option<Arc<crate::credentials::ProviderCredentials>>,
    terminal: Arc<TerminalCoordinator>,
    files: Arc<dyn WorkspaceFilesPort>,
    harnesses: Arc<HarnessRegistry>,
    python_kernel: Option<Arc<PythonKernelCoordinator>>,
    fleet_automation: Option<Arc<FleetAutomationRepository>>,
    model_availability: custos_adapters::providers::catalog::ModelAvailability,
    model_catalog: custos_adapters::providers::catalog::ModelCatalogService,
    oauth_callback_server: Option<Arc<crate::oauth_callback_server::OAuthCallbackServer>>,
    pack_registry: Arc<custos_packs::DomainPackRegistry>,
    assistant: Option<Arc<AssistantRepository>>,
    outbox: Option<Arc<OutboxRepository>>,
    sqlite_store: Option<Arc<custos_persistence::SqliteTaskStore>>,
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
            usage_runs: None,
            workspace: None,
            candidate_mutation: tokio::sync::Mutex::new(()),
            research: None,
            providers: None,
            credentials: None,
            terminal: Arc::new(TerminalCoordinator::new()),
            files: Arc::new(WorkspaceFilesCoordinator::new()),
            harnesses: Arc::new(HarnessRegistry::new()),
            python_kernel: Some(Arc::new(PythonKernelCoordinator::new())),
            fleet_automation: None,
            model_availability: Default::default(),
            model_catalog: custos_adapters::providers::catalog::ModelCatalogService::new(),
            oauth_callback_server: None,
            pack_registry: Arc::new(custos_packs::create_default_pack_registry()),
            assistant: None,
            outbox: None,
            sqlite_store: None,
        }
    }

    pub fn with_model_availability(mut self, availability: custos_adapters::providers::catalog::ModelAvailability) -> Self {
        self.model_availability = availability;
        self
    }

    pub fn with_sqlite_store(mut self, store: Arc<custos_persistence::SqliteTaskStore>) -> Self {
        self.sqlite_store = Some(store);
        self
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

    pub fn with_usage_runs(mut self, runs: Arc<dyn custos_core::contracts::storage::RunPort>) -> Self {
        self.usage_runs = Some(runs);
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

    pub fn with_assistant(mut self, assistant: Arc<AssistantRepository>) -> Self {
        self.assistant = Some(assistant);
        self
    }

    pub fn with_outbox(mut self, outbox: Arc<OutboxRepository>) -> Self {
        self.outbox = Some(outbox);
        self
    }

    pub fn with_providers(mut self, providers: Arc<ProviderRepository>) -> Self {
        #[cfg(test)] { self.credentials=Some(crate::credential_tests::fake_credentials(providers.clone())); }
        self.providers = Some(providers);
        self
    }

    pub fn with_credentials(mut self, credentials: Arc<crate::credentials::ProviderCredentials>) -> Self {
        self.credentials = Some(credentials);
        self
    }
    fn credential_service(&self) -> Result<&Arc<crate::credentials::ProviderCredentials>, DomainError> {
        self.credentials.as_ref().ok_or_else(|| DomainError::Validation(
            "secure_storage_unavailable: Credential service is not configured".into()))
    }
    async fn read_provider_token(&self, account: &str) -> Result<Option<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.get_oauth_token(account).await
    }
    async fn refresh_provider_token(&self, previous: &custos_domain::OAuthTokenRecord, refreshed: &custos_domain::OAuthTokenRecord) -> Result<(), DomainError> {
        self.credential_service()?.save_refreshed(previous, refreshed).await
    }
    async fn write_provider_connection(&self, token: &custos_domain::OAuthTokenRecord, config: &custos_domain::ProviderConfig) -> Result<(), DomainError> {
        self.credential_service()?.save_oauth_connection(token, Some(config)).await
    }
    async fn remove_provider_credential(&self, account: &str, remove_provider: bool) -> Result<(), DomainError> {
        self.credential_service()?.delete(account, remove_provider).await
    }
    async fn provider_tokens(&self) -> Result<Vec<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.list_oauth_tokens().await
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

    pub fn with_packs(mut self, pack_registry: Arc<custos_packs::DomainPackRegistry>) -> Self {
        self.pack_registry = pack_registry;
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
                    vec!["recipes.list", "recipes.get", "executions.list", "executions.get", "executions.resume.assess"],
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
                    Some("runs"),
                    vec!["list", "save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.runs",
                    "Runs",
                    CapabilityGroup::Compute,
                    "Experiment attempts, environments, outputs, and execution receipts.",
                    Some("runs"),
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
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "compute.pty",
                    "Terminal",
                    CapabilityGroup::Compute,
                    "Bounded PTY streams scoped to an execution workspace.",
                    Some("terminal"),
                    vec!["spawn", "write", "resize", "read", "terminate", "list", "get"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "compute.pty", "Terminal", CapabilityGroup::Compute,
                    "Bounded PTY streams scoped to an execution workspace.", Some("terminal"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "code.files", "Files", CapabilityGroup::Code,
                    "Repository tree, file buffers, and source anchor inspection.", Some("files"),
                    vec!["tree", "read", "write"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "code.files", "Files", CapabilityGroup::Code,
                    "Repository tree, file buffers, and source anchor inspection.", Some("files"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "code.changes", "Changes", CapabilityGroup::Code,
                    "Diff review and Git stage decisions in an execution workspace.", Some("changes"),
                    vec!["diff", "file_diff", "stage", "unstage", "discard"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "code.changes", "Changes", CapabilityGroup::Code,
                    "Diff review and Git stage decisions in an execution workspace.", Some("changes"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
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
            if self.workflow.is_some() && self.usage_runs.is_some() {
                CapabilityDescriptor::available(
                    "coordination.kanban", "Activity", CapabilityGroup::Coordination,
                    "Persisted Task runs and live run events.", Some("kanban"),
                    vec!["runs.list", "runs.watch", "runs.cancel"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "coordination.kanban", "Activity", CapabilityGroup::Coordination,
                    "Persisted Task runs and live run events.", Some("kanban"),
                    "Workflow and persisted run projection are required.",
                )
            },
            if self.harnesses.list().iter().any(|harness| harness.is_available) {
                CapabilityDescriptor::available(
                    "coding.agents",
                    "Coding Agents",
                    CapabilityGroup::Code,
                    "Start a registered native coding harness in a selected execution workspace.",
                    Some("agents"),
                    vec!["list_harnesses", "start_run", "cancel_run"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "coding.agents",
                    "Coding Agents",
                    CapabilityGroup::Code,
                    "Inspect registered native coding harnesses.",
                    Some("agents"),
                    "No registered coding harness binary is available on this machine.",
                    vec!["list_harnesses"],
                )
            },
            if self.assistant.is_some() {
                CapabilityDescriptor::available(
                    "assistant.drafts",
                    "Drafts",
                    CapabilityGroup::Personal,
                    "Durable communication drafts stored by the daemon.",
                    Some("drafts"),
                    vec!["list", "get", "save", "delete"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "assistant.drafts",
                    "Drafts",
                    CapabilityGroup::Personal,
                    "Durable communication drafts stored by the daemon.",
                    Some("drafts"),
                    "AssistantRepository is not configured on this daemon instance.",
                )
            },
            if self.outbox.is_some() {
                CapabilityDescriptor::available(
                    "assistant.outbox",
                    "Outbox",
                    CapabilityGroup::Personal,
                    "Inspect, approve, and dispatch recorded external action items and receipts.",
                    Some("outbox"),
                    vec!["list", "create", "approve", "dispatch"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "assistant.outbox",
                    "Outbox",
                    CapabilityGroup::Personal,
                    "Inspect recorded external effect attempts and receipts.",
                    Some("outbox"),
                    "OutboxRepository is not configured on this daemon instance.",
                )
            },
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
                if browser_execution_adapter_connected() {
                    CapabilityDescriptor::available(
                        "browser.tabs",
                        "Browser",
                        CapabilityGroup::Browser,
                        "Governed headless browser automation via agent-browser.",
                        Some("browser"),
                        vec!["sessions.list", "sessions.create", "tabs.list", "tabs.create", "tabs.navigate", "tabs.click", "tabs.fill", "tabs.snapshot", "tabs.screenshot", "tabs.close"],
                    )
                } else {
                    CapabilityDescriptor::degraded(
                        "browser.tabs",
                        "Browser",
                        CapabilityGroup::Browser,
                        "Persisted browser session and tab intents.",
                        Some("browser"),
                        "Browser execution adapter is not connected; navigation and snapshots are refused.",
                        vec!["sessions.list", "sessions.create", "tabs.list", "tabs.create", "tabs.close"],
                    )
                }
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
                if remote_execution_adapter_connected() {
                    CapabilityDescriptor::available(
                        "remote.fleet",
                        "Remote Fleet",
                        CapabilityGroup::Compute,
                        "SSH remote host inventory, ping latency probes, and distributed node execution.",
                        Some("fleet"),
                        vec!["hosts.list", "hosts.register", "hosts.ping", "fleet.exec"],
                    )
                } else {
                    CapabilityDescriptor::degraded(
                        "remote.fleet",
                        "Remote Fleet",
                        CapabilityGroup::Compute,
                        "Persisted SSH host inventory.",
                        Some("fleet"),
                        "SSH transport is not connected; probes and remote execution are refused.",
                        vec!["hosts.list", "hosts.register"],
                    )
                }
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
                CapabilityDescriptor::available(
                    "automation.headless",
                    "Automation",
                    CapabilityGroup::Coordination,
                    "Persisted headless automation jobs with execution dispatch.",
                    Some("automation"),
                    vec!["jobs.list", "jobs.create", "jobs.run"],
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

    pub async fn live_run(&self, session_id: &str, run_id: &str, cursor: u64) -> Result<custos_domain::LiveRunReplay, DomainError> {
        if self.session_manager.get_session(&SessionId(session_id.into())).await.is_none() {
            return Err(DomainError::Validation("Unknown session".into()));
        }
        self.workflow.as_ref().ok_or_else(|| DomainError::Validation("Workflow unavailable".into()))?.live_run(session_id, run_id, cursor).await
    }

    pub async fn handle_request(&self, req: ApiRequest) -> ApiResponse {
        let command_id = req
            .params
            .get("command_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let payload_digest = if command_id.is_some() {
            Some(custos_domain::digest(
                serde_json::to_string(&req.params).unwrap_or_default().as_bytes(),
            ))
        } else {
            None
        };

        if let (Some(cmd_id), Some(digest), Some(store)) = (
            command_id.as_deref(),
            payload_digest.as_deref(),
            self.sqlite_store.as_ref(),
        ) {
            match store.command_ledger().get_command(cmd_id) {
                Ok(Some(entry)) => {
                    if entry.payload_digest == digest {
                        let val = serde_json::from_str(&entry.result_json)
                            .unwrap_or(serde_json::Value::Null);
                        return ApiResponse::success(req.id, val);
                    } else {
                        return ApiResponse::error(
                            req.id,
                            format!("Command ID conflict: '{cmd_id}' already executed with different payload"),
                        );
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(command_id = %cmd_id, error = %e, "Failed to inspect command ledger");
                }
            }
        }

        let method_name = req.method.clone();
        let resp = match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            "v1.tasks.create" => {
                let params: CreateTaskRequest = match serde_json::from_value(req.params.clone()) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let mut meta = params.metadata.unwrap_or_else(|| serde_json::json!({}));
                if let Some(sid) = req.params.get("session_id").and_then(|v| v.as_str()) {
                    if meta.get("session_id").is_none() {
                        meta["session_id"] = serde_json::json!(sid);
                    }
                }

                let cmd = CreateTask {
                    title: params.title,
                    metadata: Some(meta),
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
            "v1.tasks.list" => {
                let session_id_opt = req.params.get("session_id").and_then(|v| v.as_str());
                match self.task_service.list_tasks().await {
                    Ok(tasks) => {
                        let filtered: Vec<_> = if let Some(sid) = session_id_opt {
                            tasks
                                .into_iter()
                                .filter(|t| {
                                    t.metadata.get("session_id").and_then(|v| v.as_str()) == Some(sid)
                                })
                                .collect()
                        } else {
                            tasks
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
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
            "v1.sessions.open_in_lens" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let lens: custos_domain::WorkbenchLens = match req.params.get("lens") {
                    Some(value) => match serde_json::from_value(value.clone()) {
                        Ok(lens) => lens,
                        Err(e) => return ApiResponse::error(req.id, format!("Invalid lens: {e}")),
                    },
                    None => return ApiResponse::error(req.id, "Missing lens param"),
                };
                match self.session_manager.open_in_lens(&session_id, lens).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "status": "opened" })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
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

                match self.task_service.get_task(&task_id).await {
                    Ok(Some(_)) => {}
                    Ok(None) => return ApiResponse::error(req.id, format!("Task {task_id} not found")),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                }
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
            "v1.workflow.session_runs" => {
                let Some(session)=req.params.get("session_id").and_then(|v|v.as_str()) else {return ApiResponse::error(req.id,"Missing session_id");};
                if self.session_manager.get_session(&SessionId(session.to_string())).await.is_none() {return ApiResponse::error(req.id,"Unknown session");}
                let Some(store)=&self.usage_runs else {return ApiResponse::error(req.id,"Run store unavailable");};
                let tasks=match self.task_service.list_tasks().await {Ok(tasks)=>tasks,Err(e)=>return ApiResponse::error(req.id,e.to_string())};
                let mut runs=Vec::new();
                for task in tasks {
                    let task_runs=match store.list_runs_for_task(&task.id).await {Ok(runs)=>runs,Err(e)=>return ApiResponse::error(req.id,e.to_string())};
                    for run in task_runs {
                        if run.metadata.get("session_id").and_then(|v|v.as_str())==Some(session) && (run.metadata.get("background").and_then(|v|v.as_bool())==Some(true) || run.metadata.get("journal_occurred_at").is_some() || !run.status.is_terminal()) {
                            runs.push(serde_json::json!({"run_id":run.id,"model":run.metadata.get("requested_model").filter(|value|!value.is_null()).or_else(||run.metadata.get("harness_id")),"status":run.status.to_string(),"journal_occurred_at":run.metadata.get("journal_occurred_at")}));
                        }
                    }
                }
                ApiResponse::success(req.id,serde_json::json!(runs))
            }
            "v1.sessions.metrics" | "v1.session.metrics" => {
                let session_opt = req.params.get("session_id").and_then(|v| v.as_str());
                let Some(store) = &self.usage_runs else {
                    return ApiResponse::error(req.id, "Run store unavailable");
                };
                let tasks = match self.task_service.list_tasks().await {
                    Ok(tasks) => tasks,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut tokens_in: u64 = 0;
                let mut tokens_out: u64 = 0;
                let mut total_tokens: u64 = 0;
                let mut active_runs: u64 = 0;
                let pending_approvals: u64 = 0;

                for task in tasks {
                    let task_runs = match store.list_runs_for_task(&task.id).await {
                        Ok(runs) => runs,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    for run in task_runs {
                        let matches_session = match session_opt {
                            Some(s) if !s.is_empty() => {
                                run.metadata.get("session_id").and_then(|v| v.as_str()) == Some(s)
                            }
                            _ => true,
                        };
                        if !matches_session {
                            continue;
                        }

                        if !run.status.is_terminal() {
                            active_runs += 1;
                        }

                        if let Some(usage) = run.metadata.get("usage") {
                            let in_tk = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let out_tk = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let tot_tk = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(in_tk + out_tk);
                            tokens_in += in_tk;
                            tokens_out += out_tk;
                            total_tokens += tot_tk;
                        }
                    }
                }

                let estimated_cost_usd = (tokens_in as f64 * 0.000003) + (tokens_out as f64 * 0.000015);
                let rounded_cost = (estimated_cost_usd * 10000.0).round() / 10000.0;

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "session_id": session_opt,
                        "tokens_in": tokens_in,
                        "tokens_out": tokens_out,
                        "total_tokens": total_tokens,
                        "estimated_cost_usd": rounded_cost,
                        "active_runs": active_runs,
                        "pending_approvals": pending_approvals
                    }),
                )
            }
            "v1.system.metrics" | "v1.system.telemetry" => {
                let memory_resident_mb = if let Some(out) = std::process::Command::new("ps")
                    .args(["-A", "-o", "rss=,comm="])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                {
                    let mut total_kb: f64 = 0.0;
                    for line in out.lines() {
                        let lower = line.to_lowercase();
                        if lower.contains("custos") {
                            if let Some(rss_str) = line.split_whitespace().next() {
                                if let Ok(rss) = rss_str.parse::<f64>() {
                                    total_kb += rss;
                                }
                            }
                        }
                    }
                    if total_kb == 0.0 {
                        let pid = std::process::id();
                        let rss_kb: f64 = std::process::Command::new("ps")
                            .args(["-o", "rss=", "-p", &pid.to_string()])
                            .output()
                            .ok()
                            .and_then(|out| String::from_utf8(out.stdout).ok())
                            .and_then(|s| s.trim().parse::<f64>().ok())
                            .unwrap_or(0.0);
                        ((rss_kb / 1024.0) * 10.0).round() / 10.0
                    } else {
                        ((total_kb / 1024.0) * 10.0).round() / 10.0
                    }
                } else {
                    0.0
                };

                let mut active_runs: u64 = 0;
                if let Some(store) = &self.usage_runs {
                    if let Ok(tasks) = self.task_service.list_tasks().await {
                        for task in tasks {
                            if let Ok(runs) = store.list_runs_for_task(&task.id).await {
                                for run in runs {
                                    if !run.status.is_terminal() {
                                        active_runs += 1;
                                    }
                                }
                            }
                        }
                    }
                }

                let active_ports: Vec<u16> = std::process::Command::new("lsof")
                    .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| {
                        let mut ports = std::collections::BTreeSet::new();
                        for line in s.lines().skip(1) {
                            if let Some(part) = line.split_whitespace().nth(8) {
                                if let Some(port_str) = part.rsplit(':').next() {
                                    if let Ok(p) = port_str.parse::<u16>() {
                                        if (1024..=49151).contains(&p) {
                                            ports.insert(p);
                                        }
                                    }
                                }
                            }
                        }
                        ports.into_iter().collect()
                    })
                    .unwrap_or_default();

                let caffeinate_status = {
                    let mut lock = CAFFEINATE_STATE.lock().unwrap();
                    let (mode, child) = &mut *lock;
                    if mode.is_empty() {
                        *mode = "off".to_string();
                    }
                    let is_active = child.as_mut().map(|c| c.try_wait().ok().flatten().is_none()).unwrap_or(false);
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    })
                };

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "memory_resident_mb": memory_resident_mb,
                        "active_runs": active_runs,
                        "active_ports": active_ports,
                        "caffeinate": caffeinate_status
                    }),
                )
            }
            "v1.system.caffeinate" => {
                let mode = req.params.get("mode").and_then(|v| v.as_str()).unwrap_or("off");
                let mut lock = CAFFEINATE_STATE.lock().unwrap();
                let (current_mode, child) = &mut *lock;
                *current_mode = mode.to_string();

                let should_run = mode == "always";
                if should_run {
                    if child.is_none() {
                        *child = std::process::Command::new("caffeinate")
                            .args(["-i", "-s"])
                            .spawn()
                            .ok();
                    }
                } else if let Some(mut c) = child.take() {
                    let _ = c.kill();
                }

                let is_active = child.as_mut().map(|c| c.try_wait().ok().flatten().is_none()).unwrap_or(false);
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    }),
                )
            }
            "v1.workflow.live_run" => {
                let session = req.params.get("session_id").and_then(|v|v.as_str()).unwrap_or("");
                let run = req.params.get("run_id").and_then(|v|v.as_str()).unwrap_or("");
                let cursor = req.params.get("cursor").and_then(|v|v.as_u64()).unwrap_or(0);
                match self.live_run(session,run,cursor).await {
                    Ok(replay) => match serde_json::to_value(replay) { Ok(value) => ApiResponse::success(req.id,value), Err(error) => ApiResponse::error(req.id,error.to_string()) },
                    Err(error) => ApiResponse::error(req.id,error.to_string()),
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

                let resolved_workspace_root = if let Some(ref workspace_id) = params.workspace_id {
                    let workspace = match self
                        .resolve_workspace_for_dispatch(&req.id, workspace_id)
                        .await
                    {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    if !workspace.is_ready() {
                        return ApiResponse::error(
                            req.id,
                            format!("Workspace '{workspace_id}' is not ready for execution"),
                        );
                    }
                    Some(workspace.path)
                } else if params.preferred_mode.as_deref() == Some("native") {
                    return ApiResponse::error(
                        req.id,
                        "Native Task runs require workspace_id; raw workspace paths are not accepted",
                    );
                } else {
                    params.workspace_root.clone()
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
                if let Some(root) = resolved_workspace_root {
                    cmd = cmd.with_workspace_root(root);
                }
                if let Some(ref ws_id) = params.workspace_id {
                    cmd = cmd.with_workspace_id(ws_id.clone());
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

                cmd.reasoning_effort = params.reasoning_effort;
                cmd.provider_account_id = params.provider_account_id;

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

                let background = params.background;
                let result = if background { workflow.start_run_background(cmd).await } else { workflow.start_run(cmd).await };
                match result {
                    Ok(handle) => {
                        if let (Some(ref sid), Some(ref out)) = (params.session_id.as_ref().filter(|_| !background), handle.output.as_ref()) {
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
            METHOD_WORKSPACES_DETECT => {
                let raw_path_str = req.params.get("path").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("");
                let expanded_path = if raw_path_str.is_empty() || raw_path_str == "." {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                } else if raw_path_str.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&raw_path_str[2..])
                    } else {
                        std::path::PathBuf::from(raw_path_str)
                    }
                } else if raw_path_str == "~" {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(raw_path_str))
                } else {
                    std::path::PathBuf::from(raw_path_str)
                };

                let canonical_path = match expanded_path.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        if let Ok(curr) = std::env::current_dir().and_then(|c| c.canonicalize()) {
                            curr
                        } else {
                            return ApiResponse::error(
                                req.id,
                                format!("Directory '{}' does not exist or cannot be accessed: {}", raw_path_str, e),
                            );
                        }
                    }
                };

                if !canonical_path.is_dir() {
                    return ApiResponse::error(
                        req.id,
                        format!("Path '{}' is not a directory", raw_path_str),
                    );
                }

                // 1. Deep Probe Git (files vs dirs, worktrees, commondir)
                let git_bin = resolve_git_binary();
                let deep_git_probe = resolve_deep_git_paths(&canonical_path).await;

                let mut is_git = deep_git_probe.is_some();
                let mut root_path = canonical_path.to_string_lossy().to_string();
                let mut branch: Option<String> = None;
                let mut is_dirty = false;
                let mut dirty_count = 0;
                let mut remote_url: Option<String> = None;
                let mut is_worktree = deep_git_probe.as_ref().map(|(_, _, wt)| *wt).unwrap_or(false);

                // Check via git rev-parse for root work-tree toplevel
                let toplevel_res = tokio::process::Command::new(&git_bin)
                    .current_dir(&canonical_path)
                    .args(["rev-parse", "--is-inside-work-tree", "--show-toplevel"])
                    .output()
                    .await;

                if let Ok(out) = toplevel_res {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let lines: Vec<&str> = stdout.lines().collect();
                        if let Some(first) = lines.first() {
                            if *first == "true" {
                                is_git = true;
                                if let Some(tl) = lines.get(1) {
                                    root_path = tl.trim().to_string();
                                }
                            }
                        }
                    }
                }

                let mut worktrees: Vec<serde_json::Value> = Vec::new();
                let mut github_host: Option<String> = None;
                let mut github_owner: Option<String> = None;
                let mut github_repo: Option<String> = None;
                let mut github_avatar_url: Option<String> = None;

                if is_git {
                    // Get current branch
                    if let Ok(b_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                        .output()
                        .await
                    {
                        if b_out.status.success() {
                            let b_str = String::from_utf8_lossy(&b_out.stdout).trim().to_string();
                            if !b_str.is_empty() {
                                branch = Some(b_str);
                            }
                        }
                    }

                    // Check dirty status
                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["status", "--porcelain=v1"])
                        .output()
                        .await
                    {
                        if s_out.status.success() {
                            let changes = String::from_utf8_lossy(&s_out.stdout);
                            dirty_count = changes.lines().filter(|l| !l.trim().is_empty()).count();
                            is_dirty = dirty_count > 0;
                        }
                    }

                    // Get remote URL (origin first, then upstream)
                    for remote_name in ["origin", "upstream"] {
                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                            .current_dir(&root_path)
                            .args(["remote", "get-url", remote_name])
                            .output()
                            .await
                        {
                            if r_out.status.success() {
                                let r_str = String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                                if !r_str.is_empty() {
                                    remote_url = Some(r_str);
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(ref r_url) = remote_url {
                        let (gh_host, gh_owner, gh_repo, gh_avatar) = parse_github_remote(r_url);
                        github_host = gh_host;
                        github_owner = gh_owner;
                        github_repo = gh_repo;
                        github_avatar_url = gh_avatar;
                    }

                    // Check if linked worktree
                    let dir_out = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--git-dir", "--git-common-dir"])
                        .output()
                        .await;
                    if let Ok(d_out) = dir_out {
                        if d_out.status.success() {
                            let out_str = String::from_utf8_lossy(&d_out.stdout);
                            let d_lines: Vec<&str> = out_str.lines().collect();
                            if d_lines.len() >= 2 && d_lines[0] != d_lines[1] {
                                is_worktree = true;
                            }
                        }
                    }

                    // List all worktrees for this repository
                    worktrees = list_git_worktrees(&git_bin, &root_path).await;
                }

                // 1b. Scan immediate subdirectories for child Git repositories / workspaces
                let mut sub_repositories: Vec<serde_json::Value> = Vec::new();
                if let Ok(mut dir_entries) = tokio::fs::read_dir(&canonical_path).await {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                        if let Ok(file_type) = entry.file_type().await {
                            if file_type.is_dir() {
                                let sub_dir = entry.path();
                                if let Some((_sub_gitdir, _sub_commondir, _sub_is_wt)) = resolve_deep_git_paths(&sub_dir).await {
                                    let sub_name = entry.file_name().to_string_lossy().to_string();
                                    let mut sub_branch = String::from("main");
                                    if let Ok(sub_b_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                                        .output()
                                        .await
                                    {
                                        if sub_b_out.status.success() {
                                            let s = String::from_utf8_lossy(&sub_b_out.stdout).trim().to_string();
                                            if !s.is_empty() {
                                                sub_branch = s;
                                            }
                                        }
                                    }

                                    let mut sub_dirty_count = 0;
                                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["status", "--porcelain=v1"])
                                        .output()
                                        .await
                                    {
                                        if s_out.status.success() {
                                            let changes = String::from_utf8_lossy(&s_out.stdout);
                                            sub_dirty_count = changes.lines().filter(|l| !l.trim().is_empty()).count();
                                        }
                                    }

                                    let mut sub_remote_url: Option<String> = None;
                                    for remote_name in ["origin", "upstream"] {
                                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                                            .current_dir(&sub_dir)
                                            .args(["remote", "get-url", remote_name])
                                            .output()
                                            .await
                                        {
                                            if r_out.status.success() {
                                                let r = String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                                                if !r.is_empty() {
                                                    sub_remote_url = Some(r);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    let (sub_gh_host, sub_gh_owner, sub_gh_repo, sub_gh_avatar) = if let Some(ref r) = sub_remote_url {
                                        parse_github_remote(r)
                                    } else {
                                        (None, None, None, None)
                                    };

                                    let sub_worktrees = list_git_worktrees(&git_bin, &sub_dir.to_string_lossy()).await;

                                    sub_repositories.push(serde_json::json!({
                                        "name": sub_name,
                                        "path": sub_dir.to_string_lossy().to_string(),
                                        "branch": sub_branch,
                                        "is_git": true,
                                        "is_dirty": sub_dirty_count > 0,
                                        "dirty_count": sub_dirty_count,
                                        "remote_url": sub_remote_url,
                                        "github_host": sub_gh_host,
                                        "github_owner": sub_gh_owner,
                                        "github_repo": sub_gh_repo,
                                        "github_avatar_url": sub_gh_avatar,
                                        "worktrees": sub_worktrees
                                    }));
                                }
                            }
                        }
                    }
                }

                // 2. Detect Project Type
                let root_p = std::path::Path::new(&root_path);
                let project_type = if root_p.join("Cargo.toml").exists() {
                    "Rust"
                } else if root_p.join("package.json").exists() {
                    "TypeScript / Node"
                } else if root_p.join("pyproject.toml").exists() || root_p.join("requirements.txt").exists() {
                    "Python"
                } else if root_p.join("go.mod").exists() {
                    "Go"
                } else if root_p.join("pom.xml").exists() || root_p.join("build.gradle").exists() {
                    "Java"
                } else if is_git {
                    "Git Repository"
                } else {
                    "Folder"
                };

                let name = root_p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string());

                let result = serde_json::json!({
                    "path": canonical_path.to_string_lossy(),
                    "root_path": root_path,
                    "name": name,
                    "is_git": is_git,
                    "branch": branch,
                    "is_dirty": is_dirty,
                    "dirty_count": dirty_count,
                    "remote_url": remote_url,
                    "github_host": github_host,
                    "github_owner": github_owner,
                    "github_repo": github_repo,
                    "github_avatar_url": github_avatar_url,
                    "project_type": project_type,
                    "is_worktree": is_worktree,
                    "worktrees": worktrees,
                    "sub_repositories": sub_repositories
                });

                ApiResponse::success(req.id, result)
            }
            METHOD_WORKSPACES_PICK_FOLDER => {
                #[cfg(target_os = "macos")]
                {
                    let script = if req.params.get("purpose").and_then(|v| v.as_str()) == Some("model") {
                        "POSIX path of (choose folder with prompt \"Select Folder Containing GGUF Models\")"
                    } else {
                        "POSIX path of (choose folder with prompt \"Select Project or Repository Folder\")"
                    };
                    let out = tokio::process::Command::new("osascript")
                        .arg("-e")
                        .arg(script)
                        .output()
                        .await;

                    match out {
                        Ok(o) if o.status.success() => {
                            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                            ApiResponse::success(req.id, serde_json::json!({ "path": p }))
                        }
                        _ => ApiResponse::success(req.id, serde_json::json!({ "path": null })),
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    ApiResponse::success(req.id, serde_json::json!({ "path": null }))
                }
            }
            METHOD_WORKSPACES_CANDIDATES_SPAWN => {
                let _mutation = self.candidate_mutation.lock().await;
                let command_id = req.params.get("command_id").and_then(|value| value.as_str()).unwrap_or(&req.id).to_string();
                if command_id.is_empty() || command_id.len() > 128 || !command_id.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')) {
                    return ApiResponse::error(req.id, "Candidate command_id must be an identifier of at most 128 characters");
                }
                let request_hash = blake3::hash(req.params.to_string().as_bytes()).to_hex().to_string();
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id)
                        if !id.trim().is_empty()
                            && id.trim().len() <= 128
                            && id.trim().chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')) =>
                    {
                        id.trim().to_string()
                    }
                    _ => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
                };

                match self.task_service.get_task(&task_id).await {
                    Ok(Some(_)) => {}
                    Ok(None) => return ApiResponse::error(req.id, format!("Task '{task_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }

                let recorded = match coordinator.list_workspaces().await {
                    Ok(workspaces) => workspaces.into_iter().filter(|workspace| workspace.metadata.get("batch_command_id").and_then(|value| value.as_str()) == Some(&command_id)).collect::<Vec<_>>(),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                if !recorded.is_empty() {
                    if recorded.iter().any(|workspace| workspace.owner_task_id.as_deref() != Some(&task_id) || workspace.metadata.get("batch_request_hash").and_then(|value| value.as_str()) != Some(&request_hash)) {
                        return ApiResponse::error(req.id, "This command_id already belongs to a different candidate request");
                    }
                    let requested = req.params.get("count").and_then(|value| value.as_u64()).unwrap_or(2).clamp(1, 8) as usize;
                    let pending = recorded.iter().any(|workspace| !workspace.is_ready());
                    return ApiResponse::success(req.id, serde_json::json!({"batch_command_id": command_id, "task_id": task_id,
                        "candidates": recorded, "failed": [], "partial": pending || requested != recorded.len(), "needs_reconciliation": pending || requested != recorded.len(), "replayed": true}));
                }
                let count = req.params.get("count").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let count = count.clamp(1, 8);

                let base_repo_path = if let Some(base_ws_id) = req.params.get("base_workspace_id").and_then(|v| v.as_str()) {
                    match coordinator.get_workspace(&custos_domain::WorkspaceId::new(base_ws_id)).await {
                        Ok(Some(ws)) if ws.is_ready() => ws.path,
                        _ => return ApiResponse::error(req.id, format!("Base workspace '{base_ws_id}' not found")),
                    }
                } else if let Some(repo_path) = req.params.get("repo_path").and_then(|v| v.as_str()) {
                    repo_path.to_string()
                } else {
                    return ApiResponse::error(req.id, "Either 'base_workspace_id' or 'repo_path' must be provided");
                };

                let base_branch = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let base_branch = match base_branch {
                    Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Candidate creation requires a Git checkout"),
                };
                if base_branch.is_empty() {
                    return ApiResponse::error(req.id, "Candidate creation requires a named base branch");
                }
                let base_status = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["status", "--porcelain", "--untracked-files=all"])
                    .output()
                    .await;
                match base_status {
                    Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                    Ok(out) if out.status.success() => return ApiResponse::error(req.id, "Candidate creation requires a clean base checkout; uncommitted files would not be copied"),
                    _ => return ApiResponse::error(req.id, "Could not inspect candidate base checkout"),
                }

                let base_commit_hash = if let Some(h) = req.params.get("base_commit_hash").and_then(|v| v.as_str()) {
                    Some(h.to_string())
                } else {
                    let mut cmd = tokio::process::Command::new("git");
                    cmd.current_dir(&base_repo_path).arg("rev-parse").arg("HEAD");
                    cmd.output().await.ok().filter(|o| o.status.success()).map(|o| {
                        String::from_utf8_lossy(&o.stdout).trim().to_string()
                    })
                };

                let mut spawned_workspaces = Vec::new();
                let mut failed_workspaces = Vec::new();
                let batch_command_id = command_id;
                let base_path = std::path::Path::new(&base_repo_path);
                let (Some(parent), Some(repo_name)) = (base_path.parent(), base_path.file_name()) else {
                    return ApiResponse::error(req.id, "Candidate base must have a named parent directory");
                };
                for i in 1..=count {
                    let branch = format!("custos/candidate/{task_id}/{i}");
                    let candidate_path = parent
                        .join(".custos-worktrees")
                        .join(repo_name)
                        .join(&task_id)
                        .join(format!("candidate-{}", i))
                        .to_string_lossy()
                        .to_string();

                    let metadata = serde_json::json!({
                        "candidate_index": i,
                        "task_id": task_id,
                        "branch": branch,
                        "base_commit_hash": base_commit_hash,
                        "batch_command_id": &batch_command_id,
                        "batch_request_hash": &request_hash,
                    });

                    let create_req = CreateWorkspaceRequest {
                        name: format!("candidate-{}-{}", task_id, i),
                        kind: custos_domain::WorkspaceKind::Git {
                            repo_path: base_repo_path.clone(),
                            branch: branch.clone(),
                            base_commit: base_commit_hash.clone(),
                        },
                        path: candidate_path,
                        lineage: Some(custos_domain::WorkspaceLineage {
                            parent_workspace_id: req.params.get("base_workspace_id").and_then(|v| v.as_str()).map(custos_domain::WorkspaceId::new),
                            base_commit: base_commit_hash.clone(),
                            target_branch: Some(base_branch.clone()),
                            head_commit: None,
                        }),
                        owner_task_id: Some(task_id.clone()),
                        metadata: Some(metadata),
                        setup_script: req.params.get("setup_script").and_then(|v| v.as_str()).map(str::to_string),
                    };

                    match coordinator.create_workspace(create_req).await {
                        Ok(ws) => spawned_workspaces.push(ws),
                        Err(e) => {
                            failed_workspaces.push(serde_json::json!({
                                "candidate_index": i,
                                "branch": branch,
                                "error": e.to_string(),
                            }));
                        }
                    }
                }

                if spawned_workspaces.is_empty() && !failed_workspaces.is_empty() {
                    return ApiResponse::error(req.id, format!("Failed to create candidate worktrees: {:?}", failed_workspaces));
                }

                let candidates_val = serde_json::to_value(&spawned_workspaces).unwrap_or_default();
                ApiResponse::success(req.id, serde_json::json!({
                    "batch_command_id": batch_command_id,
                    "task_id": task_id,
                    "base_repo_path": base_repo_path,
                    "base_commit_hash": base_commit_hash,
                    "candidates": candidates_val,
                    "failed": failed_workspaces,
                    "partial": !failed_workspaces.is_empty(),
                }))
            }
            METHOD_WORKSPACES_CANDIDATES_COMPARE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let target_workspaces: Vec<custos_domain::ExecutionWorkspace> = if let Some(ws_ids) = req.params.get("workspace_ids").and_then(|v| v.as_array()) {
                    let mut list = Vec::new();
                    for val in ws_ids {
                        if let Some(id_str) = val.as_str() {
                            match coordinator.get_workspace(&custos_domain::WorkspaceId::new(id_str)).await {
                                Ok(Some(ws)) => list.push(ws),
                                Ok(None) => return ApiResponse::error(req.id, format!("Workspace '{id_str}' not found")),
                                Err(e) => return ApiResponse::error(req.id, format!("Failed to retrieve workspace '{id_str}': {e}")),
                            }
                        }
                    }
                    if list.len() != ws_ids.len() { return ApiResponse::error(req.id, "Invalid workspace ID in comparison"); }
                    list
                } else if let Some(task_id) = req.params.get("task_id").and_then(|v| v.as_str()) {
                    let all = match coordinator.list_workspaces().await { Ok(all) => all, Err(error) => return ApiResponse::error(req.id, error.to_string()) };
                    all.into_iter().filter(|w| w.owner_task_id.as_deref() == Some(task_id)).collect()
                } else {
                    return ApiResponse::error(req.id, "Either 'workspace_ids' or 'task_id' must be provided");
                };

                if target_workspaces.is_empty() {
                    return ApiResponse::error(req.id, "No candidate workspaces found for comparison");
                }

                let baseline = &target_workspaces[0];
                let Some(owner_task) = baseline.owner_task_id.as_deref() else {
                    return ApiResponse::error(req.id, "Comparison requires Task-owned candidate worktrees");
                };
                let Some(base_commit) = baseline.base_commit_hash.as_deref().filter(|hash| !hash.is_empty()) else {
                    return ApiResponse::error(req.id, "Comparison requires a recorded base commit");
                };
                let requested_task = req.params.get("task_id").and_then(|value| value.as_str());
                let mut selected_ids = std::collections::HashSet::new();
                for candidate in &target_workspaces {
                    if !selected_ids.insert(candidate.id.to_string()) || !candidate.is_ready()
                        || candidate.owner_task_id.as_deref() != Some(owner_task)
                        || requested_task.is_some_and(|task| task != owner_task)
                        || candidate.base_commit_hash.as_deref() != Some(base_commit) {
                        return ApiResponse::error(req.id, "Candidates must be distinct ready worktrees of one Task and base commit");
                    }
                }

                let first_repo = match &target_workspaces[0].kind {
                    custos_domain::WorkspaceKind::Git { repo_path, .. } => repo_path.clone(),
                    _ => return ApiResponse::error(req.id, format!("Workspace '{}' is not a Git workspace; candidates must be Git worktrees", target_workspaces[0].id)),
                };

                for ws in &target_workspaces {
                    match &ws.kind {
                        custos_domain::WorkspaceKind::Git { repo_path, .. } => {
                            if *repo_path != first_repo {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Candidate workspace '{}' belongs to repository '{}', but baseline repository is '{}'", ws.id, repo_path, first_repo)
                                );
                            }
                        }
                        _ => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace '{}' is not a Git workspace", ws.id)
                            );
                        }
                    }
                }

                let mut candidate_comparisons = Vec::new();
                for ws in target_workspaces {
                    let dirty_res = coordinator.inspect_dirty(&ws.id).await;
                    let diff_res = coordinator.diff(&ws.id).await;

                    let (dirty, dirty_err) = match dirty_res {
                        Ok(d) => (Some(d), None),
                        Err(e) => (None, Some(e.to_string())),
                    };
                    let (diff_text, diff_err) = match diff_res {
                        Ok(d) => (Some(d), None),
                        Err(e) => (None, Some(e.to_string())),
                    };

                    let branch = match &ws.kind {
                        custos_domain::WorkspaceKind::Git { branch, .. } => branch.clone(),
                        _ => "main".to_string(),
                    };

                    let inspection_ok = dirty_err.is_none() && diff_err.is_none();
                    let is_ready_for_integration = inspection_ok && ws.is_ready();

                    candidate_comparisons.push(serde_json::json!({
                        "workspace_id": ws.id.to_string(),
                        "name": ws.name,
                        "path": ws.path,
                        "branch": branch,
                        "base_commit_hash": ws.base_commit_hash,
                        "owner_task_id": ws.owner_task_id,
                        "is_dirty": dirty.as_ref().map(|d| d.is_dirty).unwrap_or(false),
                        "modified_count": dirty.as_ref().map(|d| d.modified_files.len()).unwrap_or(0),
                        "untracked_count": dirty.as_ref().map(|d| d.untracked_files.len()).unwrap_or(0),
                        "deleted_count": dirty.as_ref().map(|d| d.deleted_files.len()).unwrap_or(0),
                        "modified_files": dirty.as_ref().map(|d| d.modified_files.clone()).unwrap_or_default(),
                        "untracked_files": dirty.as_ref().map(|d| d.untracked_files.clone()).unwrap_or_default(),
                        "deleted_files": dirty.as_ref().map(|d| d.deleted_files.clone()).unwrap_or_default(),
                        "diff": diff_text.unwrap_or_default(),
                        "dirty_error": dirty_err,
                        "diff_error": diff_err,
                        "inspection_error": if !inspection_ok { Some("Failed to inspect candidate worktree status or diff") } else { None },
                        "is_ready_for_integration": is_ready_for_integration,
                        "status": ws.status.to_string(),
                    }));
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "count": candidate_comparisons.len(),
                    "candidates": candidate_comparisons,
                }))
            }
            METHOD_WORKSPACES_INTEGRATE => {
                let _mutation = self.candidate_mutation.lock().await;
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let winner_id = match req.params.get("winner_workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => custos_domain::WorkspaceId::new(id),
                    _ => return ApiResponse::error(req.id, "Missing 'winner_workspace_id' parameter"),
                };

                let mut winner_ws = match coordinator.get_workspace(&winner_id).await {
                    Ok(Some(ws)) => ws,
                    _ => return ApiResponse::error(req.id, format!("Winner workspace '{winner_id}' not found")),
                };

                let (repo_path, winner_branch) = match &winner_ws.kind {
                    custos_domain::WorkspaceKind::Git { repo_path, branch, .. } => (repo_path.clone(), branch.clone()),
                    _ => return ApiResponse::error(req.id, "Winner workspace must be a Git workspace"),
                };

                if winner_ws.owner_task_id.is_none() || !winner_ws.metadata.is_object() {
                    return ApiResponse::error(req.id, "Integration requires a Task-owned candidate worktree");
                }
                if let Some(intent) = winner_ws.metadata.get("integration").cloned() {
                    if let Some(result) = intent.get("result").filter(|result| result.get("merge_success").and_then(|value| value.as_bool()) == Some(true)) { return ApiResponse::success(req.id, result.clone()); }
                    let recorded_head = intent.get("winner_head").and_then(|value| value.as_str()).unwrap_or("");
                    let recorded_target = intent.get("target_branch").and_then(|value| value.as_str()).unwrap_or("");
                    let target_ref = format!("refs/heads/{recorded_target}");
                    let ancestry = tokio::process::Command::new("git").current_dir(&repo_path)
                        .args(["merge-base", "--is-ancestor", recorded_head, &target_ref]).output().await;
                    if matches!(ancestry, Ok(ref output) if output.status.success()) {
                        let result = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                            "target_branch": intent.get("target_branch"), "merge_success": true, "merge_status": "merged_reconciled",
                            "merge_detail": "Recorded winner commit is already included in the recorded target branch; no merge was replayed", "is_stale_base": false, "loser_cleanup": []});
                        winner_ws.metadata["integration"]["result"] = result.clone();
                        if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, error.to_string()); }
                        return ApiResponse::success(req.id, result);
                    }
                    let merge_head = tokio::process::Command::new("git").current_dir(&repo_path).args(["rev-parse", "--verify", "MERGE_HEAD"]).output().await;
                    let conflict = matches!(merge_head, Ok(ref output) if output.status.success());
                    return ApiResponse::success(req.id, serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                        "target_branch": intent.get("target_branch"), "merge_success": false, "merge_status": if conflict { "conflict_needs_resolution" } else { "uncertain" },
                        "merge_detail": "An earlier integration has no confirmed result. Inspect the target repository; the command was not replayed.", "is_stale_base": false, "loser_cleanup": []}));
                }
                let target_branch = req.params.get("target_branch").and_then(|v| v.as_str())
                    .or(winner_ws.lineage.target_branch.as_deref());
                let Some(target_branch) = target_branch.map(str::to_string) else {
                    return ApiResponse::error(req.id, "Candidate has no recorded target branch");
                };

                if !winner_ws.is_ready() {
                    return ApiResponse::error(req.id, "Winner workspace is not ready for integration");
                }

                // `git merge` modifies the branch currently checked out in repo_path,
                // not the branch named by target_branch. Refuse a mismatched checkout.
                let checked_out = tokio::process::Command::new("git")
                    .current_dir(&repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let checked_out = match checked_out {
                    Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Could not verify the checked-out target branch"),
                };
                if checked_out != target_branch {
                    return ApiResponse::error(req.id, format!(
                        "Target checkout is on '{checked_out}', not '{target_branch}'; integration was not started"
                    ));
                }

                for (label, path) in [("target", repo_path.as_str()), ("winner", winner_ws.path.as_str())] {
                    let status = tokio::process::Command::new("git")
                        .current_dir(path)
                        .args(["status", "--porcelain", "--untracked-files=all"])
                        .output()
                        .await;
                    match status {
                        Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                        Ok(out) if out.status.success() => return ApiResponse::error(req.id, format!(
                            "The {label} workspace has uncommitted or untracked changes; review them before integration"
                        )),
                        _ => return ApiResponse::error(req.id, format!("Could not inspect the {label} workspace")),
                    }
                }

                let winner_checkout = tokio::process::Command::new("git")
                    .current_dir(&winner_ws.path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                match winner_checkout {
                    Ok(out) if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == winner_branch => {}
                    _ => return ApiResponse::error(req.id, "Winner workspace no longer checks out its recorded branch"),
                }

                // Stale-Base Drift Check
                let mut rev_cmd = tokio::process::Command::new("git");
                rev_cmd.current_dir(&repo_path).arg("rev-parse").arg(&target_branch);
                let current_target_head = rev_cmd.output().await.ok().filter(|o| o.status.success()).map(|o| {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                });

                let (Some(ref current_head), Some(ref base_commit)) = (&current_target_head, &winner_ws.base_commit_hash) else {
                    return ApiResponse::error(req.id, "Cannot verify candidate base against target HEAD");
                };
                let is_stale_base = current_head != base_commit;
                if is_stale_base {
                    return ApiResponse::error(req.id, format!(
                        "Candidate base {base_commit} is stale relative to target {current_head}; integration was not started"
                    ));
                }

                let winner_head = match tokio::process::Command::new("git").current_dir(&winner_ws.path).args(["rev-parse", "HEAD"]).output().await {
                    Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Could not verify winner HEAD"),
                };
                let preflight = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                    "target_branch": target_branch, "current_target_head": current_target_head, "winner_head": winner_head,
                    "winner_base_commit": winner_ws.base_commit_hash, "is_stale_base": false, "preview": true,
                    "merge_success": false, "merge_detail": "Ready for explicit integration", "merge_status": "ready"});
                if req.params.get("preview").and_then(|value| value.as_bool()) == Some(true) { return ApiResponse::success(req.id, preflight); }
                if req.params.get("expected_target_head").and_then(|value| value.as_str()) != current_target_head.as_deref()
                    || req.params.get("expected_winner_head").and_then(|value| value.as_str()) != Some(&winner_head) {
                    return ApiResponse::error(req.id, "Review a fresh integration preview before applying; expected heads are missing or changed");
                }
                winner_ws.metadata["integration"] = serde_json::json!({"winner_head": winner_head, "target_head": current_target_head,
                    "target_branch": target_branch, "started_at": chrono::Utc::now().to_rfc3339()});
                if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, error.to_string()); }

                // Merge winner branch into target repository
                let mut merge_cmd = tokio::process::Command::new("git");
                merge_cmd.current_dir(&repo_path)
                    .arg("merge")
                    .arg("--no-ff")
                    .arg("-m")
                    .arg(format!("Merge winner candidate {} ({})", winner_ws.name, winner_branch))
                    .arg(&winner_branch);

                let merge_output = merge_cmd.output().await;
                let (merge_success, merge_detail, conflict_detected) = match merge_output {
                    Ok(out) => {
                        let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
                        let success = out.status.success();
                        let conflict = !success && (text.contains("CONFLICT") || text.contains("Automatic merge failed"));
                        (success, text.trim().to_string(), conflict)
                    }
                    Err(e) => (false, format!("Failed to spawn git merge: {e}"), false),
                };

                let merge_status = if merge_success {
                    "merged"
                } else if conflict_detected {
                    "conflict_needs_resolution"
                } else {
                    "failed"
                };

                // Integration never implies permission to delete other workspaces.
                // Candidate cleanup is a separate, explicit archive operation.
                let loser_cleanup_results: Vec<serde_json::Value> = Vec::new();

                let result = serde_json::json!({
                    "winner_workspace_id": winner_id.to_string(),
                    "winner_branch": winner_branch,
                    "target_branch": target_branch,
                    "is_stale_base": is_stale_base,
                    "current_target_head": current_target_head,
                    "winner_base_commit": winner_ws.base_commit_hash,
                    "merge_success": merge_success,
                    "merge_status": merge_status,
                    "conflict_detected": conflict_detected,
                    "merge_detail": merge_detail,
                    "loser_cleanup": loser_cleanup_results,
                });
                winner_ws.metadata["integration"]["result"] = result.clone();
                if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, format!("Integration effect requires reconciliation: {error}")); }
                ApiResponse::success(req.id, result)
            }
            METHOD_SETTINGS_GET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                match store.get_settings() {
                    Ok(settings) => ApiResponse::success(req.id, serde_json::to_value(&settings).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SETTINGS_SET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                if let Some(map) = req.params.as_object() {
                    let mut updates = std::collections::HashMap::with_capacity(map.len());
                    for (k, v) in map {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        updates.insert(k.clone(), val_str);
                    }
                    if let Err(e) = store.set_settings(&updates) {
                        return ApiResponse::error(req.id, e.to_string());
                    }
                    match store.get_settings() {
                        Ok(settings) => ApiResponse::success(req.id, serde_json::to_value(&settings).unwrap_or_default()),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    ApiResponse::error(req.id, "Invalid params: expected JSON object of key-value settings")
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
                let file_bytes = src.local_path.as_ref().and_then(|p| std::fs::read(p).ok());
                let src = match custos_core::research_ingress::source_draft_with_bytes(src, file_bytes.as_deref()) {
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
            "v1.research.executions.get" | "v1.research.executions.resume.assess" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let execution_id = match req.params.get("execution_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing execution_id parameter"),
                };
                match research.get_execution_record(execution_id) {
                    Ok(Some(record)) => match if req.method == "v1.research.executions.resume.assess" {
                        serde_json::to_value(record.assess_checkpoint_resume())
                    } else {
                        serde_json::to_value(&record)
                    } {
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
                if anno.id.trim().is_empty() || anno.note.trim().is_empty() || anno.note.len() > 16_384
                    || anno.actor.trim().is_empty() || anno.side_chat_session_id.is_some() || anno.submitted_turn_id.is_some() {
                    return ApiResponse::error(req.id, "Annotation requires an identity, actor and note; conversation links must come from a real handoff");
                }
                let versions = match research.list_artifact_lineage(&anno.artifact_path) {
                    Ok(versions) => versions,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                if !versions.iter().any(|version| version.version == anno.artifact_version) {
                    return ApiResponse::error(req.id, "Annotation must target a recorded artifact version");
                }
                let valid_target = match &anno.target {
                    custos_domain::AnnotationTarget::Text { start_offset, end_offset, exact_text } =>
                        start_offset < end_offset && !exact_text.is_empty() && exact_text.len() <= 65_536,
                    custos_domain::AnnotationTarget::ImagePin { x_ratio, y_ratio, pin_number } =>
                        x_ratio.is_finite() && y_ratio.is_finite() && (0.0..=1.0).contains(x_ratio)
                            && (0.0..=1.0).contains(y_ratio) && *pin_number > 0,
                    custos_domain::AnnotationTarget::DomElement { selector } => !selector.trim().is_empty() && selector.len() <= 4096,
                };
                if !valid_target || anno.status != custos_domain::AnnotationStatus::Pending {
                    return ApiResponse::error(req.id, "Annotations enter as pending notes with a valid target; they do not certify review completion");
                }
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
                let mut content_version = None;
                let mut file = None;
                if artifact_path.starts_with("notes/") {
                    let note_id = artifact_path.trim_start_matches("notes/").trim_end_matches(".md");
                    match research.get_note(note_id) {
                        Ok(Some(note)) => { content_version = Some(note.version); content = Some(note.content); },
                        Ok(None) => {},
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    }
                } else if let Some(workspace_id) = req.params.get("workspace_id").and_then(|value| value.as_str()) {
                    if versions.is_empty() {
                        return ApiResponse::error(req.id, "Artifact has no recorded lineage");
                    }
                    let workspace = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    let artifact = std::path::Path::new(artifact_path);
                    let relative = if artifact.is_absolute() {
                        match artifact.strip_prefix(&workspace.path) {
                            Ok(relative) => relative.to_string_lossy().to_string(),
                            Err(_) => return ApiResponse::error(req.id, "Artifact is outside the selected workspace"),
                        }
                    } else { artifact_path.to_string() };
                    let current = match self.files.read_file(&workspace, &relative, Some(10 * 1024 * 1024)).await {
                        Ok(current) => current,
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    };
                    let bytes = if current.is_binary {
                        match base64::engine::general_purpose::STANDARD.decode(&current.content) {
                            Ok(bytes) => bytes,
                            Err(error) => return ApiResponse::error(req.id, format!("Invalid binary artifact encoding: {error}")),
                        }
                    } else { current.content.as_bytes().to_vec() };
                    let hash = blake3::hash(&bytes).to_hex().to_string();
                    if !current.truncated && (latest_content_hash == hash || latest_content_hash == format!("blake3:{hash}")) {
                        content_version = Some(latest_version);
                        if !current.is_binary { content = Some(current.content.clone()); }
                    }
                    file = Some(current);
                }

                let resp = ArtifactDetailResponse {
                    artifact_path: artifact_path.to_string(),
                    latest_version,
                    latest_content_hash,
                    versions,
                    annotations,
                    content,
                    file,
                    content_version,
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
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let mut cells: Vec<NotebookCell> = match req.params.get("cells").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing or invalid cells parameter"),
                };
                if cells.iter().any(|cell| cell.session_id != session_id) {
                    return ApiResponse::error(req.id, "Notebook cell session_id must match the requested session_id");
                }
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                if let Some(kernel) = self.python_kernel.as_ref() {
                    match kernel.get_status(session_id, Some(research)).await {
                        Ok(state) if state.status == custos_domain::KernelStatus::Busy => return ApiResponse::error(req.id, "Wait for notebook execution or interrupt it before changing cells"),
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        _ => {},
                    }
                }
                let stored = match research.list_notebook_cells(session_id) {
                    Ok(cells) => cells,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                for cell in &mut cells {
                    if let Some(previous) = stored.iter().find(|saved| saved.id == cell.id) {
                        if previous.source == cell.source && previous.cell_type == cell.cell_type {
                            cell.execution_count = previous.execution_count;
                            cell.status = previous.status;
                            cell.stdout = previous.stdout.clone();
                            cell.stderr = previous.stderr.clone();
                            cell.output_image = previous.output_image.clone();
                            cell.wall_ms = previous.wall_ms;
                        } else {
                            cell.execution_count = None;
                            cell.status = CellExecutionStatus::Idle;
                            cell.stdout = None;
                            cell.stderr = None;
                            cell.output_image = None;
                            cell.wall_ms = None;
                        }
                        cell.epoch = previous.epoch;
                    } else {
                        cell.execution_count = None;
                        cell.status = CellExecutionStatus::Idle;
                        cell.stdout = None;
                        cell.stderr = None;
                        cell.output_image = None;
                        cell.wall_ms = None;
                        cell.epoch = 0;
                    }
                }
                match research.replace_notebook_cells(session_id, &cells) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "saved": true, "count": cells.len() })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let mut params: ExecuteCellParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid execute cell params: {e}")),
                };
                let workspace_id = match params.workspace_id.as_deref().filter(|id| !id.trim().is_empty()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Notebook execution requires a registered workspace_id"),
                };
                let workspace = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(workspace) => workspace,
                    Err(response) => return response,
                };
                let research = match self.research.as_ref() {
                    Some(repository) => repository,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let saved = match research.list_notebook_cells(&params.session_id) {
                    Ok(cells) => cells,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match saved.iter().find(|cell| cell.id == params.cell_id) {
                    Some(cell) if cell.source == params.code && cell.cell_type == custos_domain::NotebookCellType::Code => {},
                    _ => return ApiResponse::error(req.id, "Save the exact code cell in this session before executing it"),
                }
                // cwd is resolved from the registered workspace, never from daemon process state.
                if params.cwd.as_deref().is_some_and(|cwd| cwd != workspace.path) {
                    return ApiResponse::error(req.id, "Notebook cwd must match the registered workspace");
                }
                params.cwd = Some(workspace.path);

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
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match self.remove_provider_credential(id, true).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "id": id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.routing.get" | "v1.providers.routing.save" | "v1.providers.route" => {
                let repo = match self.providers.as_ref() { Some(repo)=>repo,None=>return ApiResponse::error(req.id,"ProviderRepository not configured") };
                let service = req.params.get("service_type").and_then(|v|v.as_str()).unwrap_or("");
                if service.is_empty() { return ApiResponse::error(req.id,"Missing provider service"); }
                let result = match req.method.as_str() {
                    "v1.providers.route" => {
                        let model = req.params.get("model").and_then(|v|v.as_str()).unwrap_or("");
                        crate::runtime::reserve_available_provider_account(repo,service,model,req.params.get("reasoning_effort").and_then(|v|v.as_str()),&self.model_availability).map(|account|serde_json::json!({"account_id":account}))
                    },
                    "v1.providers.routing.save" => {
                        let enabled = match req.params.get("round_robin").and_then(|v|v.as_bool()) { Some(v)=>v,None=>return ApiResponse::error(req.id,"Missing round_robin mode") };
                        let order: Vec<String> = match req.params.get("account_order").cloned().map(serde_json::from_value) { Some(Ok(v))=>v,_=>return ApiResponse::error(req.id,"Invalid account order") };
                        repo.save_routing_policy(service,enabled,&order).and_then(|()|repo.routing_policy(service))
                    },
                    _ => repo.routing_policy(service),
                };
                return match result { Ok(value)=>ApiResponse::success(req.id,value),Err(e)=>ApiResponse::error(req.id,e.to_string()) };
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
                let existing_provider = match providers_repo.get_provider(&id) { Ok(value)=>value,Err(e)=>return ApiResponse::error(req.id,e.to_string()) };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(||existing_provider.as_ref().map(|p|p.name.as_str()).unwrap_or(&id))
                    .to_string();
                let service_type = req
                    .params
                    .get("service_type")
                    .or_else(|| req.params.get("provider_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(||existing_provider.as_ref().map(|p|p.service_type.as_str()).unwrap_or("anthropic"))
                    .to_string();
                let raw_api_key = req.params.get("api_key").and_then(|v| v.as_str());
                let masked = if raw_api_key.is_some_and(|key|!key.trim().is_empty()) { "••••••••".to_string() } else { providers_repo.get_provider(&id).ok().flatten().map(|p|p.api_key_masked).unwrap_or_else(||"none".into()) };
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
                match self.credential_service() {
                    Ok(service) => match service.save_provider(&cfg, raw_api_key).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": cfg.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
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

                let token_record = if req.params.get("api_key").and_then(|v|v.as_str()).is_some_and(|key|!key.trim().is_empty()) { None } else if self.credentials.is_some() {
                    match self.read_provider_token(provider_id.as_deref().unwrap_or(provider_type)).await { Ok(token)=>token,Err(e)=>return ApiResponse::error(req.id,e.to_string()) }
                } else { None };
                let explicit_key = req
                    .params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty());
                let oauth = explicit_key.is_none()
                    && token_record.as_ref().is_some_and(|tok| {
                        tok.refresh_token.is_some()
                            || tok
                                .scope
                                .as_deref()
                                .is_some_and(|s| !s.trim().is_empty())
                    });
                let mut api_key_owned = explicit_key
                    .map(str::to_string)
                    .or_else(|| token_record.as_ref().map(|tok| tok.access_token.clone()));
                if oauth {
                    if let Some(tok) = token_record.as_ref().filter(|tok| tok.is_expired(120)) {
                        let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(&tok.service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                        let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(&settings.0, &settings.1);
                        match manager
                            .refresh_token(
                                &settings.2,
                                tok,
                            )
                            .await
                        {
                            Ok(refreshed) => {
                                if self.credentials.is_some() {
                                    if let Err(e) = self.refresh_provider_token(tok, &refreshed).await {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                }
                                api_key_owned = Some(refreshed.access_token);
                            }
                            Err(_) => {
                                return ApiResponse::error(
                                    req.id,
                                    "OAuth refresh failed; reconnect the account",
                                )
                            }
                        }
                    }
                }
                let probe = if oauth && provider_type == "copilot" {
                    match api_key_owned.as_deref() { Some(token) => custos_adapters::providers::probe::probe_copilot_models(token).await, None => Err(custos_domain::DomainError::Validation("Copilot OAuth credentials missing".into())) }
                } else if provider_type == "antigravity" || (oauth && provider_type == "gemini") {
                    match api_key_owned.as_deref() {
                        Some(token) => {
                            let mut combined = Vec::new();
                            let mut seen_ids = std::collections::HashSet::new();
                            if let Ok(models) = custos_adapters::providers::antigravity::probe_codeassist_models("antigravity", "https://cloudcode-pa.googleapis.com", token).await {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) { combined.push(m); }
                                }
                            }
                            if let Ok(models) = custos_adapters::providers::antigravity::probe_codeassist_models("gemini", "https://cloudcode-pa.googleapis.com", token).await {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) { combined.push(m); }
                                }
                            }
                            if combined.is_empty() {
                                custos_adapters::providers::antigravity::probe_codeassist_models("antigravity", "https://cloudcode-pa.googleapis.com", token).await
                            } else {
                                Ok(combined)
                            }
                        }
                        None => Err(custos_domain::DomainError::Validation("Google OAuth credentials missing".into()))
                    }
                } else if oauth && matches!(provider_type, "openai" | "codex") {
                    match api_key_owned.as_deref() {
                        Some(token) => custos_adapters::providers::probe::probe_oauth_models(token).await,
                        None => Err(custos_domain::DomainError::Validation(
                            "OAuth credentials missing".into(),
                        )),
                    }
                } else {
                    custos_adapters::providers::probe::probe_endpoint_models(
                        &base_url,
                        api_key_owned.as_deref(),
                        if oauth && provider_type == "anthropic" { "anthropic-oauth" } else { provider_type },
                    )
                    .await
                };
                match probe {
                    Ok(mut models) => {
                        models.retain(|model| custos_adapters::providers::probe::conversation_model_is_visible(&model.id, model.name.as_deref()));
                        let mut unavailable_models = std::collections::HashMap::new();
                        if !matches!(provider_type, "openai" | "codex" | "anthropic" | "gemini" | "antigravity" | "copilot" | "deepseek" | "local" | "ollama" | "groq" | "openrouter" | "together" | "cerebras" | "perplexity" | "mistral" | "lmstudio" | "custom") { models.clear(); }
                        if !matches!(provider_type, "openai" | "codex" | "copilot") {
                            for model in &mut models {
                                if provider_type == "antigravity" && custos_adapters::providers::antigravity::supports_tiered_reasoning(&model.id) { continue; }
                                model.efforts.clear(); model.default_effort = None;
                            }
                        }
                        if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id.as_deref()) {
                            let catalog_result = self.model_catalog.merge_probed_models(provider_type, models.clone());
                            let cache_key = custos_adapters::providers::probe::verified_catalog_key(pid, &base_url, oauth, token_record.as_ref().map(|t| t.created_at).unwrap_or(0), provider_type);
                            if let Err(e) = repo.save_catalog_models(&cache_key, &catalog_result.models) { return ApiResponse::error(req.id, e.to_string()); }
                            if let Err(e) = self.model_availability.refresh(&cache_key) { return ApiResponse::error(req.id, e.to_string()); }
                            unavailable_models = match self.model_availability.snapshot(&cache_key) { Ok(value) => value, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                            models.retain(|model| !unavailable_models.contains_key("*") && !unavailable_models.contains_key(&model.id));
                        }
                        match serde_json::to_value(&models) {
                            Ok(val) => ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "models": val,
                                    "count": models.len(),
                                    "unavailable_models": unavailable_models,
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
                    let record = repo.get_provider(pid).ok().flatten();
                    let service = record
                        .as_ref()
                        .map(|p| p.service_type.as_str())
                        .or(provider_type)
                        .unwrap_or("local");
                    let endpoint = record
                        .as_ref()
                        .and_then(|p| p.endpoint_url.as_deref())
                        .filter(|s| !s.trim().is_empty())
                        .or_else(|| default_model_probe_base_url(service))
                        .unwrap_or("");
                    let cache_key = if record.is_some() {
                        match crate::runtime::provider_catalog_scope(repo, pid) { Ok(scope) => scope, Err(error) => return ApiResponse::error(req.id, error.to_string()) }
                    } else {
                        custos_adapters::providers::probe::verified_catalog_key(pid, endpoint, false, 0, service)
                    };
                    let fetched_at = match repo.catalog_fetched_at(&cache_key) {
                        Ok(timestamp) => timestamp,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    return match repo.list_catalog_models(Some(&cache_key)) {
                        Ok(mut cached) => {
                            let unavailable = match self.model_availability.snapshot(&cache_key) { Ok(value) => value, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                            cached.retain(|model| custos_adapters::providers::probe::conversation_model_is_visible(&model.id, Some(&model.label)) && !unavailable.contains_key("*") && !unavailable.contains_key(&model.id));
                            for model in &mut cached { model.provider_type = service.to_string(); }
                            ApiResponse::success(req.id, serde_json::json!({ "origin": "cached", "models": cached, "fetched_at": fetched_at, "unavailable_models": unavailable }))
                        },
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let catalog = self.model_catalog.get_catalog(provider_type);
                // Scanned files are inventory, not composed inference routes.
                // Exposing them as `local/<file_name>` sends them to the HTTP/Ollama adapter.
                match serde_json::to_value(&catalog) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.list" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match serde_json::to_value(&folders) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.add" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_path = match req.params.get("folder_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing folder_path parameter"),
                };
                let path = std::path::Path::new(folder_path);
                if !path.exists() || !path.is_dir() {
                    return ApiResponse::error(req.id, format!("Path does not exist or is not a directory: {}", folder_path));
                }
                let label = req.params.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());
                let now = chrono::Utc::now().timestamp_millis();
                let folder = custos_domain::LocalModelFolder {
                    id: custos_domain::new_id("fld"),
                    folder_path: folder_path.to_string(),
                    label,
                    is_active: true,
                    auto_scan: true,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = repo.save_local_folder(&folder) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let discovered = custos_adapters::local_inference::folder_scanner::scan_folder_for_models(&folder, &hardware);
                for m in &discovered {
                    if let Err(error) = repo.upsert_cached_model(m) {
                        return ApiResponse::error(req.id, format!("Cannot save discovered local model: {error}"));
                    }
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "folder": folder,
                    "models_discovered": discovered.len(),
                    "models": discovered
                }))
            }
            "v1.local_models.folders.remove" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match repo.delete_local_folder(id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "deleted": true, "id": id })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.scan" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let mut total_discovered = Vec::new();
                for f in &folders {
                    if let Some(fid) = folder_id {
                        if f.id != fid { continue; }
                    }
                    if !f.is_active { continue; }
                    let found = custos_adapters::local_inference::folder_scanner::scan_folder_for_models(f, &hardware);
                    for m in &found {
                        if let Err(error) = repo.upsert_cached_model(m) {
                            return ApiResponse::error(req.id, format!("Cannot save discovered local model: {error}"));
                        }
                    }
                    total_discovered.extend(found);
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "scanned_folders": folders.len(),
                    "models_count": total_discovered.len(),
                    "models": total_discovered,
                    "hardware": hardware,
                    "scanned_at": chrono::Utc::now().timestamp_millis()
                }))
            }
            "v1.local_models.list" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                match repo.list_cached_models(folder_id) {
                    Ok(mut models) => {
                        models.retain(|model| std::path::Path::new(&model.file_path).is_file());
                        ApiResponse::success(req.id, serde_json::json!({ "models": models }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.hardware" => {
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                match serde_json::to_value(&hardware) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.profile.get" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => ApiResponse::success(req.id, serde_json::json!({ "profile": profile })),
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                        let profile = custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(), revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(), gpu_layers: None,
                            eval_batch_size: Some(512), cpu_threads: None,
                            flash_attention: if hardware.is_apple_silicon { Some(true) } else { None },
                            mmap: true, mlock: false, idle_timeout_seconds: None, updated_at: 0,
                        };
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.profile.save" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let descriptor = match repo.get_cached_model_by_path(model_path) {
                    Ok(Some(value)) => value,
                    Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let context_size = req.params.get("context_size").and_then(|v| v.as_u64()).unwrap_or(4096);
                if context_size < 512 || context_size > descriptor.context_length {
                    return ApiResponse::error(req.id, format!("context_size must be between 512 and {} for this artifact", descriptor.context_length));
                }
                let gpu_mode = req.params.get("gpu_mode").and_then(|v| v.as_str()).unwrap_or("auto");
                if !matches!(gpu_mode, "auto" | "max" | "off" | "manual") {
                    return ApiResponse::error(req.id, "gpu_mode must be auto, max, off, or manual");
                }
                let gpu_layers = req.params.get("gpu_layers").and_then(|v| v.as_u64()).map(|v| v.min(u32::MAX as u64) as u32);
                if gpu_mode == "manual" && gpu_layers.is_none() {
                    return ApiResponse::error(req.id, "manual GPU mode requires gpu_layers");
                }
                let revision = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(current)) => current.revision.saturating_add(1),
                    Ok(None) => 1,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let profile = custos_domain::LocalModelLoadProfile {
                    model_path: model_path.to_string(), revision, context_size,
                    gpu_mode: gpu_mode.to_string(), gpu_layers,
                    eval_batch_size: req.params.get("eval_batch_size").and_then(|v| v.as_u64()).map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    cpu_threads: req.params.get("cpu_threads").and_then(|v| v.as_u64()).map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    flash_attention: req.params.get("flash_attention").and_then(|v| v.as_bool()),
                    mmap: req.params.get("mmap").and_then(|v| v.as_bool()).unwrap_or(true),
                    mlock: req.params.get("mlock").and_then(|v| v.as_bool()).unwrap_or(false),
                    idle_timeout_seconds: req.params.get("idle_timeout_seconds").and_then(|v| v.as_u64()),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                match repo.save_local_model_load_profile(&profile) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "profile": profile })),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.preflight" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let path = std::path::Path::new(model_path);
                if !path.exists() || !path.is_file() {
                    return ApiResponse::error(req.id, format!("Model file does not exist: {}", model_path));
                }
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let descriptor = match custos_adapters::local_inference::gguf_inspector::inspect_gguf_file(path, "preflight", &hardware) {
                    Ok(d) => d,
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to parse GGUF file: {}", e)),
                };
                let requested_context = req.params.get("requested_context").and_then(|v| v.as_u64()).unwrap_or(4096).min(descriptor.context_length);
                let inspected_context = descriptor.context_length.min(8192).max(1);
                let inspected_kv_bytes = descriptor.total_memory_required.saturating_sub(descriptor.file_size_bytes);
                let estimated_kv_bytes = inspected_kv_bytes
                    .saturating_mul(requested_context)
                    .checked_div(inspected_context)
                    .unwrap_or(inspected_kv_bytes);
                let estimated_memory_bytes = descriptor.file_size_bytes.saturating_add(estimated_kv_bytes);
                let can_load = estimated_memory_bytes < hardware.total_ram_bytes;
                let mut issues = Vec::new();
                if !can_load {
                    issues.push(format!(
                        "Estimated model and KV-cache memory ({} bytes) is at least the detected physical RAM ({} bytes); loading is blocked by the current preflight policy",
                        estimated_memory_bytes, hardware.total_ram_bytes
                    ));
                }
                if descriptor.compatibility == custos_domain::ModelCompatibility::Red {
                    issues.push("Model memory requirements with large context may exceed memory. Offloading or reduced context recommended.".to_string());
                } else if descriptor.compatibility == custos_domain::ModelCompatibility::Yellow {
                    issues.push("Model may require partial CPU offloading and run with reduced speed".to_string());
                }
                let response = custos_domain::LocalModelPreflightResponse {
                    model_path: model_path.to_string(),
                    can_load,
                    architecture: descriptor.architecture,
                    quantization: descriptor.quantization,
                    file_size_bytes: descriptor.file_size_bytes,
                    context_limit: requested_context,
                    estimated_memory_bytes,
                    free_ram_bytes: hardware.free_ram_bytes,
                    metal_vram_limit_bytes: hardware.metal_vram_limit_bytes,
                    compatibility: descriptor.compatibility,
                    recommended_gpu_layers: None,
                    issues,
                };
                match serde_json::to_value(&response) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.load" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(), revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(), gpu_layers: None,
                            eval_batch_size: Some(512), cpu_threads: None, flash_attention: None,
                            mmap: true, mlock: false, idle_timeout_seconds: None, updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match custos_adapters::local_inference::preload_model_with_profile(model_path, &profile).await {
                    Ok(instance) => {
                        match serde_json::to_value(&instance) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to load local model: {e}")),
                }
            }
            "v1.local_models.ensure" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let force_reload = req.params.get("force_reload").and_then(|v| v.as_bool()).unwrap_or(false);
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };

                let instances = custos_adapters::local_inference::list_loaded_instances().await.unwrap_or_default();
                let clean_model = model_path.strip_prefix("native:").unwrap_or(model_path);
                let existing = instances.into_iter().find(|inst| {
                    inst.model_path == clean_model || inst.model_name == clean_model
                });

                if let Some(inst) = existing {
                    if !force_reload && inst.profile_revision == profile.revision {
                        return ApiResponse::success(req.id, serde_json::json!({
                            "instance": inst,
                            "drift_detected": false,
                            "reloaded": false
                        }));
                    }
                }

                match custos_adapters::local_inference::preload_model_with_profile(model_path, &profile).await {
                    Ok(instance) => {
                        ApiResponse::success(req.id, serde_json::json!({
                            "instance": instance,
                            "drift_detected": false,
                            "reloaded": true
                        }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to ensure local model: {e}")),
                }
            }
            "v1.local_models.verify" => {
                let model_opt = req.params.get("model_path")
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str());

                let target_model = match model_opt {
                    Some(m) if !m.trim().is_empty() => m.trim().to_string(),
                    _ => {
                        let instances = custos_adapters::local_inference::list_loaded_instances().await.unwrap_or_default();
                        match instances.first() {
                            Some(inst) => inst.model_name.clone(),
                            None => return ApiResponse::error(req.id, "No active local model instance is loaded to verify"),
                        }
                    }
                };

                let clean_model = target_model.strip_prefix("native:").unwrap_or(&target_model);
                let start = std::time::Instant::now();
                match custos_adapters::local_inference::verify_sidecar_generation(clean_model).await {
                    Ok(output) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        ApiResponse::success(req.id, serde_json::json!({
                            "verified": true,
                            "model": clean_model,
                            "test_output": output,
                            "latency_ms": latency_ms
                        }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Generation probe failed: {e}")),
                }
            }
            "v1.local_models.unload" => {
                let model_identifier = match req.params.get("model_path")
                    .or_else(|| req.params.get("instance_id"))
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path or instance_id parameter"),
                };
                let clean_model = model_identifier.strip_prefix("native:").unwrap_or(model_identifier);
                match custos_adapters::local_inference::evict_model(clean_model).await {
                    Ok(evicted) => {
                        ApiResponse::success(req.id, serde_json::json!({ "evicted": evicted, "model": clean_model }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to unload model: {}", e)),
                }
            }
            "v1.local_models.instances" => {
                match custos_adapters::local_inference::list_loaded_instances().await {
                    Ok(instances) => ApiResponse::success(req.id, serde_json::json!({ "instances": instances })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.usage" => {
                let Some(store) = &self.usage_runs else { return ApiResponse::error(req.id, "Usage store unavailable"); };
                let tasks = match self.task_service.list_tasks().await { Ok(tasks) => tasks, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let mut records = Vec::new();
                for task in tasks {
                    let runs = match store.list_runs_for_task(&task.id).await { Ok(runs) => runs, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                    for run in runs {
                        if run.metadata.get("actual_mode").and_then(|v| v.as_str()) != Some("model") { continue; }
                        if run.metadata.get("model_id").and_then(|v| v.as_str()).is_some_and(|id| id.starts_with("custos-")) { continue; }
                        let usage = run.metadata.get("usage").cloned().unwrap_or(serde_json::Value::Null);
                        let failure = run.metadata.get("failure_reason").and_then(|v| v.as_str()).unwrap_or("");
                        records.push(serde_json::json!({"id":run.id,"account_id":run.metadata.get("provider_account_id"),"model":run.metadata.get("model_id").or_else(||run.metadata.get("requested_model")),"status":run.status.to_string(),"recorded_at":run.started_at,"input_tokens":usage.get("input_tokens"),"output_tokens":usage.get("output_tokens"),"total_tokens":usage.get("total_tokens"),"rate_limited":failure.contains("429") || failure.contains("RESOURCE_EXHAUSTED")}));
                    }
                }
                let mut quotas = Vec::new();
                if req.params.get("include_quota").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(repo) = &self.providers {
                        let providers = match repo.list_providers() { Ok(p) => p, Err(e) => return ApiResponse::error(req.id,e.to_string()) };
                        for provider in providers {
                            let mut snapshot = serde_json::json!({"account_id":provider.id,"status":if provider.service_type == "local" {"not_applicable"} else {"unsupported"},"windows":[],"fetched_at":chrono::Utc::now()});
                            if provider.service_type != "local" {
                                match self.read_provider_token(&provider.id).await {
                                    Ok(Some(mut token)) if token.refresh_token.is_some() || provider.service_type == "copilot" => {
                                        if token.is_expired(120) && token.refresh_token.is_some() {
                                            let refreshed = match custos_adapters::providers::oauth_pkce::oauth_settings(&token.service_type) {
                                                Ok(settings) => custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(&settings.0,&settings.1).refresh_token(&settings.2,&token).await,
                                                Err(e) => Err(e),
                                            };
                                            match refreshed { Ok(value) => { if let Err(e)=self.refresh_provider_token(&token, &value).await { return ApiResponse::error(req.id,e.to_string()); } token=value; }, Err(_) => { snapshot["status"]=serde_json::json!("unavailable"); snapshot["error"]=serde_json::json!("Reconnect account to refresh quota"); quotas.push(snapshot); continue; } }
                                        }
                                        match custos_adapters::providers::probe::probe_account_quota(&provider.service_type,&token.access_token).await {
                                            Ok(value) => {
                                                if value["status"] == "available" && value["windows"].as_array().is_some_and(|windows| !windows.is_empty() && windows.iter().all(|window| window["used_percent"].as_f64().is_some_and(|used| (0.0..100.0).contains(&used)))) {
                                                    if let Ok(scope) = crate::runtime::provider_catalog_scope(repo, &provider.id) { if let Err(error) = self.model_availability.quota_recovered(&scope) { tracing::debug!(%error, "Quota recovery projection failed"); } }
                                                }
                                                snapshot["status"]=value["status"].clone(); snapshot["windows"]=value["windows"].clone(); snapshot["source"]=value["source"].clone(); },
                                            Err(e) => { snapshot["status"]=serde_json::json!("unavailable"); snapshot["error"]=serde_json::json!(e.to_string()); }
                                        }
                                    },
                                    Err(_) => { snapshot["status"]=serde_json::json!("unavailable"); },
                                    _ => {}
                                }
                            }
                            quotas.push(snapshot);
                        }
                    }
                }
                tracing::debug!(request_id = %req.id, records = records.len(), quota_accounts = quotas.len(), "Provider usage snapshot loaded");
                ApiResponse::success(req.id, serde_json::json!({"records":records,"quotas":quotas,"fetched_at":chrono::Utc::now()}))
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

                let oauth_tokens = if self.credentials.is_some() {
                    match self.provider_tokens().await { Ok(tokens) => tokens, Err(e) => return ApiResponse::error(req.id, e.to_string()) }
                } else {
                    Vec::new()
                };

                let active_oauth = oauth_tokens
                    .into_iter()
                    .find(|t| !t.access_token.trim().is_empty());

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

                if let Some(tok) = active_oauth {
                    let display_name = match tok.provider_id.as_str() {
                        "openai" => "OpenAI (OAuth PKCE)".to_string(),
                        "github-copilot" | "copilot" => "GitHub Copilot (OAuth)".to_string(),
                        "google" | "gemini" => "Google Gemini (OAuth)".to_string(),
                        "antigravity" => "Antigravity Sovereign (OAuth)".to_string(),
                        other => format!("{other} (OAuth)"),
                    };
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": display_name,
                            "message": format!("{display_name} authentication active"),
                            "expires_at": tok.expires_at,
                            "provider_id": tok.provider_id,
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
                            "configured": true,
                            "active_provider": "Custos Sovereign (Local / Offline)",
                            "message": "Custos Offline Sovereign runtime sẵn sàng",
                        }),
                    )
                }
            }
            METHOD_OAUTH_AUTHORIZE => {
                let params: OAuthAuthorizeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let raw_service = params.service_type.as_deref().or(params.provider_id.as_deref()).unwrap_or("openai");
                let service = raw_service.split('-').next().unwrap_or(raw_service);
                if service == "copilot" {
                    let server = match self.oauth_callback_server.as_ref() { Some(v) => v, None => return ApiResponse::error(req.id, "OAuth callback server not configured") };
                    return match custos_adapters::providers::oauth_pkce::start_copilot_device().await {
                        Ok(mut device) => {
                            if device.get("device_code").and_then(|v| v.as_str()).is_none() || device.get("user_code").and_then(|v| v.as_str()).is_none() { return ApiResponse::error(req.id, "Invalid GitHub device authorization response"); }
                            server.start_copilot_device(device.clone(), params.provider_id.clone().unwrap_or_else(|| "copilot".into()));
                            if let Some(object) = device.as_object_mut() { object.remove("device_code"); }
                            device["flow_type"] = serde_json::json!("device_code");
                            device["authorization_url"] = device.get("verification_uri").cloned().unwrap_or(serde_json::json!("https://github.com/login/device"));
                            ApiResponse::success(req.id, device)
                        }
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(service) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let auth_url = params
                    .auth_endpoint
                    .as_deref()
                    .unwrap_or(&settings.0);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(&settings.2);
                let service_type_or_default = service;
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(if service_type_or_default == "anthropic" { "https://platform.claude.com/oauth/code/callback" } else if service_type_or_default == "antigravity" { "http://localhost:51121/oauth-callback" } else { custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI });
                let scope = params.scope.as_deref().or(Some(settings.3.as_str()));

                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(auth_url, token_url);
                let challenge = custos_adapters::providers::oauth_pkce::OAuthPkceManager::generate_pkce_challenge();

                match manager.build_authorization_url(client_id, redirect_uri, scope, &challenge) {
                    Ok(res) => {
                        // Start the loopback callback server on port 1455 if available
                        if let Some(ref cb_server) = self.oauth_callback_server {
                            let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                            let service_type = service;
                            let pending = crate::oauth_callback_server::PendingOAuthContext {
                                provider_id: provider_id.to_string(),
                                service_type: service_type.to_string(),
                                client_id: client_id.to_string(),
                                code_verifier: challenge.code_verifier.clone(),
                                state: challenge.state.clone(),
                                redirect_uri: redirect_uri.to_string(),
                                expires_at: chrono::Utc::now().timestamp_millis() + 600_000,
                            };
                            if let Err(e) = cb_server.start_listening(pending).await {
                                tracing::warn!(
                                    provider_id = %provider_id,
                                    "Notice: OAuth loopback callback listener failed to bind: {e}"
                                );
                            }
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
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let params: OAuthExchangeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                let raw_service = params.service_type.as_deref().or(params.provider_id.as_deref()).unwrap_or("openai");
                let service_type = raw_service.split('-').next().unwrap_or(raw_service);
                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(&settings.2);
                let service_type_or_default = service_type;
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(if service_type_or_default == "anthropic" { "https://platform.claude.com/oauth/code/callback" } else if service_type_or_default == "antigravity" { "http://localhost:51121/oauth-callback" } else { custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI });
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);

                let callback_server = match self.oauth_callback_server.as_ref() { Some(v) => v, None => return ApiResponse::error(req.id, "OAuth callback server not configured") };
                if let Err(e) = callback_server.validate_manual_callback(provider_id, &params.code_verifier, redirect_uri, &params.code_or_url) { return ApiResponse::error(req.id, e.to_string()); }
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager
                    .exchange_code_with_identity(
                        provider_id,
                        service_type,
                        client_id,
                        &params.code_or_url,
                        redirect_uri,
                        &params.code_verifier,
                    )
                    .await
                {
                    Ok((token_record, identity)) => {
                        // Upsert matching provider config so it appears in UI as active
                        let now = chrono::Utc::now().timestamp_millis();
                        let masked = "oauth".to_string();
                        let prov_cfg = custos_domain::ProviderConfig {
                            id: provider_id.to_string(),
                            name: identity.unwrap_or_else(|| format!("{provider_id} (OAuth)")),
                            service_type: service_type.to_string(),
                            api_key_masked: masked,
                            status: "active".to_string(),
                            endpoint_url: if service_type == "gemini" { Some("https://cloudcode-pa.googleapis.com".into()) } else { default_model_probe_base_url(service_type).map(str::to_string) },
                            default_model: None,
                            context_window: None,
                            fast_mode: Some(false),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = self.write_provider_connection(&token_record, &prov_cfg).await { return ApiResponse::error(req.id, e.to_string()); }

                        match serde_json::to_value(serde_json::json!({"provider_id":token_record.provider_id,"service_type":token_record.service_type,"connected":true,"expires_at":token_record.expires_at,"scope":token_record.scope,"has_refresh_token":token_record.refresh_token.is_some()})) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_REFRESH => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let params: OAuthRefreshParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let existing = match self.read_provider_token(&params.provider_id).await {
                    Ok(Some(tok)) => tok,
                    Ok(None) => return ApiResponse::error(req.id, format!("No OAuth token found for {}", params.provider_id)),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(&existing.service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager.refresh_token(client_id, &existing).await {
                    Ok(refreshed) => {
                        if let Err(e) = self.refresh_provider_token(&existing, &refreshed).await {
                            return ApiResponse::error(req.id, format!("Failed to update token: {e}"));
                        }
                        match serde_json::to_value(serde_json::json!({"provider_id":refreshed.provider_id,"service_type":refreshed.service_type,"connected":true,"expires_at":refreshed.expires_at,"scope":refreshed.scope,"has_refresh_token":refreshed.refresh_token.is_some()})) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_GET => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.read_provider_token(provider_id).await {
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
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.remove_provider_credential(provider_id, false).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "provider_id": provider_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_CANCEL => {
                if let Some(server) = self.oauth_callback_server.as_ref() { server.cancel(); }
                ApiResponse::success(req.id, serde_json::json!({"cancelled":true}))
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
                        crate::oauth_callback_server::CallbackServerStatus::Exchanging { .. } => {
                            serde_json::json!({ "status": "exchanging" })
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
            METHOD_GITHUB_ACCOUNTS => {
                let gh = match resolve_gh_binary() { Some(gh) => gh, None => return ApiResponse::success(req.id, serde_json::json!({"installed":false,"accounts":[]})) };
                match custos_adapters::repository_git::github_accounts(&gh).await {
                    Ok(value) => ApiResponse::success(req.id,value),
                    Err(error) => ApiResponse::error(req.id,error.to_string()),
                }
            }
            METHOD_GIT_REPOSITORY | METHOD_GIT_SWITCH | METHOD_GIT_FETCH | METHOD_GIT_ACCOUNT | METHOD_GIT_GITHUB => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.is_empty() => id,
                    _ => return ApiResponse::error(req.id,"An exact workspace_id is required"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id,workspace_id).await { Ok(ws) => ws, Err(response) => return response };
                if !matches!(ws.kind,custos_domain::WorkspaceKind::Git { .. }) { return ApiResponse::error(req.id,"Local registered Git checkout required; remote-host Git routing is not available here"); }
                let _guard = self.candidate_mutation.lock().await;
                let path = std::path::Path::new(&ws.path);
                let gh = resolve_gh_binary();
                let result: Result<serde_json::Value,DomainError> = async {
                    use custos_adapters::repository_git as git;
                    tracing::debug!(request_id = %req.id, workspace_id, method = %req.method, "Repository Git operation");
                    match req.method.as_str() {
                        METHOD_GIT_REPOSITORY => git::repository(path).await,
                        METHOD_GIT_ACCOUNT => {
                            let binding = req.params.get("account").filter(|v| !v.is_null()).map(|v| serde_json::from_value::<git::GitHubBinding>(v.clone()).map_err(|_| DomainError::Validation("Invalid account preference".into()))).transpose()?;
                            git::bind_account(path,gh.as_deref(),binding.as_ref()).await?;
                            git::repository(path).await
                        }
                        METHOD_GIT_GITHUB => {
                            let binding = git::account_binding(path).await?.ok_or_else(|| DomainError::Validation("Choose a repository GitHub account first".into()))?;
                            git::github_viewer(gh.as_deref().ok_or_else(|| DomainError::Validation("GitHub CLI is not installed".into()))?,&binding).await
                        }
                        METHOD_GIT_FETCH => {
                            let remote = req.params.get("remote").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit remote required".into()))?;
                            git::fetch_remote(path,gh.as_deref(),remote).await
                        }
                        METHOD_GIT_SWITCH => {
                            let branch = req.params.get("branch").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit branch required".into()))?;
                            let expected = req.params.get("expected_head").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Observed HEAD required".into()))?;
                            if self.terminal.list_sessions(Some(workspace_id)).await?.iter().any(|s| matches!(s.status,custos_domain::TerminalSessionStatus::Active)) {
                                return Err(DomainError::Conflict("Close active checkout terminals before switching, or use a new worktree".into()));
                            }
                            let runs = self.usage_runs.as_ref().ok_or_else(|| DomainError::Validation("Cannot verify active runs; branch switch unavailable".into()))?;
                            for task in self.task_service.list_tasks().await? {
                                for run in runs.list_runs_for_task(&task.id).await? {
                                    let target = run.metadata.get("execution_workspace_id").or_else(||run.metadata.get("workspace_id")).and_then(|v|v.as_str());
                                    if !run.status.is_terminal() && (target.is_none() || target==Some(workspace_id)) {
                                        return Err(DomainError::Conflict("Active or unscoped run prevents branch switching; use a separate worktree".into()));
                                    }
                                }
                            }
                            git::switch_branch(path,branch,expected).await
                        }
                        _ => Err(DomainError::Validation("Unsupported Git operation".into())),
                    }
                }.await;
                match result { Ok(value) => ApiResponse::success(req.id,value), Err(error) => ApiResponse::error(req.id,error.to_string()) }
            }
            METHOD_GITHUB_LOGIN => {
                let _login_guard = GITHUB_LOGIN_LOCK.lock().await;
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Install GitHub CLI (gh) to connect. Local Git/worktrees remain available."),
                };
                // Reuse the bounded PTY lifecycle, never the model OAuth singleton.
                let existing = self.terminal.list_sessions(Some("github-auth")).await;
                if let Ok(sessions) = existing {
                    if let Some(session) = sessions.into_iter().find(|session| matches!(session.status, custos_domain::TerminalSessionStatus::Active)) {
                        return match serde_json::to_value(session) {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        };
                    }
                }
                let command = github_login_command(&gh);
                tracing::info!(request_id = %req.id, "Starting user-requested GitHub CLI login");
                match self.terminal.spawn("github-auth", &std::env::temp_dir(), Some(&command), 100, 28).await {
                    Ok(session) => match serde_json::to_value(session) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_AUTH_STATUS => {
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::success(req.id, serde_json::json!({
                        "authenticated": false, "source": "gh_cli", "installed": false,
                        "error": "GitHub CLI (gh) is not installed. Local Git/worktrees remain available."
                    })),
                };
                // The authenticated viewer is authoritative. Never export gh's token or
                // trust a cached profile / a fragment of multi-account auth-status text.
                let mut command = tokio::process::Command::new(gh);
                command.args(["api", "--hostname", "github.com", "user"]);
                command.kill_on_drop(true);
                let result = tokio::time::timeout(std::time::Duration::from_secs(10), command.output()).await;
                let value = match result {
                    Ok(Ok(output)) => github_auth_metadata(output.status.success(), &output.stdout),
                    Ok(Err(_)) => serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "Could not launch GitHub CLI."}),
                    Err(_) => serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "GitHub verification timed out. Check the network and retry."}),
                };
                ApiResponse::success(req.id, value)
            }
            METHOD_GITHUB_VIEWER => {
                let gh_bin = match resolve_gh_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "GitHub CLI (`gh`) is not installed or not in PATH",
                        );
                    }
                };

                match tokio::process::Command::new(&gh_bin)
                    .args(["api", "user"])
                    .output()
                    .await
                {
                    Ok(out) if out.status.success() => {
                        match serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => {
                                ApiResponse::error(req.id, format!("Invalid JSON from gh api user: {e}"))
                            }
                        }
                    }
                    Ok(out) => {
                        let err_msg = String::from_utf8_lossy(&out.stderr);
                        ApiResponse::error(req.id, format!("gh api user failed: {}", err_msg.trim()))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to run gh api user: {e}")),
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
            METHOD_WORKSPACE_GIT_COMMIT => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) if !m.trim().is_empty() => m,
                    _ => return ApiResponse::error(req.id, "Missing message parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.commit(&ws, message).await {
                    Ok(out) => ApiResponse::success(req.id, serde_json::json!({ "committed": true, "output": out })),
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

                // Require authoritative workspace binding to prevent arbitrary filesystem escapes
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        );
                    }
                };

                let cwd_path = if let Some(ws_id) = &params.workspace_id {
                    match self.resolve_workspace_for_dispatch(&req.id, ws_id).await {
                        Ok(ws) => {
                            let base = std::path::PathBuf::from(ws.path);
                            if let Some(c) = &params.cwd {
                                let requested = std::path::PathBuf::from(c);
                                if requested.is_absolute() && !requested.starts_with(&base) {
                                    return ApiResponse::error(
                                        req.id,
                                        format!("Security boundary violation: cwd '{c}' escapes registered workspace '{}'", base.display()),
                                    );
                                }
                                requested
                            } else {
                                base
                            }
                        }
                        Err(resp) => return resp,
                    }
                } else if let Some(c) = &params.cwd {
                    let requested = std::path::PathBuf::from(c);
                    // Check if requested cwd is within ANY registered workspace
                    let registered_workspaces = coordinator.list_workspaces().await.unwrap_or_default();
                    let canonical = requested.canonicalize().unwrap_or_else(|_| requested.clone());
                    let is_within_registered = registered_workspaces.iter().any(|ws| {
                        let ws_path = std::path::PathBuf::from(&ws.path);
                        let ws_canonical = ws_path.canonicalize().unwrap_or(ws_path);
                        canonical.starts_with(&ws_canonical)
                    });
                    if !is_within_registered {
                        return ApiResponse::error(
                            req.id,
                            format!("Security boundary violation: raw harness execution requires cwd inside a registered workspace, '{c}' is outside registered boundaries"),
                        );
                    }
                    requested
                } else {
                    return ApiResponse::error(
                        req.id,
                        "Missing workspace_id: raw harness execution must be bound to a registered workspace",
                    );
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
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; navigation is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let raw_url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(raw_url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                let mut tab = match fleet.get_browser_tab(tab_id) {
                    Ok(Some(t)) => t,
                    Ok(None) => return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let now = chrono::Utc::now().timestamp_millis();
                tab.url = raw_url.to_string();
                tab.title = if raw_url.is_empty() || raw_url == "about:blank" {
                    "New Tab".to_string()
                } else {
                    raw_url.to_string()
                };

                // Launch real browser navigation via agent-browser if available
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    if !raw_url.is_empty() && raw_url != "about:blank" {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(1500),
                            tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("open")
                                .arg(raw_url)
                                .output(),
                        )
                        .await;
                    }
                }

                tab.status = BrowserTabStatus::Ready;
                tab.updated_at = now;
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_SNAPSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; snapshot capture is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = if let Some(t) = req.params.get("tab_id").and_then(|v| v.as_str()) {
                    t.to_string()
                } else {
                    return ApiResponse::error(req.id, "Missing tab_id parameter");
                };

                let mut tab = match fleet.get_browser_tab(&tab_id) {
                    Ok(Some(t)) => t,
                    Ok(None) => return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();

                if let Ok(explicit_snapshot) = serde_json::from_value::<BrowserPageSnapshot>(req.params.clone()) {
                    tab.last_snapshot = Some(explicit_snapshot);
                    tab.updated_at = now;
                    let _ = fleet.save_browser_tab(&tab);
                    return ApiResponse::success(req.id, serde_json::json!({ "saved": true, "tab_id": tab_id }));
                }

                // If agent-browser is available, execute real AX tree snapshot and real screenshot
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let snap_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("snapshot")
                        .arg("-i")
                        .arg("--urls")
                        .output()
                        .await;

                    let screenshot_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("screenshot")
                        .output()
                        .await;

                    let mut screenshot_path = None;
                    if let Ok(ref s_out) = screenshot_out {
                        if s_out.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&s_out.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    screenshot_path = Some(p.to_string());
                                }
                            }
                        }
                    }

                    if let Ok(s_out) = snap_out {
                        if s_out.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&s_out.stdout) {
                                if let Some(data) = val.get("data") {
                                    let origin = data.get("origin").and_then(|v| v.as_str()).unwrap_or(&tab.url).to_string();
                                    let snap_text = data.get("snapshot").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let refs_val = data.get("refs");

                                    let mut links = Vec::new();
                                    if let Some(refs_obj) = refs_val.and_then(|v| v.as_object()) {
                                        for (key, val) in refs_obj {
                                            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                            let role = val.get("role").and_then(|v| v.as_str()).unwrap_or("");
                                            links.push(format!("@{} [{}]: {}", key, role, name));
                                        }
                                    }

                                    let summary = format!(
                                        "URL: {}\n\nAccessibility Tree (Interactive Elements):\n{}\n\nInteractive Refs Count: {}",
                                        origin,
                                        if snap_text.is_empty() { "No interactive elements found" } else { &snap_text },
                                        links.len()
                                    );

                                    let snapshot = BrowserPageSnapshot {
                                        tab_id: tab_id.clone(),
                                        url: origin.clone(),
                                        title: if tab.title == "New Tab" { origin } else { tab.title.clone() },
                                        dom_tree_summary: summary,
                                        text_content: snap_text,
                                        links,
                                        viewport_width: 1280,
                                        viewport_height: 800,
                                        screenshot_uri: screenshot_path,
                                        timestamp: now,
                                    };

                                    tab.last_snapshot = Some(snapshot);
                                    tab.updated_at = now;
                                    let _ = fleet.save_browser_tab(&tab);
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "saved": true,
                                        "tab_id": tab_id,
                                        "snapshot": tab.last_snapshot
                                    }));
                                }
                            }
                        }
                    }
                }

                // Fallback: HTTP GET fetch if agent-browser binary is not found
                let snapshot = if tab.url.starts_with("http://") || tab.url.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap_or_default();
                    let start = std::time::Instant::now();
                    match client.get(&tab.url).send().await {
                        Ok(resp) => {
                            let status_code = resp.status().as_u16();
                            let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
                            let duration = start.elapsed().as_millis() as u64;
                            let html_text = resp.text().await.unwrap_or_default();

                            tab.network_requests.push(BrowserNetworkRequest {
                                url: tab.url.clone(),
                                method: "GET".to_string(),
                                status: Some(status_code),
                                content_type: content_type.clone(),
                                duration_ms: duration,
                                timestamp: now,
                            });

                            let extracted_title = if let Some(start_idx) = html_text.to_lowercase().find("<title>") {
                                let rest = &html_text[start_idx + 7..];
                                rest.find("</title>").map(|end_idx| rest[..end_idx].trim().to_string())
                            } else {
                                None
                            };

                            let mut links = Vec::new();
                            let mut search_slice = html_text.as_str();
                            while let Some(href_pos) = search_slice.find("href=\"") {
                                let remainder = &search_slice[href_pos + 6..];
                                if let Some(quote_end) = remainder.find('"') {
                                    let href = &remainder[..quote_end];
                                    if (href.starts_with("http://") || href.starts_with("https://") || href.starts_with('/')) && links.len() < 25 {
                                        links.push(href.to_string());
                                    }
                                    search_slice = &remainder[quote_end..];
                                } else {
                                    break;
                                }
                            }

                            let mut plain_text = String::new();
                            let mut in_tag = false;
                            for c in html_text.chars() {
                                if c == '<' {
                                    in_tag = true;
                                } else if c == '>' {
                                    in_tag = false;
                                    plain_text.push(' ');
                                } else if !in_tag {
                                    plain_text.push(c);
                                }
                                if plain_text.len() > 4000 {
                                    break;
                                }
                            }
                            let compact_text = plain_text.split_whitespace().collect::<Vec<_>>().join(" ");
                            let final_title = extracted_title.unwrap_or_else(|| tab.url.clone());
                            tab.title = final_title.clone();

                            BrowserPageSnapshot {
                                tab_id: tab_id.clone(),
                                url: tab.url.clone(),
                                title: final_title.clone(),
                                dom_tree_summary: format!(
                                    "Page Title: {}\nHTTP Status: {}\nExtracted Links: {}\nText Preview:\n{}",
                                    final_title,
                                    status_code,
                                    links.len(),
                                    if compact_text.len() > 300 { &compact_text[..300] } else { &compact_text }
                                ),
                                text_content: compact_text,
                                links,
                                viewport_width: 1280,
                                viewport_height: 800,
                                screenshot_uri: None,
                                timestamp: now,
                            }
                        }
                        Err(err) => {
                            tab.console_logs.push(BrowserConsoleEntry {
                                level: "error".to_string(),
                                message: format!("Failed to fetch {}: {}", tab.url, err),
                                timestamp: now,
                            });
                            BrowserPageSnapshot {
                                tab_id: tab_id.clone(),
                                url: tab.url.clone(),
                                title: tab.title.clone(),
                                dom_tree_summary: format!("Connection error fetching {}: {}", tab.url, err),
                                text_content: String::new(),
                                links: Vec::new(),
                                viewport_width: 1280,
                                viewport_height: 800,
                                screenshot_uri: None,
                                timestamp: now,
                            }
                        }
                    }
                } else {
                    BrowserPageSnapshot {
                        tab_id: tab_id.clone(),
                        url: tab.url.clone(),
                        title: tab.title.clone(),
                        dom_tree_summary: format!("Local tab: {}", tab.url),
                        text_content: String::new(),
                        links: Vec::new(),
                        viewport_width: 1280,
                        viewport_height: 800,
                        screenshot_uri: None,
                        timestamp: now,
                    }
                };

                tab.last_snapshot = Some(snapshot);
                tab.updated_at = now;
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({
                        "saved": true,
                        "tab_id": tab_id,
                        "snapshot": tab.last_snapshot
                    })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CLICK => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; click is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("click")
                    .arg(&formatted_ref)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            // After click, update snapshot
                            let mut tab = match fleet.get_browser_tab(tab_id) {
                                Ok(Some(t)) => t,
                                _ => return ApiResponse::success(req.id, serde_json::json!({ "clicked": formatted_ref })),
                            };
                            let snap_out = tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("--json")
                                .arg("snapshot")
                                .arg("-i")
                                .arg("--urls")
                                .output()
                                .await;

                            if let Ok(so) = snap_out {
                                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&so.stdout) {
                                    if let Some(data) = val.get("data") {
                                        let origin = data.get("origin").and_then(|v| v.as_str()).unwrap_or(&tab.url).to_string();
                                        let snap_text = data.get("snapshot").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        tab.url = origin.clone();
                                        if let Some(snap) = tab.last_snapshot.as_mut() {
                                            snap.url = origin;
                                            snap.text_content = snap_text.clone();
                                            snap.dom_tree_summary = snap_text;
                                        }
                                        let _ = fleet.save_browser_tab(&tab);
                                    }
                                }
                            }
                            match serde_json::to_value(&tab) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(_) => ApiResponse::success(req.id, serde_json::json!({ "clicked": formatted_ref })),
                            }
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Click failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_BROWSER_TABS_FILL => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; fill is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };
                let text = match req.params.get("text").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing text parameter"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("fill")
                    .arg(&formatted_ref)
                    .arg(text)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            let tab = fleet.get_browser_tab(tab_id).ok().flatten();
                            ApiResponse::success(req.id, serde_json::json!({
                                "filled": formatted_ref,
                                "text": text,
                                "tab": tab
                            }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Fill failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_BROWSER_TABS_SCREENSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; screenshot is fail-closed",
                    );
                }
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("screenshot")
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    let image_base64 = std::fs::read(p).ok().map(|bytes| {
                                        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
                                    });
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "path": p,
                                        "tab_id": tab_id,
                                        "image_base64": image_base64
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "tab_id": tab_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Screenshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
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
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(tab_id)
                        .arg("close")
                        .output()
                        .await;
                }
                match fleet.delete_browser_tab(tab_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "closed": true, "tab_id": tab_id })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_EMULATOR_DEVICES_LIST => {
                let devices = serde_json::json!([
                    {
                        "id": "iphone_15_pro",
                        "name": "iPhone 15 Pro",
                        "platform": "iOS",
                        "width": 393,
                        "height": 852,
                        "scale": 3.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "iphone_se",
                        "name": "iPhone SE (3rd gen)",
                        "platform": "iOS",
                        "width": 375,
                        "height": 667,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "pixel_8",
                        "name": "Google Pixel 8",
                        "platform": "Android",
                        "width": 412,
                        "height": 915,
                        "scale": 2.625,
                        "user_agent": "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Mobile Safari/537.36",
                        "status": "ready"
                    },
                    {
                        "id": "ipad_mini",
                        "name": "iPad Mini",
                        "platform": "iPadOS",
                        "width": 744,
                        "height": 1133,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    }
                ]);
                ApiResponse::success(req.id, devices)
            }
            METHOD_EMULATOR_LAUNCH => {
                let device_id = req.params.get("device_id").and_then(|v| v.as_str()).unwrap_or("iphone_15_pro");
                let session_id = req.params.get("session_id").and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("mobile_{}_{}", device_id, chrono::Utc::now().timestamp_millis()));
                let url = req.params.get("url").and_then(|v| v.as_str()).unwrap_or("https://google.com");

                let (width, height, scale): (u32, u32, f64) = match device_id {
                    "iphone_se" => (375, 667, 2.0),
                    "pixel_8" => (412, 915, 2.625),
                    "ipad_mini" => (744, 1133, 2.0),
                    _ => (393, 852, 3.0),
                };

                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("set")
                        .arg("viewport")
                        .arg(width.to_string())
                        .arg(height.to_string())
                        .arg(scale.to_string())
                        .output()
                        .await;

                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "session_id": session_id,
                    "device_id": device_id,
                    "width": width,
                    "height": height,
                    "scale": scale,
                    "url": url,
                    "status": "running"
                }))
            }
            METHOD_EMULATOR_NAVIGATE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "session_id": session_id,
                    "url": url,
                    "status": "ready"
                }))
            }
            METHOD_EMULATOR_SNAPSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let snap_out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("--json")
                    .arg("snapshot")
                    .arg("-i")
                    .arg("--urls")
                    .output()
                    .await;

                match snap_out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(data) = val.get("data") {
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "session_id": session_id,
                                        "snapshot": data.get("snapshot").and_then(|v| v.as_str()).unwrap_or(""),
                                        "refs": data.get("refs").cloned().unwrap_or(serde_json::json!({})),
                                        "origin": data.get("origin").and_then(|v| v.as_str()).unwrap_or("")
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "session_id": session_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Snapshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_EMULATOR_SCREENSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("--json")
                    .arg("screenshot")
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    let image_base64 = std::fs::read(p).ok().map(|bytes| {
                                        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
                                    });
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "session_id": session_id,
                                        "path": p,
                                        "image_base64": image_base64
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "session_id": session_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Screenshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_EMULATOR_TAP => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let ref_key = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let selector = if ref_key.starts_with('@') {
                    ref_key.to_string()
                } else {
                    format!("@{ref_key}")
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("click")
                    .arg(&selector)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            ApiResponse::success(req.id, serde_json::json!({
                                "session_id": session_id,
                                "ref": selector,
                                "tapped": true
                            }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Tap failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
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
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; SSH probe is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let mut host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => return ApiResponse::error(req.id, format!("Host '{host_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin = resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o").arg("BatchMode=yes")
                   .arg("-o").arg("ConnectTimeout=3")
                   .arg("-o").arg("StrictHostKeyChecking=yes")
                   .arg("-p").arg(host.port.to_string());

                if let SshAuthMethod::KeyPair { private_key_path, .. } = &host.auth_method {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg("echo -n ping");

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        if out.status.success() {
                            host.status = RemoteHostStatus::Online;
                            host.last_ping_ms = Some(duration_ms);
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(req.id, serde_json::json!({
                                "host_id": host.id,
                                "status": "online",
                                "ping_ms": duration_ms
                            }))
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            host.status = RemoteHostStatus::Offline;
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(req.id, serde_json::json!({
                                "host_id": host.id,
                                "status": "offline",
                                "error": stderr.trim()
                            }))
                        }
                    }
                    Err(e) => {
                        host.status = RemoteHostStatus::Offline;
                        host.updated_at = now;
                        let _ = fleet.save_remote_host(&host);
                        ApiResponse::success(req.id, serde_json::json!({
                            "host_id": host.id,
                            "status": "offline",
                            "error": e.to_string()
                        }))
                    }
                }
            }
            METHOD_FLEET_EXEC => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; remote command is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let command = match req.params.get("command").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing command parameter"),
                };
                let host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => return ApiResponse::error(req.id, format!("Host '{host_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin = resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o").arg("BatchMode=yes")
                   .arg("-o").arg("ConnectTimeout=10")
                   .arg("-o").arg("StrictHostKeyChecking=yes")
                   .arg("-p").arg(host.port.to_string());

                if let SshAuthMethod::KeyPair { private_key_path, .. } = &host.auth_method {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg(command);

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let receipt = FleetExecReceipt {
                            execution_id: new_id("fexec"),
                            host_id: host.id.clone(),
                            command: command.to_string(),
                            exit_code: out.status.code(),
                            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                            duration_ms,
                            // Process exit is an operational observation, not
                            // independent verification of the remote effect.
                            verified: false,
                            timestamp: now,
                        };
                        let _ = fleet.save_fleet_execution(&receipt);
                        match serde_json::to_value(&receipt) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn SSH process: {e}"))
                    }
                }
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
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing 'job_id' parameter"),
                };

                let has_permit = req.params.get("permit_id").is_some();
                let force_execute = req.params.get("force_execute").and_then(|v| v.as_bool()).unwrap_or(false);
                if !has_permit && !force_execute {
                    return ApiResponse::error(
                        req.id,
                        "Headless execution is unavailable until an executor and scoped authority checks are connected",
                    );
                }

                match fleet.get_headless_job(job_id) {
                    Ok(Some(mut job)) => {
                        job.status = custos_domain::HeadlessJobStatus::Running;
                        job.started_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        let mut cmd = tokio::process::Command::new("sh");
                        cmd.arg("-c").arg(&job.spec.command_or_script);
                        let timeout_secs = job.spec.timeout_secs.clamp(1, 300);
                        let out = tokio::time::timeout(
                            std::time::Duration::from_secs(timeout_secs),
                            cmd.output(),
                        ).await;

                        let (stdout, stderr, exit_code, success) = match out {
                            Ok(Ok(o)) => (
                                String::from_utf8_lossy(&o.stdout).to_string(),
                                String::from_utf8_lossy(&o.stderr).to_string(),
                                o.status.code().unwrap_or(0),
                                o.status.success(),
                            ),
                            Ok(Err(e)) => (String::new(), e.to_string(), 1, false),
                            Err(_) => (String::new(), "Execution timed out".to_string(), 124, false),
                        };

                        job.status = if success {
                            custos_domain::HeadlessJobStatus::Completed
                        } else {
                            custos_domain::HeadlessJobStatus::Failed
                        };
                        job.exit_code = Some(exit_code);
                        job.output_log = if stderr.is_empty() { stdout } else { format!("{stdout}\n{stderr}") };
                        job.completed_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        match serde_json::to_value(&job) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Job '{job_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_MANIFESTS_LIST => {
                let manifests = self.pack_registry.list_packs();
                match serde_json::to_value(&manifests) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_LIST => {
                let filter_lens = req.params.get("lens").and_then(|v| v.as_str());
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let all_jobs = self.pack_registry.list_all_jobs();
                let mut catalog_items = Vec::new();

                for job in all_jobs {
                    if let Some(fl) = filter_lens {
                        let matches_lens = match job.lens {
                            custos_domain::WorkbenchLens::Coding => {
                                fl.eq_ignore_ascii_case("coding") || fl.eq_ignore_ascii_case("code")
                            }
                            custos_domain::WorkbenchLens::Research => {
                                fl.eq_ignore_ascii_case("research")
                            }
                            custos_domain::WorkbenchLens::Assistant => {
                                fl.eq_ignore_ascii_case("assistant") || fl.eq_ignore_ascii_case("chat")
                            }
                        };
                        if !matches_lens {
                            continue;
                        }
                    }

                    if let Some(fp) = filter_pack {
                        if !job.job_id.starts_with(fp) {
                            continue;
                        }
                    }

                    let (status, reason) = match job.job_id.as_str() {
                        "engineering__repo_explain"
                        | "engineering__review"
                        | "research__source_qa"
                        | "research__dataset_audit"
                        | "research__reproduce"
                        | "assistant__inbox_triage"
                        | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                        "engineering__migration" => (
                            JobOperationalStatus::Planned,
                            Some("Multi-phase dependency migration requires external package registry connector".into()),
                        ),
                        "assistant__mutate_execution" => (
                            JobOperationalStatus::Planned,
                            Some("Governed outbox execution requires configured SMTP/Calendar live connector".into()),
                        ),
                        "assistant__reminder_automation" => (
                            JobOperationalStatus::Planned,
                            Some("Recurring reminder daemon automation is planned for post-MVP".into()),
                        ),

                        _ => (
                            JobOperationalStatus::Degraded,
                            Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                        ),
                    };

                    catalog_items.push(PacksJobCatalogItem {
                        job_id: job.job_id,
                        lens: job.lens,
                        name: job.name,
                        description: job.description,
                        primary_role: job.primary_role,
                        recommended_topology: job.recommended_topology,
                        status,
                        degradation_reason: reason,
                        required_inputs: job.required_inputs,
                        expected_artifacts: job.expected_artifacts,
                        required_verifiers: job.required_verifiers,
                        acceptance_rubric: job.acceptance_rubric,
                    });
                }

                match serde_json::to_value(&catalog_items) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_GET => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                match self.pack_registry.get_job(job_id) {
                    Some(job) => {
                        let (status, reason) = match job.job_id.as_str() {
                            "engineering__repo_explain"
                            | "engineering__review"
                            | "research__source_qa"
                            | "research__dataset_audit"
                            | "research__reproduce"
                            | "assistant__inbox_triage"
                            | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                            "engineering__migration"
                            | "assistant__mutate_execution"
                            | "assistant__reminder_automation" => (
                                JobOperationalStatus::Planned,
                                Some("Capability planned for post-MVP or requires live external connector".into()),
                            ),

                            _ => (
                                JobOperationalStatus::Degraded,
                                Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                            ),
                        };

                        let item = PacksJobCatalogItem {
                            job_id: job.job_id,
                            lens: job.lens,
                            name: job.name,
                            description: job.description,
                            primary_role: job.primary_role,
                            recommended_topology: job.recommended_topology,
                            status,
                            degradation_reason: reason,
                            required_inputs: job.required_inputs,
                            expected_artifacts: job.expected_artifacts,
                            required_verifiers: job.required_verifiers,
                            acceptance_rubric: job.acceptance_rubric,
                        };

                        match serde_json::to_value(&item) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => ApiResponse::error(req.id, format!("Workbench Job '{job_id}' not found")),
                }
            }
            METHOD_PACKS_ROLES_LIST => {
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let roles = if let Some(pid) = filter_pack {
                    match self.pack_registry.get_pack(pid) {
                        Some(pack) => pack.roles(),
                        None => return ApiResponse::error(req.id, format!("Domain pack '{pid}' not found")),
                    }
                } else {
                    self.pack_registry.list_all_roles()
                };

                match serde_json::to_value(&roles) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_RUN => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                let job = match self.pack_registry.get_job(job_id) {
                    Some(j) => j,
                    None => return ApiResponse::error(req.id, format!("Workbench Job '{job_id}' not found")),
                };

                let session_id = req.params.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let workspace_id = req.params.get("workspace_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let inputs = req.params.get("inputs").cloned().unwrap_or(serde_json::Value::Null);
                if !inputs.is_null() {
                    tracing::debug!(job_id = %job_id, inputs = ?inputs, "Running job with pre-populated inputs");
                }

                let title = format!("[{}] {}", job.name, job.description);
                let mut metadata = serde_json::json!({
                    "source": "packs_job",
                    "job_id": job.job_id,
                    "pack_id": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "engineering",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "lens": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "coding",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "primary_role": job.primary_role,
                    "inputs": inputs,
                });
                if let Some(ref sid) = session_id {
                    metadata["session_id"] = serde_json::json!(sid);
                }
                if let Some(ref wid) = workspace_id {
                    metadata["workspace_id"] = serde_json::json!(wid);
                }

                let cmd = CreateTask {
                    title,
                    metadata: Some(metadata),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => {
                        if let Some(ref sid) = session_id {
                            let _ = self
                                .session_manager
                                .append_user_message(
                                    &SessionId(sid.clone()),
                                    &format!("Started workbench job `{}` (Task {})", job.job_id, task.id),
                                )
                                .await;
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "task_id": task.id,
                                "job_id": job.job_id,
                                "status": "created",
                                "lens": job.lens,
                                "primary_role": job.primary_role,
                                "recommended_topology": job.recommended_topology,
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_LIST => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let session_id = req.params.get("session_id").and_then(|v| v.as_str());
                let task_id = req.params.get("task_id").and_then(|v| v.as_str());
                match assistant.list_drafts_sync(session_id, task_id) {
                    Ok(drafts) => ApiResponse::success(req.id, serde_json::to_value(&drafts).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_GET => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match assistant.get_draft_sync(id) {
                    Ok(Some(draft)) => ApiResponse::success(req.id, serde_json::to_value(&draft).unwrap_or_default()),
                    Ok(None) => ApiResponse::error(req.id, format!("Draft '{id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_SAVE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let mut params = req.params;
                let Some(fields) = params.as_object_mut() else { return ApiResponse::error(req.id, "Expected draft object"); };
                let now = chrono::Utc::now().to_rfc3339();
                fields.entry("status").or_insert(serde_json::json!("draft"));
                fields.entry("created_at").or_insert(serde_json::json!(now));
                fields.insert("updated_at".into(), serde_json::json!(now));
                let mut draft: AssistantDraft = match serde_json::from_value(params) {
                    Ok(d) => d,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid AssistantDraft: {e}")),
                };
                if draft.id.is_empty() || draft.id.len() > 128 || draft.title.trim().is_empty()
                    || !matches!(draft.status.as_str(), "draft" | "approved" | "archived")
                    || !matches!(draft.kind.as_str(), "email" | "document" | "schedule" | "note") {
                    return ApiResponse::error(req.id, "Invalid draft id, title, kind or status");
                }
                match assistant.get_draft_sync(&draft.id) {
                    Ok(Some(existing)) => {
                        if existing.session_id != draft.session_id || existing.task_id != draft.task_id {
                            return ApiResponse::error(req.id, "Draft scope cannot be changed");
                        }
                        if existing.payload_lock.is_some() && (existing.title != draft.title || existing.content != draft.content
                            || existing.recipient != draft.recipient || existing.kind != draft.kind || existing.status != draft.status) {
                            return ApiResponse::error(req.id, "Reviewed draft is locked; create a new draft to revise it");
                        }
                        draft.created_at = existing.created_at;
                    }
                    Ok(None) => { draft.created_at = now; }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                }
                draft.payload_lock = if draft.status == "approved" {
                    let payload = serde_json::json!({"title": draft.title, "content": draft.content, "kind": draft.kind, "recipient": draft.recipient});
                    Some(blake3::hash(payload.to_string().as_bytes()).to_hex().to_string())
                } else { None };
                match assistant.save_draft_sync(&draft) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::to_value(&draft).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_DELETE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match assistant.delete_draft_sync(id) {
                    Ok(deleted) => ApiResponse::success(req.id, serde_json::json!({ "deleted": deleted })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_OUTBOX_LIST => {
                let outbox = match self.outbox.as_ref() {
                    Some(o) => o,
                    None => return ApiResponse::error(req.id, "OutboxRepository not configured on daemon"),
                };
                let pending = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Pending).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let dispatching = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Dispatching).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let receipted = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Receipted).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut all = Vec::new();
                all.extend(pending);
                all.extend(dispatching);
                all.extend(receipted);
                let items: Vec<OutboxActionItem> = all.into_iter().map(|e| {
                    OutboxActionItem {
                        id: e.id,
                        task_id: e.task_id,
                        title: format!("Outbox: {}", e.action_id),
                        action_type: e.action_id,
                        recipient: "Unknown (not stored in outbox record)".into(),
                        payload_summary: format!("Payload digest: {}", e.argument_digest),
                        payload_digest: e.argument_digest,
                        status: match e.status {
                            custos_core::contracts::storage::OutboxStatus::Pending => "pending_approval".into(),
                            custos_core::contracts::storage::OutboxStatus::Dispatching => "dispatching".into(),
                            custos_core::contracts::storage::OutboxStatus::Receipted => "receipted".into(),
                            custos_core::contracts::storage::OutboxStatus::Failed => "failed".into(),
                            custos_core::contracts::storage::OutboxStatus::Uncertain => "uncertain".into(),
                        },
                        permit_id: Some(e.permit_id),
                        receipt_id: e.receipt.as_ref().map(|r| r.receipt_id.clone()),
                        created_at: e.created_at.to_rfc3339(),
                    }
                }).collect();
                ApiResponse::success(req.id, serde_json::to_value(&items).unwrap_or_default())
            }
            METHOD_ASSISTANT_OUTBOX_CREATE | METHOD_ASSISTANT_OUTBOX_APPROVE | METHOD_ASSISTANT_OUTBOX_DISPATCH => {
                ApiResponse::error(req.id, "Outbound connector execution is not configured with a mediated permit/receipt pipeline. Save and review a local draft; Outbox is read-only.")
            }
            METHOD_WORKSPACES_AGENT_STEER => {
                let guidance = match req.params.get("guidance").and_then(|v| v.as_str()) {
                    Some(g) if !g.trim().is_empty() => g,
                    _ => return ApiResponse::error(req.id, "Missing guidance parameter"),
                };
                let harness_id = req.params.get("harness_id").and_then(|v| v.as_str());
                let run_id = req.params.get("run_id").and_then(|v| v.as_str());

                if let (Some(h_id), Some(r_id)) = (harness_id, run_id) {
                    match self.harnesses.steer(h_id, r_id, guidance).await {
                        Ok(()) => ApiResponse::success(req.id, serde_json::json!({"steered": true})),
                        Err(err) => ApiResponse::error(req.id, format!("Failed to steer harness: {err}")),
                    }
                } else if let Some(r_id) = run_id {
                    let mut steered = false;
                    for desc in self.harnesses.list() {
                        if self.harnesses.steer(&desc.id, r_id, guidance).await.is_ok() {
                            steered = true;
                            break;
                        }
                    }
                    if steered {
                        ApiResponse::success(req.id, serde_json::json!({"steered": true}))
                    } else {
                        ApiResponse::error(
                            req.id,
                            "Workspace steering is unavailable: this workspace is not bound to an active harness run",
                        )
                    }
                } else {
                    ApiResponse::error(
                        req.id,
                        "Workspace steering is unavailable: missing run_id or harness_id parameter",
                    )
                }
            }
            METHOD_RESOURCES_CAPABILITIES => {
                let resource_id = match req.params.get("resource_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing resource_id parameter"),
                };
                let descriptor = match self.list_capabilities().into_iter().find(|capability| {
                    capability.resource_id.as_deref() == Some(resource_id)
                }) {
                    Some(capability) => capability,
                    None => return ApiResponse::error(req.id, format!("Unknown resource_id: {resource_id}")),
                };
                let actions: Vec<ResourceActionCapability> = descriptor.supported_operations.iter().enumerate().map(|(index, operation)| {
                    ResourceActionCapability {
                        id: operation.clone(),
                        label: operation.replace('.', " ").replace('_', " "),
                        status: "available".into(),
                        reason: None,
                        primary: index == 0,
                        shortcut: None,
                    }
                }).collect();
                match serde_json::to_value(&actions) {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        };

        if let (Some(cmd_id), Some(digest), Some(store)) = (
            command_id.as_deref(),
            payload_digest.as_deref(),
            self.sqlite_store.as_ref(),
        ) {
            if let Some(res_val) = resp.result.as_ref() {
                let entry = custos_core::contracts::storage::CommandLedgerEntry {
                    command_id: cmd_id.to_string(),
                    method: method_name,
                    payload_digest: digest.to_string(),
                    result_json: serde_json::to_string(res_val).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    expires_at: chrono::Utc::now() + chrono::Duration::days(7),
                };
                if let Err(e) = store.command_ledger().record_command(&entry) {
                    tracing::warn!(command_id = %cmd_id, error = %e, "Failed to record command in ledger");
                }
            }
        }

        resp
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

        let assessment = dispatcher.handle_request(ApiRequest {
            id: "req_exec_assess".into(),
            method: "v1.research.executions.resume.assess".into(),
            params: serde_json::json!({ "execution_id": execution.id }),
        }).await;
        assert!(assessment.is_success());
        let assessment = assessment.result.unwrap();
        assert_eq!(assessment["can_start"], false);
        assert_eq!(assessment["state_inventory_declared"], false);
        assert_eq!(assessment["blockers"].as_array().unwrap().len(), 1);
        for params in [serde_json::json!({}), serde_json::json!({"execution_id": "missing"})] {
            assert!(!dispatcher.handle_request(ApiRequest {
                id: "req_exec_assess_bad".into(),
                method: "v1.research.executions.resume.assess".into(),
                params,
            }).await.is_success());
        }

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

        let mut projected_resources = std::collections::HashSet::new();
        for capability in &caps {
            if let Some(resource_id) = capability.resource_id.as_deref() {
                assert!(
                    projected_resources.insert(resource_id),
                    "resource_id '{resource_id}' must map to exactly one capability"
                );
            }
        }

        let research_runs = caps.iter().find(|c| c.id == "research.runs").unwrap();
        assert_eq!(research_runs.resource_id.as_deref(), Some("runs"));
        let synthesis = caps.iter().find(|c| c.id == "synthesis.handoff").unwrap();
        assert_eq!(synthesis.resource_id.as_deref(), Some("synthesis"));

        // Kernel task engine is available
        let tasks_cap = caps.iter().find(|c| c.id == "tasks.core").unwrap();
        assert!(tasks_cap.status.is_available());

        // A dispatcher without a workspace coordinator cannot offer scoped tools.
        let pty_cap = caps.iter().find(|c| c.id == "compute.pty").unwrap();
        assert!(!pty_cap.status.is_available());
        assert_eq!(pty_cap.resource_id.as_deref(), Some("terminal"));
        assert!(pty_cap.supported_operations.is_empty());

        let files_cap = caps.iter().find(|c| c.id == "code.files").unwrap();
        assert!(!files_cap.status.is_available());
        assert!(files_cap.supported_operations.is_empty());

        let changes_cap = caps.iter().find(|c| c.id == "code.changes").unwrap();
        assert!(!changes_cap.status.is_available());
        assert!(changes_cap.supported_operations.is_empty());

        let outbox_actions = dispatcher.handle_request(ApiRequest {
            id: "req_outbox_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "outbox" }),
        }).await;
        assert!(outbox_actions.is_success());
        let actions: Vec<ResourceActionCapability> =
            serde_json::from_value(outbox_actions.result.unwrap()).unwrap();
        assert!(actions.is_empty(), "unconfigured outbox must not advertise dispatch");

        let unknown_actions = dispatcher.handle_request(ApiRequest {
            id: "req_unknown_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "made_up" }),
        }).await;
        assert!(!unknown_actions.is_success());

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
    async fn task_conversation_attachment_uses_existing_task_without_duplicate_creation() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store));
        let task = service.execute_create(CreateTask { title: "Existing work".into(), metadata: None, contract: None }).await.unwrap().0;
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge);
        let request = |method: &str, params| ApiRequest { id: "task-chat".into(), method: method.into(), params };
        let session = dispatcher.handle_request(request("v1.sessions.create", serde_json::json!({"mode": "assisted"}))).await.result.unwrap();
        let id = session["id"].as_str().unwrap();
        let invalid = dispatcher.handle_request(request("v1.sessions.attach", serde_json::json!({"session_id": id, "task_id": "missing-task"}))).await;
        assert!(!invalid.is_success());
        let unbound = dispatcher.handle_request(request("v1.sessions.get", serde_json::json!({"session_id": id}))).await.result.unwrap();
        assert!(unbound.get("attached_to").is_none_or(|value| value.is_null()));
        for _ in 0..2 {
            let attached = dispatcher.handle_request(request("v1.sessions.attach", serde_json::json!({"session_id": id, "task_id": task.id}))).await;
            assert!(attached.is_success(), "{:?}", attached.error);
        }
        let attached = dispatcher.handle_request(request("v1.sessions.get", serde_json::json!({"session_id": id}))).await.result.unwrap();
        assert_eq!(attached["attached_to"], task.id);
        let tasks = dispatcher.handle_request(request("v1.tasks.list", serde_json::json!({}))).await.result.unwrap();
        assert_eq!(tasks.as_array().unwrap().len(), 1);
        assert_eq!(tasks[0]["id"], task.id);
    }

    #[tokio::test]
    async fn assistant_draft_creation_review_and_scope_are_enforced() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store));
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let db = custos_persistence::connection::DbConnection::open_in_memory().unwrap();
        let assistant = Arc::new(AssistantRepository::new(db.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_assistant(assistant.clone());
        let request = |params| ApiRequest { id: "draft-test".into(), method: METHOD_ASSISTANT_DRAFTS_SAVE.into(), params };
        let created = dispatcher.handle_request(request(serde_json::json!({"id": "draft-test", "title": "Draft", "kind": "email", "content": "Exact text", "session_id": "session-a"}))).await;
        assert!(created.is_success(), "{:?}", created.error);
        let mut draft = created.result.unwrap();
        assert_eq!(draft["status"], "draft");
        assert!(draft["created_at"].as_str().is_some());
        draft["session_id"] = serde_json::json!("session-b");
        assert!(!dispatcher.handle_request(request(draft.clone())).await.is_success());
        draft["session_id"] = serde_json::json!("session-a");
        draft["status"] = serde_json::json!("approved");
        draft["payload_lock"] = serde_json::json!("locked");
        let reviewed = dispatcher.handle_request(request(draft)).await;
        assert!(reviewed.is_success(), "{:?}", reviewed.error);
        let mut reviewed = reviewed.result.unwrap();
        assert_eq!(reviewed["payload_lock"].as_str().unwrap().len(), 64);
        reviewed["content"] = serde_json::json!("Changed after review");
        assert!(!dispatcher.handle_request(request(reviewed)).await.is_success());
        assert_eq!(assistant.get_draft_sync("draft-test").unwrap().unwrap().content, "Exact text");
        let events: i64 = db.lock().unwrap().query_row("SELECT COUNT(*) FROM outbox_messages WHERE topic = 'assistant.draft.saved'", [], |row| row.get(0)).unwrap();
        assert_eq!(events, 2, "only committed mutations emit notifications");
        for method in [METHOD_ASSISTANT_OUTBOX_CREATE, METHOD_ASSISTANT_OUTBOX_APPROVE, METHOD_ASSISTANT_OUTBOX_DISPATCH] {
            assert!(!dispatcher.handle_request(ApiRequest { id: "outbound-test".into(), method: method.into(), params: serde_json::json!({}) }).await.is_success());
        }
    }

    #[tokio::test]
    async fn candidate_creation_and_integration_replay_use_recorded_workspace_state() {
        use custos_core::contracts::WorkspaceRepository;
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repository"); std::fs::create_dir(&repo).unwrap();
        let git = |path: &std::path::Path, args: &[&str]| {
            let output = std::process::Command::new("git").current_dir(path)
                .env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1")
                .args(["-c", "user.name=Custos Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false"])
                .args(args).output().unwrap();
            assert!(output.status.success(), "{:?}: {}", args, String::from_utf8_lossy(&output.stderr));
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        git(&repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("file.txt"), "base").unwrap(); git(&repo, &["add", "."]); git(&repo, &["commit", "-m", "base"]);
        // Merge commands inherit only fixture-local identity, never the user's repo configuration.
        git(&repo, &["config", "user.name", "Custos Test"]); git(&repo, &["config", "user.email", "test@example.invalid"]); git(&repo, &["config", "commit.gpgsign", "false"]);
        let store = Arc::new(custos_persistence::SqliteTaskStore::new(&temp.path().join("test.db").to_string_lossy()).unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let task = service.execute_create(CreateTask { title: "Candidate integration test".into(), metadata: None, contract: None }).await.unwrap().0;
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(store.clone(), Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new())));
        let mut base = custos_domain::ExecutionWorkspace::new(custos_domain::WorkspaceId::new("base"), "base", custos_domain::WorkspaceKind::Git { repo_path: repo.to_string_lossy().into(), branch: "main".into(), base_commit: None }, repo.to_string_lossy().to_string());
        base.status = custos_domain::WorkspaceStatus::Ready; store.save_workspace(&base).await.unwrap();
        let make_dispatcher = || LocalApiDispatcher::new(service.clone(), sessions.clone(), bridge.clone()).with_workspace(coordinator.clone());
        let dispatcher = make_dispatcher();
        let spawn_params = serde_json::json!({"task_id": task.id, "base_workspace_id": "base", "count": 1, "command_id": "create-candidates-once"});
        let request = |method: &str, params: serde_json::Value| ApiRequest { id: "fixture-request".into(), method: method.into(), params };
        let first = dispatcher.handle_request(request(METHOD_WORKSPACES_CANDIDATES_SPAWN, spawn_params.clone())).await;
        assert!(first.is_success(), "{:?}", first.error);
        let first = first.result.unwrap();
        let winner_id = first["candidates"][0]["id"].as_str().unwrap().to_string();
        let candidate = std::path::PathBuf::from(first["candidates"][0]["path"].as_str().unwrap());
        let second = make_dispatcher().handle_request(request(METHOD_WORKSPACES_CANDIDATES_SPAWN, spawn_params.clone())).await;
        assert!(second.is_success(), "{:?}", second.error);
        assert_eq!(second.result.unwrap()["candidates"][0]["id"], winner_id);
        assert_eq!(store.list_workspaces().await.unwrap().len(), 2);
        let mut different = spawn_params; different["count"] = serde_json::json!(2);
        assert!(!dispatcher.handle_request(request(METHOD_WORKSPACES_CANDIDATES_SPAWN, different)).await.is_success());
        std::fs::write(candidate.join("file.txt"), "candidate").unwrap(); git(&candidate, &["add", "."]); git(&candidate, &["commit", "-m", "candidate"]);
        let before = git(&repo, &["rev-parse", "HEAD"]);
        assert!(!dispatcher.handle_request(request(METHOD_WORKSPACES_INTEGRATE, serde_json::json!({"winner_workspace_id": winner_id}))).await.is_success());
        let preview = dispatcher.handle_request(request(METHOD_WORKSPACES_INTEGRATE, serde_json::json!({"winner_workspace_id": winner_id, "preview": true}))).await;
        assert!(preview.is_success(), "{:?}", preview.error);
        let preview = preview.result.unwrap();
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before, "preview must not merge");
        let apply = serde_json::json!({"winner_workspace_id": winner_id, "expected_target_head": preview["current_target_head"], "expected_winner_head": preview["winner_head"]});
        let mut stale = apply.clone(); stale["expected_winner_head"] = serde_json::json!("stale");
        assert!(!dispatcher.handle_request(request(METHOD_WORKSPACES_INTEGRATE, stale)).await.is_success());
        let integrated = dispatcher.handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply.clone())).await;
        assert!(integrated.is_success(), "{:?}", integrated.error);
        assert_eq!(integrated.result.unwrap()["merge_success"], true);
        let merged_head = git(&repo, &["rev-parse", "HEAD"]);
        assert_ne!(merged_head, before);
        let repeated = make_dispatcher().handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply.clone())).await;
        assert!(repeated.is_success()); assert_eq!(repeated.result.unwrap()["merge_success"], true);
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), merged_head);
        let mut interrupted = store.get_workspace(&custos_domain::WorkspaceId::new(&winner_id)).await.unwrap().unwrap();
        interrupted.metadata["integration"].as_object_mut().unwrap().remove("result");
        store.save_workspace(&interrupted).await.unwrap();
        // Recovery follows the recorded target even when the user switches checkout.
        git(&repo, &["checkout", "-b", "unrelated", &before]);
        let reconciled = make_dispatcher().handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply)).await;
        assert!(reconciled.is_success()); assert_eq!(reconciled.result.unwrap()["merge_status"], "merged_reconciled");
        assert_eq!(git(&repo, &["rev-parse", "main"]), merged_head);
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before, "recovery must not change the user's checkout");
        assert!(candidate.exists(), "integration never removes candidate checkout");
    }

    #[tokio::test]
    async fn candidate_comparison_rejects_mixed_scope_and_reports_inspection_failure() {
        use custos_core::contracts::WorkspaceRepository;
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(store.clone(), Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new())));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_workspace(coordinator);
        let make = |id: &str, task: &str, base: &str, repo: &str| {
            let mut workspace = custos_domain::ExecutionWorkspace::new(custos_domain::WorkspaceId::new(id), id.to_string(),
                custos_domain::WorkspaceKind::Git { repo_path: repo.into(), branch: id.into(), base_commit: Some(base.into()) },
                format!("/missing/candidate/{id}"));
            workspace.status = custos_domain::WorkspaceStatus::Ready;
            workspace.owner_task_id = Some(task.into()); workspace.base_commit_hash = Some(base.into()); workspace
        };
        for workspace in [make("a", "task-a", "base-a", "/repo-a"), make("b", "task-b", "base-a", "/repo-a"), make("c", "task-a", "base-c", "/repo-a"), make("d", "task-a", "base-a", "/repo-d")] {
            store.save_workspace(&workspace).await.unwrap();
        }
        for ids in [serde_json::json!(["a", "missing"]), serde_json::json!(["a", "b"]), serde_json::json!(["a", "c"]), serde_json::json!(["a", "d"]), serde_json::json!(["a", "a"]), serde_json::json!(["a", 42])] {
            let response = dispatcher.handle_request(ApiRequest { id: "compare".into(), method: METHOD_WORKSPACES_CANDIDATES_COMPARE.into(), params: serde_json::json!({"workspace_ids": ids}) }).await;
            assert!(!response.is_success(), "comparison must reject mismatched or invalid candidates");
        }
        let response = dispatcher.handle_request(ApiRequest { id: "inspect".into(), method: METHOD_WORKSPACES_CANDIDATES_COMPARE.into(), params: serde_json::json!({"workspace_ids": ["a"]}) }).await;
        assert!(response.is_success());
        let candidate = &response.result.unwrap()["candidates"][0];
        assert_eq!(candidate["is_ready_for_integration"], false);
        assert!(candidate["inspection_error"].is_string());
    }

    #[tokio::test]
    async fn artifact_reads_are_workspace_scoped_and_annotations_are_version_bound() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(store.clone(), Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new())));
        let research = Arc::new(store.research().clone());
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_workspace(coordinator).with_research(research.clone());
        let path = temp.path().join("workspace").to_string_lossy().to_string();
        let created = dispatcher.handle_request(ApiRequest { id: "artifact-workspace".into(), method: METHOD_WORKSPACES_CREATE.into(), params: serde_json::json!({"name": "artifacts", "kind": {"type": "folder", "path": path}, "path": path}) }).await;
        assert!(created.is_success(), "{:?}", created.error);
        let workspace_id = created.result.unwrap()["id"].as_str().unwrap().to_string();
        let file = std::path::Path::new(&path).join("analysis.py");
        std::fs::write(&file, "print(42)").unwrap();
        research.record_artifact_lineage(&ArtifactLineageNode { artifact_path: "analysis.py".into(), version: 1, content_hash: blake3::hash(b"print(42)").to_hex().to_string(), produced_by_run_id: None, parent_version_hash: None, timestamp: 0 }).unwrap();
        let read = |params| ApiRequest { id: "artifact-read".into(), method: METHOD_ARTIFACTS_GET.into(), params };
        let scoped = dispatcher.handle_request(read(serde_json::json!({"artifact_path": "analysis.py", "workspace_id": workspace_id}))).await;
        assert!(scoped.is_success(), "{:?}", scoped.error);
        assert_eq!(scoped.result.as_ref().unwrap()["content_version"], 1);
        assert_eq!(scoped.result.as_ref().unwrap()["content"], "print(42)");
        std::fs::write(&file, "print('changed')").unwrap();
        let changed = dispatcher.handle_request(read(serde_json::json!({"artifact_path": "analysis.py", "workspace_id": workspace_id}))).await;
        assert!(changed.is_success());
        assert!(changed.result.as_ref().unwrap().get("content_version").is_none());
        assert!(changed.result.as_ref().unwrap()["content"].is_null());
        let escaped = dispatcher.handle_request(read(serde_json::json!({"artifact_path": "../secret.txt", "workspace_id": workspace_id}))).await;
        assert!(!escaped.is_success());
        let unscoped = dispatcher.handle_request(read(serde_json::json!({"artifact_path": file.to_string_lossy()}))).await;
        assert!(unscoped.is_success());
        assert!(unscoped.result.as_ref().unwrap()["content"].is_null());
        let annotation = custos_domain::AnnotationRecord::new_text("analysis.py", 1, 0, 5, "print", "Check this calculation", "researcher");
        let save = |annotation: &custos_domain::AnnotationRecord| ApiRequest { id: "annotation".into(), method: "v1.research.annotations.save".into(), params: serde_json::to_value(annotation).unwrap() };
        assert!(dispatcher.handle_request(save(&annotation)).await.is_success());
        let mut forged = annotation.clone(); forged.artifact_version = 999;
        assert!(!dispatcher.handle_request(save(&forged)).await.is_success());
        forged = annotation.clone(); forged.status = custos_domain::AnnotationStatus::Addressed;
        assert!(!dispatcher.handle_request(save(&forged)).await.is_success());
        assert_eq!(research.list_annotations_for_artifact("analysis.py", Some(1)).unwrap().len(), 1);
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
        let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
            store.clone(), Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new())));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_research(research).with_workspace(coordinator);
        let session_id = "test_nb_session_42";
        let path = temp_dir.path().join("notebook-workspace").to_string_lossy().to_string();
        let created = dispatcher.handle_request(ApiRequest { id: "nb_workspace".into(), method: METHOD_WORKSPACES_CREATE.into(),
            params: serde_json::json!({"name": "notebook", "kind": {"type": "folder", "path": path}, "path": path}) }).await;
        assert!(created.is_success(), "{:?}", created.error);
        let workspace_id = created.result.unwrap()["id"].as_str().unwrap().to_string();

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

        let mut forged = cell.clone();
        forged.status = custos_domain::CellExecutionStatus::Success;
        forged.stdout = Some("fabricated output".into());
        forged.execution_count = Some(99);
        let forged_resp = dispatcher.handle_request(ApiRequest {
            id: "nb_forged_result".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({ "session_id": session_id, "cells": [forged] }),
        }).await;
        assert!(forged_resp.is_success());

        let mut wrong_session = cell.clone();
        wrong_session.session_id = "other_session".into();
        let wrong_resp = dispatcher.handle_request(ApiRequest {
            id: "nb_wrong_session".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({ "session_id": session_id, "cells": [wrong_session] }),
        }).await;
        assert!(!wrong_resp.is_success());

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
        assert_eq!(loaded_cells[0].status, custos_domain::CellExecutionStatus::Idle);
        assert_eq!(loaded_cells[0].execution_count, None);
        assert_eq!(loaded_cells[0].stdout, None);

        let unscoped = dispatcher.handle_request(ApiRequest { id: "nb_unscoped".into(), method: METHOD_NOTEBOOK_EXECUTE.into(), params: serde_json::json!({"session_id": session_id, "cell_id": cell.id, "code": cell.source}) }).await;
        assert!(!unscoped.is_success());
        let changed_source = dispatcher.handle_request(ApiRequest { id: "nb_changed".into(), method: METHOD_NOTEBOOK_EXECUTE.into(), params: serde_json::json!({"session_id": session_id, "cell_id": cell.id, "code": "print('not saved')", "workspace_id": workspace_id}) }).await;
        assert!(!changed_source.is_success());

        // 4. Execute cell
        let exec_req = ApiRequest {
            id: "nb_exec_1".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": "print(6 * 7)",
                "workspace_id": workspace_id,
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

        // Changing code into notes must not retain executed Python output.
        let mut markdown = cell.clone();
        markdown.cell_type = custos_domain::NotebookCellType::Markdown;
        let save_markdown = dispatcher.handle_request(ApiRequest {
            id: "nb_markdown".into(), method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({"session_id": session_id, "cells": [markdown]}),
        }).await;
        assert!(save_markdown.is_success(), "{:?}", save_markdown.error);
        let notes = dispatcher.handle_request(ApiRequest {
            id: "nb_notes".into(), method: METHOD_NOTEBOOK_CELLS_LIST.into(),
            params: serde_json::json!({"session_id": session_id}),
        }).await.result.unwrap();
        assert_eq!(notes[0]["cell_type"], "markdown");
        assert!(notes[0]["stdout"].is_null());
        assert!(notes[0]["execution_count"].is_null());
        let execute_notes = dispatcher.handle_request(ApiRequest {
            id: "nb_notes_execute".into(), method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({"session_id": session_id, "cell_id": cell.id, "code": cell.source, "workspace_id": workspace_id}),
        }).await;
        assert!(!execute_notes.is_success());

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
        if browser_execution_adapter_connected() {
            assert!(nav_res.is_success());
        } else {
            assert!(!nav_res.is_success());
            assert!(nav_res.error.unwrap().contains("browser adapter"));
        }

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
        if browser_execution_adapter_connected() {
            assert!(snap_res.is_success());
        } else {
            assert!(!snap_res.is_success());
            assert!(snap_res.error.unwrap().contains("browser adapter"));
        }

        let close_req = ApiRequest {
            id: "bt_close_1".into(),
            method: METHOD_BROWSER_TABS_CLOSE.into(),
            params: serde_json::json!({ "tab_id": tab.id }),
        };
        let close_res = dispatcher.handle_request(close_req).await;
        assert!(close_res.is_success());

        // 2. Fleet inventory is durable, while probe/exec use real OpenSSH transport.
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
        if remote_execution_adapter_connected() {
            assert!(ping_res.is_success());
            let result_val = ping_res.result.unwrap();
            assert_eq!(result_val.get("status").and_then(|v| v.as_str()), Some("offline"));
        } else {
            assert!(!ping_res.is_success());
            assert!(ping_res.error.unwrap().contains("remote execution adapter"));
        }

        let exec_fleet_req = ApiRequest {
            id: "fleet_exec_1".into(),
            method: METHOD_FLEET_EXEC.into(),
            params: serde_json::json!({
                "host_id": host.id,
                "command": "nvidia-smi --query-gpu=name --format=csv"
            }),
        };
        let exec_fleet_res = dispatcher.handle_request(exec_fleet_req).await;
        if remote_execution_adapter_connected() {
            assert!(exec_fleet_res.is_success());
            let exec_val = exec_fleet_res.result.unwrap();
            let receipt: FleetExecReceipt = serde_json::from_value(exec_val).unwrap();
            assert_eq!(receipt.host_id, host.id);
            assert!(!receipt.verified);
        } else {
            assert!(!exec_fleet_res.is_success());
            assert!(exec_fleet_res
                .error
                .unwrap()
                .contains("remote execution adapter"));
        }

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
    async fn provider_usage_reads_durable_runs_without_duplicates_or_secrets() {
        use custos_core::contracts::storage::RunPort;
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::with_store(store.clone()));
        let bridge = Arc::new(BridgeService::new(sessions.clone(),service.clone()));
        let dispatcher = LocalApiDispatcher::new(service,sessions,bridge).with_usage_runs(store.clone()).with_providers(Arc::new(store.providers().clone()));
        let response = dispatcher.handle_request(ApiRequest::new("task", "v1.tasks.create", serde_json::json!({"title":"Usage fixture"}))).await;
        let id = response.result.as_ref().unwrap()["id"].as_str().unwrap().to_string();
        let mut run = custos_domain::Run::new(id,1);
        run.metadata=serde_json::json!({"actual_mode":"model","provider_account_id":"account-A","requested_model":"ollama/test","usage":{"input_tokens":7,"output_tokens":3,"total_tokens":10},"secret":"never-export"});
        store.save_run(&run).await.unwrap();
        for _ in 0..2 {
            let report=dispatcher.handle_request(ApiRequest::new("usage", "v1.providers.usage",serde_json::json!({}))).await;
            assert!(report.is_success());
            let value=report.result.unwrap();
            assert_eq!(value["records"].as_array().unwrap().len(),1);
            assert_eq!(value["records"][0]["account_id"],"account-A");
            assert_eq!(value["records"][0]["total_tokens"],10);
            assert!(!value.to_string().contains("never-export"));
        }
        run.metadata["usage"]=serde_json::Value::Null;
        run.metadata["failure_reason"]=serde_json::json!("429 RESOURCE_EXHAUSTED");
        store.save_run(&run).await.unwrap();
        let value=dispatcher.handle_request(ApiRequest::new("usage", "v1.providers.usage",serde_json::json!({}))).await.result.unwrap();
        assert!(value["records"][0]["total_tokens"].is_null());
        assert_eq!(value["records"][0]["rate_limited"],true);
    }

    #[tokio::test]
    async fn discovered_catalog_ignores_legacy_cache_and_preserves_verified_snapshot() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let _ = socket.read(&mut buf).unwrap();
            let body = r#"{"data":[{"id":"only-live-model","name":"Live Model"}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge)
            .with_providers(Arc::new(store.providers().clone()));
        let save = dispatcher.handle_request(ApiRequest { id: "save".into(), method: METHOD_PROVIDERS_SAVE.into(),
            params: serde_json::json!({"id":"account", "service_type":"openai", "name":"Account", "endpoint_url":base, "api_key":"eyJ-api-key", "status":"active"}) }).await;
        assert!(save.is_success());
        let baseline = custos_adapters::providers::catalog::default_canonical_models();
        store
            .providers()
            .save_catalog_models("account", &baseline)
            .unwrap();
        let catalog_request = || ApiRequest {
            id: "catalog".into(),
            method: METHOD_MODELS_CATALOG.into(),
            params: serde_json::json!({"provider_id":"account"}),
        };
        let legacy = dispatcher
            .handle_request(catalog_request())
            .await
            .result
            .unwrap();
        assert!(legacy["models"].as_array().unwrap().is_empty());
        let probe_request = || ApiRequest {
            id: "probe".into(),
            method: METHOD_MODELS_PROBE.into(),
            params: serde_json::json!({"provider_id":"account"}),
        };
        let live = dispatcher.handle_request(probe_request()).await;
        assert!(live.is_success(), "{:?}", live.error);
        server.join().unwrap();
        let snapshot = dispatcher
            .handle_request(catalog_request())
            .await
            .result
            .unwrap();
        assert_eq!(snapshot["models"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["models"][0]["id"], "only-live-model");
        assert_eq!(snapshot["models"][0]["provider_type"], "openai");
        assert!(snapshot["fetched_at"].as_i64().unwrap() > 0);
        assert!(!dispatcher
            .handle_request(probe_request())
            .await
            .is_success());
        assert_eq!(
            dispatcher
                .handle_request(catalog_request())
                .await
                .result
                .unwrap(),
            snapshot
        );
        let other = dispatcher
            .handle_request(ApiRequest {
                id: "other".into(),
                method: METHOD_MODELS_CATALOG.into(),
                params: serde_json::json!({"provider_id":"other-account"}),
            })
            .await
            .result
            .unwrap();
        assert!(other["models"].as_array().unwrap().is_empty());
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
        dispatcher.credential_service().unwrap().save_oauth_token(&token_record).await.unwrap();

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

    #[tokio::test]
    async fn test_packs_api_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("custos_packs_test.db");
        let store =
            Arc::new(custos_persistence::SqliteTaskStore::new(&db_path.to_string_lossy()).unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let pack_registry = Arc::new(custos_packs::create_default_pack_registry());
        let dispatcher = LocalApiDispatcher::new(task_service.clone(), session_manager.clone(), bridge_service)
            .with_packs(pack_registry);

        // 1. List Pack Manifests via v1.packs.manifests.list
        let man_req = ApiRequest {
            id: "pack_1".into(),
            method: METHOD_PACKS_MANIFESTS_LIST.into(),
            params: serde_json::json!({}),
        };
        let man_res = dispatcher.handle_request(man_req).await;
        assert!(man_res.is_success(), "man_res failed: {:?}", man_res.error);
        let manifests: Vec<custos_packs::DomainPackManifest> =
            serde_json::from_value(man_res.result.unwrap()).unwrap();
        assert_eq!(manifests.len(), 3);
        assert!(manifests.iter().any(|m| m.id == "engineering"));
        assert!(manifests.iter().any(|m| m.id == "research"));
        assert!(manifests.iter().any(|m| m.id == "assistant"));

        // 2. List Jobs (All 20 Jobs) via v1.packs.jobs.list
        let jobs_req = ApiRequest {
            id: "pack_2".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({}),
        };
        let jobs_res = dispatcher.handle_request(jobs_req).await;
        assert!(jobs_res.is_success());
        let catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(jobs_res.result.unwrap()).unwrap();
        assert_eq!(catalog.len(), 20, "Expected 20 standardized Workbench Jobs");

        // Verify status mapping:
        let repo_explain = catalog.iter().find(|j| j.job_id == "engineering__repo_explain").unwrap();
        assert_eq!(repo_explain.status, JobOperationalStatus::Available);
        assert!(repo_explain.degradation_reason.is_none());

        let source_qa = catalog.iter().find(|j| j.job_id == "research__source_qa").unwrap();
        assert_eq!(source_qa.status, JobOperationalStatus::Available);

        let inbox_triage = catalog.iter().find(|j| j.job_id == "assistant__inbox_triage").unwrap();
        assert_eq!(inbox_triage.status, JobOperationalStatus::Available);

        let bugfix = catalog.iter().find(|j| j.job_id == "engineering__bugfix").unwrap();
        assert_eq!(bugfix.status, JobOperationalStatus::Degraded);
        assert!(bugfix.degradation_reason.is_some());

        let migration = catalog.iter().find(|j| j.job_id == "engineering__migration").unwrap();
        assert_eq!(migration.status, JobOperationalStatus::Planned);

        // 2b. Filter Jobs by Lens
        let coding_req = ApiRequest {
            id: "pack_2b".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({ "lens": "coding" }),
        };
        let coding_res = dispatcher.handle_request(coding_req).await;
        assert!(coding_res.is_success());
        let coding_catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(coding_res.result.unwrap()).unwrap();
        assert_eq!(coding_catalog.len(), 7, "Expected 7 Engineering Jobs in coding lens");

        let research_req = ApiRequest {
            id: "pack_2c".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({ "lens": "research" }),
        };
        let research_res = dispatcher.handle_request(research_req).await;
        let research_catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(research_res.result.unwrap()).unwrap();
        assert_eq!(research_catalog.len(), 8, "Expected 8 Research Jobs in research lens");

        // 3. Get Specific Job via v1.packs.jobs.get
        let get_req = ApiRequest {
            id: "pack_3".into(),
            method: METHOD_PACKS_JOBS_GET.into(),
            params: serde_json::json!({ "job_id": "engineering__repo_explain" }),
        };
        let get_res = dispatcher.handle_request(get_req).await;
        assert!(get_res.is_success());
        let job_item: PacksJobCatalogItem =
            serde_json::from_value(get_res.result.unwrap()).unwrap();
        assert_eq!(job_item.job_id, "engineering__repo_explain");
        assert_eq!(job_item.primary_role, "repo_scout");
        assert_eq!(job_item.status, JobOperationalStatus::Available);

        // 3b. Get Missing Job
        let missing_req = ApiRequest {
            id: "pack_3b".into(),
            method: METHOD_PACKS_JOBS_GET.into(),
            params: serde_json::json!({ "job_id": "non_existent_job" }),
        };
        let missing_res = dispatcher.handle_request(missing_req).await;
        assert!(!missing_res.is_success());
        assert!(missing_res.error.unwrap().contains("not found"));

        // 4. List Roles via v1.packs.roles.list
        let roles_req = ApiRequest {
            id: "pack_4".into(),
            method: METHOD_PACKS_ROLES_LIST.into(),
            params: serde_json::json!({}),
        };
        let roles_res = dispatcher.handle_request(roles_req).await;
        assert!(roles_res.is_success());
        let all_roles: Vec<custos_packs::WorkerRole> =
            serde_json::from_value(roles_res.result.unwrap()).unwrap();
        assert_eq!(all_roles.len(), 12, "Expected 12 specialized Worker Roles");

        let eng_roles_req = ApiRequest {
            id: "pack_4b".into(),
            method: METHOD_PACKS_ROLES_LIST.into(),
            params: serde_json::json!({ "pack_id": "engineering" }),
        };
        let eng_roles_res = dispatcher.handle_request(eng_roles_req).await;
        let eng_roles: Vec<custos_packs::WorkerRole> =
            serde_json::from_value(eng_roles_res.result.unwrap()).unwrap();
        assert_eq!(eng_roles.len(), 4, "Expected 4 Engineering Worker Roles");

        // 5. Run Workbench Job via v1.packs.jobs.run
        let session = session_manager.create_session(custos_domain::SessionMode::Bare).await;
        let run_req = ApiRequest {
            id: "pack_5".into(),
            method: METHOD_PACKS_JOBS_RUN.into(),
            params: serde_json::json!({
                "job_id": "engineering__repo_explain",
                "session_id": session.id.0,
                "inputs": {
                    "query": "TokenBucket",
                    "scope": ["crates/custos-core"]
                }
            }),
        };
        let run_res = dispatcher.handle_request(run_req).await;
        assert!(run_res.is_success(), "run_res failed: {:?}", run_res.error);
        let run_val = run_res.result.unwrap();
        assert_eq!(run_val.get("status").unwrap().as_str(), Some("created"));
        assert_eq!(run_val.get("job_id").unwrap().as_str(), Some("engineering__repo_explain"));
        let task_id = run_val.get("task_id").unwrap().as_str().unwrap();

        // Verify task was created in TaskService with provenance metadata
        let task = task_service.get_task(task_id).await.unwrap().expect("task exists");
        assert_eq!(task.metadata.get("source").unwrap().as_str(), Some("packs_job"));
        assert_eq!(task.metadata.get("job_id").unwrap().as_str(), Some("engineering__repo_explain"));
        assert_eq!(task.metadata.get("pack_id").unwrap().as_str(), Some("engineering"));
        assert_eq!(task.metadata.get("primary_role").unwrap().as_str(), Some("repo_scout"));
    }

    #[tokio::test]
    async fn test_emulator_api_lifecycle() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let dispatcher = LocalApiDispatcher::new(
            task_service.clone(),
            session_manager.clone(),
            bridge_service.clone(),
        );

        // 1. List devices
        let list_req = ApiRequest {
            id: "emu_list_1".into(),
            method: METHOD_EMULATOR_DEVICES_LIST.into(),
            params: serde_json::json!({}),
        };
        let list_res = dispatcher.handle_request(list_req).await;
        assert!(list_res.is_success());
        let devices = list_res.result.unwrap();
        assert!(devices.as_array().unwrap().len() >= 4);

        // 2. Launch emulator device
        let launch_req = ApiRequest {
            id: "emu_launch_1".into(),
            method: METHOD_EMULATOR_LAUNCH.into(),
            params: serde_json::json!({
                "device_id": "iphone_15_pro",
                "session_id": "test_emu_session",
                "url": "https://example.com"
            }),
        };
        let launch_res = dispatcher.handle_request(launch_req).await;
        assert!(launch_res.is_success());
        let launch_val = launch_res.result.unwrap();
        assert_eq!(launch_val.get("session_id").unwrap().as_str(), Some("test_emu_session"));
        assert_eq!(launch_val.get("width").unwrap().as_u64(), Some(393));
        assert_eq!(launch_val.get("height").unwrap().as_u64(), Some(852));
    }
}

#[cfg(test)]
mod conversation_effort_api_tests {
    use super::*;
    struct EffortModel;
    #[async_trait::async_trait]
    impl custos_provider::ModelPort for EffortModel {
        fn provider_id(&self) -> &str { "fixture" }
        async fn generate(&self, request:&custos_provider::ProviderRequest) -> Result<custos_provider::ModelResponse,DomainError> {
            assert_eq!(request.reasoning_effort.as_deref(),Some("ultra"));
            assert_eq!(request.provider_account_id.as_deref(),Some("account-A"));
            assert_eq!(request.model,"gpt-6.1-sol");
            Ok(custos_provider::ModelResponse::text("answer","gpt-6.1-sol",3))
        }
    }
    #[tokio::test]
    async fn background_start_api_preserves_effort_through_persisted_run_and_provider_request() {
        use custos_core::contracts::storage::RunPort;
        let dir=tempfile::tempdir().unwrap();let db=dir.path().join("reasoning-api.db");
        let runtime=crate::runtime::CustosRuntime::bootstrap_with_model(db.to_str().unwrap(),Arc::new(EffortModel)).unwrap();
        let request=|method:&str,params| ApiRequest{id:"fixture".into(),method:method.into(),params};
        let task=runtime.local_api.handle_request(request("v1.tasks.create",serde_json::json!({"title":"Reasoning fixture"}))).await.result.unwrap();
        let session=runtime.local_api.handle_request(request("v1.sessions.create",serde_json::json!({"mode":"assisted"}))).await.result.unwrap();
        let response=runtime.local_api.handle_request(request("v1.workflow.start_run",serde_json::json!({"background":true,"task_id":task["id"],"session_id":session["id"],"turn_id":"turn-fixture","preferred_mode":"model","prompt":"hello","model":"gpt-6.1-sol","provider_account_id":"account-A","reasoning_effort":"ultra"}))).await;
        assert!(response.is_success(),"{:?}",response.error);
        let handle: custos_domain::RunHandle=serde_json::from_value(response.result.unwrap()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5),async {
            loop { let run=runtime.store.get_run(&handle.run_id).await.unwrap().unwrap();
                if run.status.is_terminal() { assert_eq!(run.status,custos_domain::RunStatus::Completed);assert_eq!(run.metadata["reasoning_effort"],"ultra");assert_eq!(run.metadata["provider_account_id"],"account-A");break; }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.unwrap();
    }
}

#[cfg(test)]
mod git_and_github_workspace_detection_tests {
    use super::*;

    #[test]
    fn github_status_requires_successful_verified_viewer_and_never_exposes_secrets() {
        for (success, body) in [(false, br#"{"login":"someone","token":"secret"}"#.as_slice()),
            (true, b"not json".as_slice()), (true, br#"{"login":""}"#.as_slice()),
            (true, br#"{"message":"Bad credentials"}"#.as_slice())] {
            let result = github_auth_metadata(success, body);
            assert_eq!(result["authenticated"], false);
            assert!(result.get("token").is_none());
        }
        let result = github_auth_metadata(true, br#"{"login":"verified","name":"Human","token":"secret","other":"private"}"#);
        assert_eq!(result["authenticated"], true);
        assert_eq!(result["user"]["login"], "verified");
        assert!(!result.to_string().contains("secret"));
        assert!(result["user"].get("other").is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn github_login_pty_supports_real_prompts_and_cancellation_without_credentials() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let fake_gh = directory.path().join("fake gh's cli");
        std::fs::write(&fake_gh, "#!/bin/sh\nprintf 'Code: TEST-CODE\\nPress Enter\\n'\nread reply\nprintf 'Browser approval pending\\n'\nread reply\n").unwrap();
        std::fs::set_permissions(&fake_gh, std::fs::Permissions::from_mode(0o700)).unwrap();
        let terminal = TerminalCoordinator::new();
        let command = github_login_command(&fake_gh);
        let session = terminal.spawn("github-auth", directory.path(), Some(&command), 100, 28).await.unwrap();
        let mut output = String::new();
        let mut seq = 0;
        for _ in 0..100 {
            let chunk = terminal.read_output(session.id.as_str(), seq, 16384).await.unwrap();
            seq = chunk.next_seq;
            output.push_str(&chunk.data);
            if output.contains("Press Enter") { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(output.contains("TEST-CODE"), "Real PTY should expose the device code");
        terminal.write(session.id.as_str(), b"\r").await.unwrap();
        for _ in 0..100 {
            let chunk = terminal.read_output(session.id.as_str(), seq, 16384).await.unwrap();
            seq = chunk.next_seq;
            output.push_str(&chunk.data);
            if output.contains("Browser approval pending") { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(output.contains("Browser approval pending"));
        terminal.terminate(session.id.as_str()).await.unwrap();
        assert!(matches!(terminal.get_session(session.id.as_str()).await.unwrap().status, custos_domain::TerminalSessionStatus::Terminated));
    }

    #[test]
    fn github_login_quotes_executable_and_uses_source_control_flow() {
        let command = github_login_command(std::path::Path::new("/tmp/it's gh"));
        assert!(command.starts_with("exec '/tmp/it'\\''s gh' "));
        assert!(command.contains("auth login --hostname github.com"));
        assert!(!command.contains("copilot"));
    }

    #[test]
    fn test_parse_github_remote_various_formats() {
        // SSH scp-style
        let (host, owner, repo, avatar) = parse_github_remote("git@github.com:civi0411/Custos.git");
        assert_eq!(host.as_deref(), Some("github.com"));
        assert_eq!(owner.as_deref(), Some("civi0411"));
        assert_eq!(repo.as_deref(), Some("Custos"));
        assert_eq!(avatar.as_deref(), Some("https://github.com/civi0411.png"));

        // HTTPS style
        let (host, owner, repo, avatar) = parse_github_remote("https://github.com/civi0411/AgentHub.git");
        assert_eq!(host.as_deref(), Some("github.com"));
        assert_eq!(owner.as_deref(), Some("civi0411"));
        assert_eq!(repo.as_deref(), Some("AgentHub"));
        assert_eq!(avatar.as_deref(), Some("https://github.com/civi0411.png"));

        // HTTPS without .git
        let (host, owner, repo, _) = parse_github_remote("https://github.com/facebook/react");
        assert_eq!(host.as_deref(), Some("github.com"));
        assert_eq!(owner.as_deref(), Some("facebook"));
        assert_eq!(repo.as_deref(), Some("react"));

        // Enterprise GitHub
        let (host, owner, repo, avatar) = parse_github_remote("git@ghe.corporate.com:ai-team/core-model.git");
        assert_eq!(host.as_deref(), Some("ghe.corporate.com"));
        assert_eq!(owner.as_deref(), Some("ai-team"));
        assert_eq!(repo.as_deref(), Some("core-model"));
        assert_eq!(avatar, None);
    }

    #[tokio::test]
    async fn test_deep_git_paths_standard_and_worktree() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().join("main_repo");
        tokio::fs::create_dir_all(&repo_path).await.unwrap();

        // Standard directory .git
        let dot_git_dir = repo_path.join(".git");
        tokio::fs::create_dir_all(&dot_git_dir).await.unwrap();
        let res = resolve_deep_git_paths(&repo_path).await;
        assert!(res.is_some());
        let (git_dir, common_dir, is_wt) = res.unwrap();
        assert_eq!(git_dir, dot_git_dir);
        assert_eq!(common_dir, dot_git_dir);
        assert!(!is_wt);

        // Linked worktree with .git file
        let wt_path = temp_dir.path().join("worktree_child");
        tokio::fs::create_dir_all(&wt_path).await.unwrap();
        let wt_gitdir = dot_git_dir.join("worktrees/child-1");
        tokio::fs::create_dir_all(&wt_gitdir).await.unwrap();
        tokio::fs::write(wt_gitdir.join("commondir"), "../..\n").await.unwrap();

        let dot_git_file = wt_path.join(".git");
        tokio::fs::write(&dot_git_file, format!("gitdir: {}\n", wt_gitdir.display())).await.unwrap();

        let wt_res = resolve_deep_git_paths(&wt_path).await;
        assert!(wt_res.is_some());
        let (wt_actual_gitdir, wt_actual_common, wt_is_wt) = wt_res.unwrap();
        assert_eq!(wt_actual_gitdir, wt_gitdir.canonicalize().unwrap());
        assert_eq!(wt_actual_common, dot_git_dir.canonicalize().unwrap());
        assert!(wt_is_wt);
    }

    #[tokio::test]
    async fn test_github_auth_status_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("gh-api.db");
        let runtime = crate::runtime::CustosRuntime::bootstrap(db.to_str().unwrap()).unwrap();
        let req = ApiRequest {
            id: "gh-auth-req".into(),
            method: METHOD_GITHUB_AUTH_STATUS.into(),
            params: serde_json::json!({}),
        };
        let res = runtime.local_api.handle_request(req).await;
        assert!(res.is_success());
        let val = res.result.unwrap();
        assert_eq!(val["source"], "gh_cli");
    }

    #[tokio::test]
    async fn test_workspaces_detect_empty_or_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("ws-detect.db");
        let runtime = crate::runtime::CustosRuntime::bootstrap(db.to_str().unwrap()).unwrap();
        
        // Test with empty path string
        let req1 = ApiRequest {
            id: "ws-det-1".into(),
            method: METHOD_WORKSPACES_DETECT.into(),
            params: serde_json::json!({ "path": "" }),
        };
        let res1 = runtime.local_api.handle_request(req1).await;
        assert!(res1.is_success());
        let val1 = res1.result.unwrap();
        assert!(val1["path"].is_string());
        assert_eq!(val1["is_git"], true);

        // Test with missing path parameter entirely
        let req2 = ApiRequest {
            id: "ws-det-2".into(),
            method: METHOD_WORKSPACES_DETECT.into(),
            params: serde_json::json!({}),
        };
        let res2 = runtime.local_api.handle_request(req2).await;
        assert!(res2.is_success());
        let val2 = res2.result.unwrap();
        assert!(val2["path"].is_string());
        assert_eq!(val2["is_git"], true);
    }

    #[tokio::test]
    async fn test_command_id_dedup_and_conflict_rejection() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::with_store(store.clone()));
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
            .with_sqlite_store(store.clone());

        let cmd_id = "cmd_stable_12345";
        let req1 = ApiRequest {
            id: cmd_id.into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({
                "title": "Idempotent Task",
                "command_id": cmd_id,
            }),
        };

        // 1. First execution
        let res1 = dispatcher.handle_request(req1.clone()).await;
        assert!(res1.is_success());
        let task1 = res1.result.unwrap();
        let task_id1 = task1["id"].as_str().unwrap().to_string();

        // 2. Exact duplicate retry with same command_id and payload
        let res2 = dispatcher.handle_request(req1).await;
        assert!(res2.is_success());
        let task2 = res2.result.unwrap();
        let task_id2 = task2["id"].as_str().unwrap().to_string();

        // Must return identical task_id, NOT create a second task!
        assert_eq!(task_id1, task_id2);

        // 3. Same command_id with DIFFERENT payload -> must be rejected with Conflict error
        let req3 = ApiRequest {
            id: "retry_conflict".into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({
                "title": "Different Title Attack",
                "command_id": cmd_id,
            }),
        };
        let res3 = dispatcher.handle_request(req3).await;
        assert!(!res3.is_success());
        assert!(res3.error.unwrap().contains("Command ID conflict"));
    }
}

