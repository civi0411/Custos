use crate::api::LocalApiDispatcher;
use custos_adapters::harness::{ClaudeCodeHarnessAdapter, HarnessRegistry};
use custos_adapters::providers::{
    ClaudeProvider, CodexProvider, CredentialResolver, FakeProvider, LocalModelProvider,
    RouterModelProvider,
};
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_bridge::BridgeService;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::TaskService;
use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;
use custos_provider::ModelPort;
use custos_runtime::session::SessionManager;
use custos_runtime::workflow::TaskRuntime;
use std::sync::Arc;

#[allow(dead_code)]
pub struct CustosRuntime {
    pub store: Arc<SqliteTaskStore>,
    pub task_service: Arc<TaskService>,
    pub session_manager: Arc<SessionManager>,
    pub bridge_service: Arc<BridgeService>,
    pub local_api: Arc<LocalApiDispatcher>,

    // Architecture Ports
    pub kernel: Arc<dyn KernelPort>,
    pub sandbox: Arc<dyn SandboxPort>,
    pub model: Arc<dyn ModelPort>,
    pub harness: Arc<dyn AgentRuntimePort>,
    pub harnesses: Arc<HarnessRegistry>,
    pub workflow: Arc<dyn WorkflowPort>,
    pub lease_manager: Arc<custos_runtime::workflow::WorkspaceLeaseManager>,
    pub workspace_coordinator: Arc<custos_runtime::workspace::WorkspaceCoordinator>,
    pub terminal_coordinator: Arc<custos_runtime::TerminalCoordinator>,
    pub python_coordinator: Arc<custos_runtime::PythonKernelCoordinator>,
}

impl CustosRuntime {
    pub fn bootstrap_profile(profile: &crate::profile::ProfileResolver) -> Result<Self, DomainError> {
        let db_path = profile.database_path();
        Self::bootstrap(&db_path.to_string_lossy())
    }

    pub fn bootstrap(database_path: &str) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);

        // Wire live OpenAI provider with dynamic OAuth token & proactive refresh
        let daemon_token_provider = Arc::new(DaemonOAuthTokenProvider {
            store: store.clone(),
        });
        let openai_provider = custos_adapters::providers::openai_chat::OpenAiChatProvider::new()
            .with_token_provider(daemon_token_provider);
        let codex_provider = Arc::new(CodexProvider::with_provider(openai_provider));

        // Multi-provider router: dynamically dispatches to OpenAI, Anthropic, Gemini, DeepSeek, or Ollama/local
        let credential_resolver = Arc::new(DaemonCredentialResolver {
            store: store.clone(),
        });
        let router_provider = Arc::new(RouterModelProvider::new(credential_resolver));

        let provider_name = std::env::var("CUSTOS_PROVIDER").unwrap_or_default();
        let model: Arc<dyn ModelPort> = match provider_name.to_lowercase().as_str() {
            "fake" => Arc::new(FakeProvider::new("fake")),
            "claude" | "anthropic" => Arc::new(ClaudeProvider::new()),
            "local" => Arc::new(LocalModelProvider::new()),
            "codex" => codex_provider.clone(),
            _ => router_provider.clone(),
        };

        Self::bootstrap_with_store_and_model(store, model)
    }

    pub fn bootstrap_with_model(database_path: &str, model: Arc<dyn ModelPort>) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);
        Self::bootstrap_with_store_and_model(store, model)
    }

    fn bootstrap_with_store_and_model(
        store: Arc<SqliteTaskStore>,
        model: Arc<dyn ModelPort>,
    ) -> Result<Self, DomainError> {
        // Crash Recovery Reconcile (Gate 3): transition any InFlight effects to Uncertain on startup
        store.outbox().reconcile_on_startup()?;
        store.reconcile_runs_on_startup()?;
        store.seed_canonical_data_if_empty()?;
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));

        let kernel = Arc::new(TrustedKernel::new(store.clone()));

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root.clone()));

        // Background OAuth Token Sweeper Job: runs every 5 minutes and refreshes expiring tokens
        let sweeper_store = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let now_ms = chrono::Utc::now().timestamp_millis();
                // Check tokens expiring within the next 10 minutes (600 seconds)
                let threshold_ms = now_ms + (600 * 1000);
                if let Ok(expiring) = sweeper_store.providers().list_expiring_oauth_tokens(threshold_ms) {
                    let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::default();
                    for tok in expiring {
                        tracing::info!(provider_id = %tok.provider_id, "Sweeper job refreshing expiring OAuth token");
                        if let Ok(refreshed) = manager.refresh_token("custos-openai-desktop", &tok).await {
                            let _ = sweeper_store.providers().save_oauth_token(&refreshed);
                            tracing::info!(provider_id = %tok.provider_id, "Sweeper job updated refreshed OAuth token in database");
                        }
                    }
                }
            }
        });

        // Sovereign Coding Harness Adapter & Multi-Harness Registry
        let lease_manager = Arc::new(custos_runtime::workflow::WorkspaceLeaseManager::new(
            workspace_root.clone(),
        ));

        let mut harness_reg = HarnessRegistry::default_with_workspace(workspace_root.clone());
        let governed = Arc::new(custos_runtime::agent::runtime_port::GovernedAgentRuntime::new(
            model.clone(),
            "Custos Governed Agent Runtime",
            vec![],
        ));
        harness_reg.register(
            governed,
            "Governed Agent Runtime",
            "Custos kernel-mediated autonomous agent runtime with ExecutionPermits.",
            "internal",
        );
        let harnesses = Arc::new(harness_reg);
        let harness = harnesses
            .get("claude-code")
            .unwrap_or_else(|| Arc::new(ClaudeCodeHarnessAdapter::new(workspace_root)));

        let workflow = Arc::new(
            TaskRuntime::new()
                .with_kernel(kernel.clone())
                .with_model(model.clone())
                .with_harness(harness.clone())
                .with_sandbox(sandbox.clone())
                .with_outbox(store.clone())
                .with_effect_ledger(store.clone())
                .with_run_store(store.clone())
                .with_lease_manager(lease_manager.clone()),
        );

        let workspace_provider = Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
        let workspace_coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
            store.clone(),
            workspace_provider,
        ));
        let terminal_coordinator = Arc::new(custos_runtime::TerminalCoordinator::new());
        let python_coordinator = Arc::new(custos_runtime::PythonKernelCoordinator::new());

        let oauth_callback_server = Arc::new(crate::oauth_callback_server::OAuthCallbackServer::new(
            store.clone(),
        ));

        let local_api = Arc::new(
            LocalApiDispatcher::new(
                task_service.clone(),
                session_manager.clone(),
                bridge_service.clone(),
            )
            .with_workflow(workflow.clone())
            .with_workspace(workspace_coordinator.clone())
            .with_terminal(terminal_coordinator.clone())
            .with_python_kernel(python_coordinator.clone())
            .with_research(Arc::new(store.research().clone()))
            .with_providers(Arc::new(store.providers().clone()))
            .with_fleet_automation(Arc::new(store.fleet_automation().clone()))
            .with_oauth_callback_server(oauth_callback_server)
            .with_harnesses(harnesses.clone()),
        );

        Ok(Self {
            store,
            task_service,
            session_manager,
            bridge_service,
            local_api,
            kernel,
            sandbox,
            model,
            harness,
            harnesses,
            workflow,
            lease_manager,
            workspace_coordinator,
            terminal_coordinator,
            python_coordinator,
        })
    }
}

