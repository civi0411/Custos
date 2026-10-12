use crate::api::LocalApiDispatcher;
use custos_adapters::harness::{ClaudeCodeHarnessAdapter, HarnessRegistry};
use custos_adapters::providers::{CredentialResolver, FakeProvider, RouterModelProvider};
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
        let credential_resolver = Arc::new(DaemonCredentialResolver {
            store: store.clone(), credentials: credentials.clone(),
        });
        let router_provider = Arc::new(RouterModelProvider::new(credential_resolver));

        let provider_name = std::env::var("CUSTOS_PROVIDER").unwrap_or_default();
        let model: Arc<dyn ModelPort> = match provider_name.to_lowercase().as_str() {
            "fake" => Arc::new(FakeProvider::new("fake")),
            // All configured model API connections currently converge through
            // the router compatibility path. Do not select the fail-closed
            // Claude/local wrapper types as production implementations merely
            // because their Rust modules exist. Native coding agents use the
            // AgentRuntimePort harness registry below, not ModelPort.
            _ => router_provider.clone(),
        };

        Self::bootstrap_with_store_and_model(store, model, credentials)
    }

    pub fn bootstrap_with_model(
        database_path: &str,
        model: Arc<dyn ModelPort>,
    ) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);
        let credentials = Self::credential_service(&store);
        Self::bootstrap_with_store_and_model(store, model, credentials)
    }

    fn credential_service(store: &Arc<SqliteTaskStore>) -> Arc<crate::credentials::ProviderCredentials> {
        let repo = Arc::new(store.providers().clone());
        #[cfg(test)]
        { crate::credential_tests::fake_credentials(repo) }
        #[cfg(not(test))]
        { Arc::new(crate::credentials::ProviderCredentials::new(repo, Arc::new(custos_adapters::providers::credential_vault::KeyringCredentialVault))) }
    }

    fn bootstrap_with_store_and_model(
        store: Arc<SqliteTaskStore>,
        model: Arc<dyn ModelPort>,
        credentials: Arc<crate::credentials::ProviderCredentials>,
    ) -> Result<Self, DomainError> {
        let startup_credentials = credentials.clone();
        tokio::spawn(async move { if startup_credentials.initialize().await.is_err() { tracing::warn!("Provider secure storage needs attention; affected accounts remain blocked"); } });
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
        let sweeper_credentials = credentials.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let now_ms = chrono::Utc::now().timestamp_millis();
                // Check tokens expiring within the next 10 minutes (600 seconds)
                let threshold_ms = now_ms + (600 * 1000);
                if let Ok(expiring) = sweeper_credentials
                    .list_expiring_oauth_tokens(threshold_ms).await
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
                                Ok(()) => tracing::info!(provider_id = %tok.provider_id, "Sweeper saved refreshed credential"),
                                Err(_) => tracing::warn!(provider_id = %tok.provider_id, "Sweeper could not save refreshed credential"),
                            }
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
        let governed = Arc::new(
            custos_runtime::agent::runtime_port::GovernedAgentRuntime::new(
                model.clone(),
                "Custos Governed Agent Runtime",
                vec![],
            ),
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
                .with_lease_manager(lease_manager.clone()),
        );

        let workspace_provider =
            Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
        let workspace_coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
            store.clone(),
            workspace_provider,
        ));
        let terminal_coordinator = Arc::new(custos_runtime::TerminalCoordinator::new());
        let python_coordinator = Arc::new(custos_runtime::PythonKernelCoordinator::new());

        let oauth_callback_server = Arc::new(
            crate::oauth_callback_server::OAuthCallbackServer::with_credentials(credentials.clone()),
        );

        let pack_registry = Arc::new(custos_packs::create_default_pack_registry());

        let local_api = Arc::new(
            LocalApiDispatcher::new(
                task_service.clone(),
                session_manager.clone(),
                bridge_service.clone(),
            )
            .with_workflow(workflow.clone())
            .with_usage_runs(store.clone())
            .with_workspace(workspace_coordinator.clone())
            .with_terminal(terminal_coordinator.clone())
            .with_python_kernel(python_coordinator.clone())
            .with_research(Arc::new(store.research().clone()))
            .with_providers(Arc::new(store.providers().clone()))
            .with_credentials(credentials.clone())
            .with_fleet_automation(Arc::new(store.fleet_automation().clone()))
            .with_oauth_callback_server(oauth_callback_server)
            .with_harnesses(harnesses.clone())
            .with_packs(pack_registry.clone()),
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

pub(crate) fn reserve_provider_account(repo: &custos_persistence::ProviderRepository, service: &str, model: &str) -> Result<Option<String>, DomainError> {
    let providers = repo.list_providers()?;
    let mut eligible = Vec::new();
    let mut stored = false;
    for account in providers.iter().filter(|p| p.service_type == service) {
        stored = true;
        if !matches!(account.status.as_str(), "active"|"configured"|"primary"|"standby") { continue; }
        let metadata = repo.credential_metadata(&account.id)?;
        if service != "local" && !metadata.as_ref().is_some_and(|m| m.state == "active" && (m.has_refresh_token || m.expires_at > chrono::Utc::now().timestamp_millis())) { continue; }
        let oauth = metadata.as_ref().is_some_and(|m| m.has_refresh_token || m.scope.as_deref().is_some_and(|s| !s.is_empty()));
        let endpoint = account.endpoint_url.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or(match service {
            "openai" => "https://api.openai.com/v1", "anthropic" => "https://api.anthropic.com/v1", "gemini" => "https://generativelanguage.googleapis.com", "copilot" => "https://api.githubcopilot.com", "antigravity" => "https://cloudcode-pa.googleapis.com", "deepseek" => "https://api.deepseek.com/v1", "local" => "http://localhost:11434", _ => "",
        });
        let key = format!("verified-probe:{}:{}:{}:{}",account.id,endpoint.trim().trim_end_matches('/'),if oauth {"oauth"} else {"api_key"},metadata.as_ref().map(|m| m.created_at).unwrap_or(0));
        if repo.list_catalog_models(Some(&key))?.iter().any(|m|m.id.eq_ignore_ascii_case(model)) {
            eligible.push(account.id.clone());
        }
    }
    if stored && eligible.is_empty() { return Err(DomainError::Validation("No active account advertises this model. Refresh models or reconnect the provider.".into())); }
    let selected = repo.select_account(service,&eligible)?;
    tracing::debug!(service, account_id = ?selected, "Reserved provider account");
    Ok(selected)
}

struct DaemonCredentialResolver {
    store: Arc<SqliteTaskStore>,
    credentials: Arc<crate::credentials::ProviderCredentials>,
}

#[async_trait::async_trait]
impl CredentialResolver for DaemonCredentialResolver {
    async fn select_account(&self, service: &str, model: &str) -> Result<Option<String>, DomainError> {
        reserve_provider_account(&self.store.providers(), service, model)
    }
    async fn validate_account(&self, account_id: &str, service: &str) -> Result<(), DomainError> {
        let record = self.store.providers().get_provider(account_id)?.ok_or_else(|| DomainError::Validation("Selected provider account no longer exists".into()))?;
        if record.service_type != service {
            return Err(DomainError::Validation("Selected account does not belong to the requested model provider".into()));
        }
        if !matches!(service,"local"|"ollama") && self.credentials.get_oauth_token(account_id).await?.is_none() {
            return Err(DomainError::Validation("secure_storage_missing: Reconnect this provider account".into()));
        }
        Ok(())
    }
    async fn uses_oauth(&self, provider_id: &str) -> Result<bool, DomainError> {
        Ok(self
            .credentials
            .get_oauth_token(provider_id).await?
            .is_some_and(|tok| {
                tok.refresh_token.is_some()
                    || tok.scope.as_deref().is_some_and(|s| !s.trim().is_empty())
            }))
    }
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        if let Some(tok) = self.credentials.get_oauth_token(provider_id).await? {
            if tok.is_expired(120) {
                if tok.refresh_token.is_none() {
                    return Err(DomainError::Validation(
                        "Provider credentials expired; reconnect the account".into(),
                    ));
                }
                let settings =
                    custos_adapters::providers::oauth_pkce::oauth_settings(&tok.service_type)?;
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    &settings.0,
                    &settings.1,
                );
                let refreshed = manager.refresh_token(&settings.2, &tok).await?;
                self.credentials.save_refreshed(&tok, &refreshed).await?;
                return Ok(Some(refreshed.access_token));
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
        if let Some(prov) = repo.get_provider(provider_id)? {
            if let Some(endpoint) = prov.endpoint_url {
                if !endpoint.trim().is_empty() {
                    return Ok(Some(endpoint.trim().to_string()));
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    fn token(oauth: bool) -> custos_domain::OAuthTokenRecord {
        custos_domain::OAuthTokenRecord {
            provider_id: "openai".into(),
            service_type: "openai".into(),
            access_token: if oauth { "opaque-oauth" } else { "eyJ-api-key" }.into(),
            refresh_token: if oauth { Some("refresh".into()) } else { None },
            expires_at: i64::MAX / 2,
            token_type: "bearer".into(),
            scope: None,
            created_at: 1,
            updated_at: 1,
        }
    }
    #[tokio::test]
    async fn credential_metadata_controls_authentication_not_token_prefix() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let resolver = DaemonCredentialResolver {
            store: store.clone(), credentials: CustosRuntime::credential_service(&store),
        };
        resolver.credentials.save_oauth_token(&token(true)).await.unwrap();
        assert!(resolver.uses_oauth("openai").await.unwrap());
        assert_eq!(
            resolver.get_token("openai").await.unwrap().unwrap(),
            "opaque-oauth"
        );
        resolver.credentials.save_oauth_token(&token(false)).await.unwrap();
        assert!(!resolver.uses_oauth("openai").await.unwrap());
        assert_eq!(
            resolver.get_token("openai").await.unwrap().unwrap(),
            "eyJ-api-key"
        );
    }
    #[tokio::test]
    async fn expired_oauth_without_refresh_fails_closed() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let resolver = DaemonCredentialResolver {
            store: store.clone(), credentials: CustosRuntime::credential_service(&store),
        };
        let mut expired = token(false);
        expired.scope = Some("openid".into());
        expired.expires_at = 0;
        resolver.credentials.save_oauth_token(&expired).await.unwrap();
        assert!(resolver.uses_oauth("openai").await.unwrap());
        assert!(resolver
            .get_token("openai")
            .await
            .unwrap_err()
            .to_string()
            .contains("expired"));
    }
    #[tokio::test]
    async fn account_pool_priority_rotation_catalog_binding_and_delete() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(crate::credentials::ProviderCredentials::new(Arc::new(store.providers().clone()), Arc::new(crate::credential_tests::FakeVault::default())));
        let repo=store.providers();
        for id in ["A","B","C"] {
            let config=custos_domain::ProviderConfig { id:id.into(), name:id.into(),service_type:"openai".into(),api_key_masked:"none".into(),status:"active".into(),endpoint_url:Some("https://api.openai.com/v1".into()),default_model:None,context_window:None,fast_mode:None,created_at:0,updated_at:0 };
            service.save_provider(&config,Some(&format!("fixture-{id}"))).await.unwrap();
            let meta=repo.credential_metadata(id).unwrap().unwrap();
            let key=format!("verified-probe:{id}:https://api.openai.com/v1:api_key:{}",meta.created_at);
            let model=custos_domain::ModelCatalogOption{id:if id=="C" {"gpt-other"} else {"gpt-test"}.into(),label:id.into(),provider_type:"openai".into(),description:None,is_default:false,default_effort:None,efforts:vec![],supports_fast_mode:false,context_window:None,pricing:None};
            repo.save_catalog_models(&key,&[model]).unwrap();
        }
        assert_eq!(reserve_provider_account(&repo,"openai","gpt-test").unwrap().as_deref(),Some("A"));
        repo.save_routing_policy("openai",false,&["B".into(),"A".into(),"C".into()]).unwrap();
        for _ in 0..3 { assert_eq!(reserve_provider_account(&repo,"openai","gpt-test").unwrap().as_deref(),Some("B")); }
        repo.save_routing_policy("openai",true,&["B".into(),"A".into(),"C".into()]).unwrap();
        for expected in ["B","A","B","A"] { assert_eq!(reserve_provider_account(&repo,"openai","gpt-test").unwrap().as_deref(),Some(expected)); }
        assert!(repo.save_routing_policy("openai",true,&["A".into(),"A".into()]).is_err());
        assert!(repo.save_routing_policy("copilot",true,&["A".into()]).is_err());
        assert!(reserve_provider_account(&repo,"openai","missing").is_err());
        service.delete("B",true).await.unwrap();
        assert_eq!(reserve_provider_account(&repo,"openai","gpt-test").unwrap().as_deref(),Some("A"));
        assert_eq!(service.get_oauth_token("A").await.unwrap().unwrap().access_token,"fixture-A");
        let mut config=repo.get_provider("A").unwrap().unwrap();
        config.endpoint_url=Some("https://different.invalid/v1".into());
        repo.save_provider(&config).unwrap();
        assert!(reserve_provider_account(&repo,"openai","gpt-test").is_err());
    }

    #[tokio::test]
    async fn account_pool_concurrent_reservations_are_balanced_and_locked_selection_fails_closed() {
        let store=Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let vault=Arc::new(crate::credential_tests::FakeVault::default());
        let service=Arc::new(crate::credentials::ProviderCredentials::new(Arc::new(store.providers().clone()),vault.clone()));
        let repo=store.providers();
        for id in ["A","B"] {
            let config=custos_domain::ProviderConfig{id:id.into(),name:id.into(),service_type:"copilot".into(),api_key_masked:"none".into(),status:"active".into(),endpoint_url:Some("https://api.githubcopilot.com".into()),default_model:None,context_window:None,fast_mode:None,created_at:0,updated_at:0};
            let mut tok=token(true); tok.provider_id=id.into();tok.service_type="copilot".into();
            service.save_oauth_connection(&tok,Some(&config)).await.unwrap();
            let key=format!("verified-probe:{id}:https://api.githubcopilot.com:oauth:{}",tok.created_at);
            let model=custos_domain::ModelCatalogOption{id:"copilot/claude-test".into(),label:"Claude".into(),provider_type:"copilot".into(),description:None,is_default:false,default_effort:None,efforts:vec![],supports_fast_mode:false,context_window:None,pricing:None};
            repo.save_catalog_models(&key,&[model]).unwrap();
        }
        repo.save_routing_policy("copilot",true,&["A".into(),"B".into()]).unwrap();
        let mut handles=vec![];
        for _ in 0..20 { let repo=(*repo).clone();handles.push(tokio::task::spawn_blocking(move||reserve_provider_account(&repo,"copilot","copilot/claude-test").unwrap().unwrap())); }
        let mut a=0; for handle in handles { if handle.await.unwrap()=="A" {a+=1;} } assert_eq!(a,10);
        vault.fail_read.store(true,std::sync::atomic::Ordering::SeqCst);
        let resolver=DaemonCredentialResolver{store,credentials:service};
        let selected=resolver.select_account("copilot","copilot/claude-test").await.unwrap().unwrap();
        assert!(resolver.get_token(&selected).await.is_err());
        assert!(resolver.validate_account(&selected,"anthropic").await.is_err());
    }

}
