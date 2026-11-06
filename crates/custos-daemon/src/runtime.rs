use crate::api::LocalApiDispatcher;
pub use crate::credential_resolver::*;
pub use crate::tool_executor::*;
use custos_adapters::harness::{ClaudeCodeHarnessAdapter, HarnessRegistry};
use custos_adapters::providers::RouterModelProvider;
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
    pub credentials: Arc<crate::credentials::ProviderCredentials>,
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
    pub pack_registry: Arc<custos_packs::DomainPackRegistry>,
}

impl CustosRuntime {
    pub fn bootstrap_profile(
        profile: &crate::profile::ProfileResolver,
    ) -> Result<Self, DomainError> {
        let db_path = profile.database_path();
        Self::bootstrap(&db_path.to_string_lossy())
    }

    pub fn bootstrap(database_path: &str) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);

        // Multi-provider router: dynamically dispatches to OpenAI, Anthropic, Gemini, DeepSeek, or Ollama/local
        let credentials = Self::credential_service(&store);
        let availability = custos_adapters::providers::catalog::ModelAvailability::default();
        let credential_resolver = Arc::new(DaemonCredentialResolver {
            availability: availability.clone(),
            store: store.clone(),
            credentials: credentials.clone(),
        });

        let local_adapter = match custos_adapters::local_inference::InferenceRuntime::get_or_init()
        {
            Ok(runtime) => {
                let local_provider = Arc::new(
                    custos_adapters::local_inference::LocalInferenceProvider::new(runtime),
                );
                Some(Arc::new(
                    custos_adapters::local_inference::NativeLocalModelAdapter::new(local_provider),
                ) as Arc<dyn ModelPort>)
            }
            Err(err) => {
                tracing::warn!(error = %err, "Local inference engine not available on this platform");
                None
            }
        };

        let mut router = RouterModelProvider::new(credential_resolver);
        if let Some(adapter) = local_adapter {
            router = router.with_native_local(adapter);
        }
        let router_provider = Arc::new(router);

        // Production inference always goes through the configured provider router.
        // Deterministic fake providers remain test fixtures and cannot be enabled
        // by an environment variable in a user daemon.
        let model: Arc<dyn ModelPort> = router_provider.clone();

        Self::bootstrap_with_store_and_model(store, model, credentials, availability)
    }

    pub fn bootstrap_with_model(
        database_path: &str,
        model: Arc<dyn ModelPort>,
    ) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);
        let credentials = Self::credential_service(&store);
        Self::bootstrap_with_store_and_model(store, model, credentials, Default::default())
    }

    pub(crate) fn credential_service(
        store: &Arc<SqliteTaskStore>,
    ) -> Arc<crate::credentials::ProviderCredentials> {
        let repo = Arc::new(store.providers().clone());
        #[cfg(test)]
        {
            crate::credential_tests::fake_credentials(repo)
        }
        #[cfg(not(test))]
        {
            let vault: Arc<dyn custos_core::contracts::credentials::CredentialVault> =
                if std::env::var("CUSTOS_VAULT_BACKEND").as_deref() == Ok("keyring") {
                    Arc::new(custos_adapters::providers::credential_vault::KeyringCredentialVault)
                } else {
                    Arc::new(custos_adapters::providers::credential_vault::FileCredentialVault::new(
                        custos_adapters::providers::credential_vault::FileCredentialVault::default_dir(),
                    ))
                };
            Arc::new(crate::credentials::ProviderCredentials::new(repo, vault))
        }
    }

    fn bootstrap_with_store_and_model(
        store: Arc<SqliteTaskStore>,
        model: Arc<dyn ModelPort>,
        credentials: Arc<crate::credentials::ProviderCredentials>,
        availability: custos_adapters::providers::catalog::ModelAvailability,
    ) -> Result<Self, DomainError> {
        let startup_credentials = credentials.clone();
        tokio::spawn(async move {
            if startup_credentials.initialize().await.is_err() {
                tracing::warn!(
                    "Provider secure storage needs attention; affected accounts remain blocked"
                );
            }
        });
        // Crash Recovery Reconcile (Gate 3): transition any InFlight effects to Uncertain on startup
        store.outbox().reconcile_on_startup()?;
        store.reconcile_runs_on_startup()?;
        let _ = store
            .command_ledger()
            .reconcile_orphaned_inflight_commands(24);
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let source_acq = Arc::new(
            custos_adapters::tools::source_fetch::SourceFetchAdapter::with_base_dir(
                &workspace_root,
            ),
        );
        let kernel = Arc::new(TrustedKernel::with_source_acquisition(
            store.clone(),
            source_acq,
        ));
        let sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root.clone()));

        // Background OAuth Token Sweeper Job: runs every 5 minutes and refreshes expiring tokens
        let sweeper_credentials = credentials.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let now_ms = chrono::Utc::now().timestamp_millis();
                // Check tokens expiring within the next 10 minutes (600 seconds)
                let threshold_ms = now_ms + (600 * 1000);
                if let Ok(expiring) = sweeper_credentials
                    .list_expiring_oauth_tokens(threshold_ms)
                    .await
                {
                    for tok in expiring {
                        let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(
                            &tok.service_type,
                        ) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                            &settings.0,
                            &settings.1,
                        );
                        tracing::info!(provider_id = %tok.provider_id, "Sweeper job refreshing expiring OAuth token");
                        if let Ok(refreshed) = manager.refresh_token(&settings.2, &tok).await {
                            match sweeper_credentials.save_refreshed(&tok, &refreshed).await {
                                Ok(()) => {
                                    tracing::info!(provider_id = %tok.provider_id, "Sweeper saved refreshed credential")
                                }
                                Err(_) => {
                                    tracing::warn!(provider_id = %tok.provider_id, "Sweeper could not save refreshed credential")
                                }
                            }
                        }
                    }
                }
            }
        });

        // Background Uncertain Runs Reconciler (Phase 1 / F04): polls uncertain runs every 60s
        let uncertain_store = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Ok(uncertain_runs) = uncertain_store.list_uncertain_runs() {
                    let now = chrono::Utc::now();
                    for mut run in uncertain_runs {
                        let is_expired = (now - run.started_at).num_hours() >= 24;

                        if is_expired {
                            tracing::warn!(run_id = %run.id, "Uncertain run timed out after 24h; marking failed");
                            run.status = custos_domain::RunStatus::Failed;
                            run.ended_at = Some(now);
                            let _ = uncertain_store.runs().save_run(&mut run);
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
        let default_tools = vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read contents of a file within the workspace",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative or absolute path within workspace" }
                        },
                        "required": ["path"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write content to a file within the workspace",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative or absolute path within workspace" },
                            "content": { "type": "string", "description": "File text content" }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_dir",
                    "description": "List files and directories in a workspace folder",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Directory path (defaults to '.')" }
                        }
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "bash",
                    "description": "Run a shell command safely inside the sandbox",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "Shell command to execute" },
                            "timeout_secs": { "type": "integer", "description": "Execution timeout in seconds" }
                        },
                        "required": ["command"]
                    }
                }
            }),
        ];

        let developer_sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root.clone()));
        let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
            developer_sandbox.clone(),
            kernel.clone(),
            store.outbox().clone(),
        ));
        let governed = Arc::new(
            custos_runtime::agent::runtime_port::GovernedAgentRuntime::new(
                model.clone(),
                "Custos Governed Agent Runtime",
                default_tools,
            )
            .with_tool_executor(tool_executor),
        );
        harness_reg.register(
            governed,
            "Governed Agent Runtime",
            "Custos kernel-mediated autonomous agent runtime with ExecutionPermits.",
            "internal",
        );
        let default_harness_id = harness_reg.default_harness_id().map(str::to_string);
        let harness_runtimes = harness_reg.runtimes();
        let harnesses = Arc::new(harness_reg);
        let harness = harnesses
            .get("claude-code")
            .unwrap_or_else(|| Arc::new(ClaudeCodeHarnessAdapter::new(workspace_root)));

        let workspace_provider =
            Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
        let workspace_coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
            store.clone(),
            workspace_provider,
        ));

        let admission_coordinator =
            Arc::new(custos_runtime::compute::WorkloadAdmissionCoordinator::default());
        custos_adapters::local_inference::set_admission_coordinator(admission_coordinator.clone());

        let workflow = Arc::new(
            TaskRuntime::new()
                .with_kernel(kernel.clone())
                .with_model(model.clone())
                .with_harnesses(harness_runtimes, default_harness_id.as_deref())
                .with_sandbox(sandbox.clone())
                .with_outbox(store.clone())
                .with_effect_ledger(store.clone())
                .with_run_store(store.clone())
                .with_session_store(store.clone())
                .with_journal_manager(session_manager.clone())
                .with_workspace_coordinator(workspace_coordinator.clone())
                .with_admission_coordinator(admission_coordinator.clone()),
        );
        workflow.validate_ready()?;
        let active_claims = store.runs().list_active_claims()?;
        workflow
            .dispatcher()
            .restore_from_active_claims(active_claims);
        let terminal_coordinator = Arc::new(custos_runtime::TerminalCoordinator::new(Arc::new(
            custos_adapters::terminal::NativePtyDriver,
        )));
        let python_coordinator = Arc::new(custos_runtime::PythonKernelCoordinator::new(Arc::new(
            custos_adapters::notebook::LocalPythonDriver,
        )));
        let files_coordinator =
            Arc::new(custos_runtime::workspace::WorkspaceFilesCoordinator::new());
        let mcp_hub = Arc::new(custos_adapters::mcp::McpFederationHub::new());

        let oauth_callback_server = Arc::new(
            crate::oauth_callback_server::OAuthCallbackServer::with_credentials(
                credentials.clone(),
            ),
        );

        // Auto-probe pre-existing local packages from disk (~/.custos/packages) into inventory
        let packages_dir = dirs::home_dir()
            .map(|h| h.join(".custos").join("packages"))
            .unwrap_or_else(|| std::path::PathBuf::from(".custos/packages"));
        if let Ok(discovered) = custos_adapters::packages::scan_local_packages(&packages_dir) {
            for pkg in discovered {
                let _ = store.packages().register_discovered_candidate(
                    &pkg.component_id,
                    &pkg.version,
                    &pkg.platform,
                    &pkg.digest,
                    &pkg.installed_path.to_string_lossy(),
                );
            }
        }

        let enabled_components = match store.packages().list_enabled_components() {
            Ok(comps) => comps,
            Err(e) => {
                tracing::error!("Failed to query enabled packages from store: {e}; falling closed to safe built-in 'assistant' only");
                vec!["assistant".into()]
            }
        };
        let enabled_refs: Vec<&str> = enabled_components.iter().map(String::as_str).collect();
        let pack_registry = Arc::new(custos_packs::create_pack_registry_from_inventory(
            &enabled_refs,
        ));
        // not-integrated: gateway awaits product caller and lifecycle test (F4)
        // not-integrated: in-memory memory store awaits product caller (F3)

        let api_deps = crate::api::LocalApiDependencies {
            terminal: terminal_coordinator.clone(),
            files: files_coordinator,
            harnesses: harnesses.clone(),
            python_kernel: Some(python_coordinator.clone()),
            pack_registry: pack_registry.clone(),
            mcp_hub,
        };

        let local_api = Arc::new(
            LocalApiDispatcher::with_dependencies(
                task_service.clone(),
                session_manager.clone(),
                bridge_service.clone(),
                api_deps,
            )
            .with_workflow(workflow.clone())
            .with_usage_runs(store.clone())
            .with_sqlite_store(store.clone())
            .with_workspace(workspace_coordinator.clone())
            .with_research(Arc::new(store.research().clone()))
            .with_model_availability(availability)
            .with_providers(Arc::new(store.providers().clone()))
            .with_credentials(credentials.clone())
            .with_fleet_automation(Arc::new(store.fleet_automation().clone()))
            .with_oauth_callback_server(oauth_callback_server)
            .with_assistant(Arc::new(store.assistant().clone()))
            .with_outbox(Arc::new(store.outbox().clone()))
            .with_admission_coordinator(admission_coordinator.clone()),
        );

        Ok(Self {
            store,
            credentials,
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
            pack_registry,
        })
    }
}