struct DaemonOAuthTokenProvider {
    store: Arc<SqliteTaskStore>,
}

#[async_trait::async_trait]
impl custos_adapters::providers::openai_chat::TokenProvider for DaemonOAuthTokenProvider {
    async fn get_access_token(&self) -> Result<String, DomainError> {
        let repo = self.store.providers();
        if let Ok(Some(tok)) = repo.get_oauth_token("openai") {
            if tok.is_expired(120) {
                if tok.refresh_token.is_some() {
                    let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::default();
                    if let Ok(refreshed) = manager.refresh_token("custos-openai-desktop", &tok).await {
                        let _ = repo.save_oauth_token(&refreshed);
                        tracing::info!("Proactively refreshed OAuth token before inference turn");
                        return Ok(refreshed.access_token);
                    }
                }
            }
            return Ok(tok.access_token);
        }

        if let Ok(env_key) = std::env::var("OPENAI_API_KEY") {
            if !env_key.trim().is_empty() {
                return Ok(env_key.trim().to_string());
            }
        }

        Err(DomainError::Validation(
            "No OpenAI credentials found. Please authenticate via OAuth 2.0 PKCE or configure an API key in Settings.".into(),
        ))
    }
}

struct DaemonCredentialResolver {
    store: Arc<SqliteTaskStore>,
}

#[async_trait::async_trait]
impl CredentialResolver for DaemonCredentialResolver {
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        let repo = self.store.providers();
        if let Ok(Some(tok)) = repo.get_oauth_token(provider_id) {
            if tok.is_expired(120) {
                if tok.refresh_token.is_some() {
                    let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::default();
                    let client_id = if provider_id == "openai" {
                        "custos-openai-desktop"
                    } else {
                        "custos-desktop"
                    };
                    if let Ok(refreshed) = manager.refresh_token(client_id, &tok).await {
                        let _ = repo.save_oauth_token(&refreshed);
                        tracing::info!(provider = %provider_id, "Proactively refreshed OAuth token before model inference turn");
                        return Ok(Some(refreshed.access_token));
                    }
                }
            }
            return Ok(Some(tok.access_token));
        }

        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            _ => "",
        };
        if !env_var.is_empty() {
            if let Ok(val) = std::env::var(env_var) {
                if !val.trim().is_empty() {
                    return Ok(Some(val.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    async fn get_endpoint_url(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        let repo = self.store.providers();
        if let Ok(Some(prov)) = repo.get_provider(provider_id) {
            if let Some(endpoint) = prov.endpoint_url {
                if !endpoint.trim().is_empty() {
                    return Ok(Some(endpoint.trim().to_string()));
                }
            }
        }
        Ok(None)
    }
}


