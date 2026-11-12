//! Builder and dependency injection methods for LocalApiDispatcher.

use super::*;
use std::sync::Arc;

impl LocalApiDispatcher {
    pub fn with_environment_coordinator(
        mut self,
        environment_coordinator: Arc<custos_runtime::environment::EnvironmentCoordinator>,
    ) -> Self {
        self.env.environment_coordinator = Some(environment_coordinator);
        self
    }

    pub fn with_admission_coordinator(
        mut self,
        admission_coordinator: Arc<custos_runtime::compute::WorkloadAdmissionCoordinator>,
    ) -> Self {
        self.env.admission_coordinator = Some(admission_coordinator);
        self
    }

    pub fn with_mcp_hub(mut self, hub: Arc<custos_adapters::mcp::McpFederationHub>) -> Self {
        self.workbenches.mcp_hub = hub;
        self
    }

    pub fn mcp_hub(&self) -> Arc<custos_adapters::mcp::McpFederationHub> {
        self.workbenches.mcp_hub.clone()
    }

    pub fn with_model_availability(
        mut self,
        availability: custos_provider::routing::availability::ModelAvailability,
    ) -> Self {
        self.providers.model_availability = availability;
        self
    }

    pub fn with_sqlite_store(mut self, store: Arc<custos_persistence::SqliteTaskStore>) -> Self {
        if self.workbenches.shared_capabilities.is_none() {
            self.workbenches.shared_capabilities = Some(Arc::new(store.shared_capabilities().clone()));
        }
        if self.workbenches.browser.is_none() {
            self.workbenches.browser = Some(Arc::new(store.browser().clone()));
        }
        let enabled = store.packages().list_enabled_components().unwrap_or_default();
        self.kernel.sqlite_store = Some(store);
        for component_id in enabled {
            self.activate_workbench_resources(&component_id);
        }
        self
    }

    pub fn with_shared_capabilities(
        mut self,
        repo: Arc<custos_persistence::SharedCapabilitiesRepository>,
    ) -> Self {
        self.workbenches.shared_capabilities = Some(repo);
        self
    }

    #[cfg(feature = "custos-workbench-quant")]
    pub fn with_quant(self, quant: Arc<dyn custos_workbench_quant::QuantPort>) -> Self {
        if let Ok(mut guard) = self.workbenches.quant.write() {
            *guard = Some(quant);
        }
        self
    }

    #[cfg(feature = "custos-workbench-quant")]
    pub fn quant_port(&self) -> Option<Arc<dyn custos_workbench_quant::QuantPort>> {
        self.workbenches.quant.read().ok().and_then(|g| g.clone())
    }

    #[cfg(feature = "custos-workbench-research")]
    pub(crate) fn research_repo(&self) -> Option<Arc<ResearchRepository>> {
        self.workbenches.research.read().ok().and_then(|g| g.clone())
    }

    #[cfg(feature = "custos-workbench-assistant")]
    pub(crate) fn assistant_repo(&self) -> Option<Arc<AssistantRepository>> {
        self.workbenches.assistant.read().ok().and_then(|g| g.clone())
    }

    pub(crate) fn activate_workbench_resources(&self, component_id: &str) {
        let Some(store) = self.kernel.sqlite_store.as_ref() else {
            return;
        };
        let _ = store;
        match component_id {
            #[cfg(feature = "custos-workbench-quant")]
            "quant" => {
                if let Ok(mut guard) = self.workbenches.quant.write() {
                    if guard.is_none() {
                        *guard = Some(Arc::new(custos_workbench_quant::QuantRepository::new(
                            store.db().writer_arc(),
                        )));
                    }
                }
            }
            #[cfg(feature = "custos-workbench-research")]
            "research" => {
                if let Ok(mut guard) = self.workbenches.research.write() {
                    if guard.is_none() {
                        *guard = Some(Arc::new(custos_workbench_research::ResearchRepository::new(
                            custos_workbench_research::repository::DbConnection::new(
                                store.db().writer_arc(),
                            ),
                        )));
                    }
                }
            }
            #[cfg(feature = "custos-workbench-assistant")]
            "assistant" | "copilot" => {
                if let Ok(mut guard) = self.workbenches.assistant.write() {
                    if guard.is_none() {
                        *guard = Some(Arc::new(
                            custos_workbench_assistant::AssistantRepository::from_raw(
                                store.db().writer_arc(),
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
        self.sync_supervision_profiles();
    }

    pub(crate) async fn release_workbench_resources(&self, component_id: &str) {
        match component_id {
            #[cfg(feature = "custos-workbench-quant")]
            "quant" => {
                if let Ok(mut guard) = self.workbenches.quant.write() {
                    *guard = None;
                }
            }
            #[cfg(feature = "custos-workbench-research")]
            "research" => {
                if let Ok(mut guard) = self.workbenches.research.write() {
                    *guard = None;
                }
            }
            #[cfg(feature = "custos-workbench-assistant")]
            "assistant" | "copilot" => {
                if let Ok(mut guard) = self.workbenches.assistant.write() {
                    *guard = None;
                }
            }
            _ => {}
        }
        let still_needs_notebook = self
            .workbenches
            .pack_registry
            .list_packs()
            .iter()
            .any(|p| p.required_capabilities.iter().any(|c| c == "notebook"));
        if !still_needs_notebook {
            if let Some(kernel) = self.env.python_kernel.as_ref() {
                kernel.shutdown_all().await;
            }
        }
        self.kernel
            .supervision
            .profiles()
            .unregister_profile(component_id);
        self.sync_supervision_profiles();
    }

    pub fn with_supervision(
        mut self,
        supervision: Arc<custos_runtime::workflow::SupervisionCoordinator>,
    ) -> Self {
        self.kernel.supervision = supervision;
        self.sync_supervision_profiles();
        self
    }

    pub(crate) fn sync_supervision_profiles(&self) {
        for (wb_id, profile) in self.workbenches.pack_registry.list_supervision_profiles() {
            self.kernel
                .supervision
                .profiles()
                .register_profile(&wb_id, profile);
        }
    }

    pub fn has_shared_capabilities(&self) -> bool {
        self.workbenches.shared_capabilities.is_some()
    }

    pub fn has_research(&self) -> bool {
        #[cfg(feature = "custos-workbench-research")]
        {
            self.research_repo().is_some() && self.workbenches.pack_registry.has_pack("research")
        }
        #[cfg(not(feature = "custos-workbench-research"))]
        {
            false
        }
    }

    pub fn has_quant(&self) -> bool {
        #[cfg(feature = "custos-workbench-quant")]
        {
            self.quant_port().is_some()
        }
        #[cfg(not(feature = "custos-workbench-quant"))]
        {
            false
        }
    }

    pub fn has_assistant(&self) -> bool {
        #[cfg(feature = "custos-workbench-assistant")]
        {
            self.assistant_repo().is_some()
        }
        #[cfg(not(feature = "custos-workbench-assistant"))]
        {
            false
        }
    }

    pub fn with_python_kernel(mut self, python_kernel: Arc<PythonKernelCoordinator>) -> Self {
        self.env.python_kernel = Some(python_kernel);
        self
    }

    pub fn with_harnesses(mut self, harnesses: Arc<HarnessRegistry>) -> Self {
        self.env.harnesses = harnesses;
        self
    }

    pub fn with_files(mut self, files: Arc<dyn WorkspaceFilesPort>) -> Self {
        self.env.files = files;
        self
    }

    pub fn with_terminal(mut self, terminal: Arc<TerminalCoordinator>) -> Self {
        self.env.terminal = terminal;
        self
    }

    pub fn with_usage_runs(
        mut self,
        runs: Arc<dyn custos_core::contracts::storage::RunPort>,
    ) -> Self {
        self.kernel.usage_runs = Some(runs);
        self
    }

    pub fn with_workflow(mut self, workflow: Arc<dyn WorkflowPort>) -> Self {
        self.kernel.workflow = Some(workflow);
        self
    }

    pub fn with_workspace(mut self, workspace: Arc<WorkspaceCoordinator>) -> Self {
        self.env.workspace = Some(workspace);
        self
    }

    #[cfg(feature = "custos-workbench-research")]
    pub fn with_research(self, research: Arc<ResearchRepository>) -> Self {
        if let Ok(mut guard) = self.workbenches.research.write() {
            *guard = Some(research);
        }
        self
    }

    #[cfg(feature = "custos-workbench-assistant")]
    pub fn with_assistant(self, assistant: Arc<AssistantRepository>) -> Self {
        if let Ok(mut guard) = self.workbenches.assistant.write() {
            *guard = Some(assistant);
        }
        self
    }

    pub fn with_outbox(mut self, outbox: Arc<OutboxRepository>) -> Self {
        self.workbenches.outbox = Some(outbox);
        self
    }

    pub(crate) fn outbox_repo(&self) -> Option<&OutboxRepository> {
        self.workbenches.outbox
            .as_deref()
            .or_else(|| self.kernel.sqlite_store.as_ref().map(|s| s.outbox()))
    }

    pub fn with_providers(mut self, providers: Arc<ProviderRepository>) -> Self {
        #[cfg(test)]
        {
            self.providers.credentials = Some(crate::credential_tests::fake_credentials(providers.clone()));
        }
        self.providers.repo = Some(providers);
        self
    }

    pub fn with_credentials(
        mut self,
        credentials: Arc<crate::credentials::ProviderCredentials>,
    ) -> Self {
        self.providers.credentials = Some(credentials);
        self
    }

    pub fn with_pack_registry(
        mut self,
        pack_registry: Arc<custos_packs::DomainPackRegistry>,
    ) -> Self {
        self.workbenches.pack_registry = pack_registry;
        self
    }

    pub fn with_browser(mut self, browser: Arc<BrowserRepository>) -> Self {
        self.workbenches.browser = Some(browser);
        self
    }

    pub fn with_fleet_automation(
        mut self,
        fleet_automation: Arc<FleetAutomationRepository>,
    ) -> Self {
        self.workbenches.fleet_automation = Some(fleet_automation);
        self
    }

    pub fn with_oauth_callback_server(
        mut self,
        oauth_callback_server: Arc<crate::oauth_callback_server::OAuthCallbackServer>,
    ) -> Self {
        self.providers.oauth_callback_server = Some(oauth_callback_server);
        self
    }

    #[deprecated(note = "use with_pack_registry instead (A02 deduplication)")]
    pub fn with_packs(self, pack_registry: Arc<custos_packs::DomainPackRegistry>) -> Self {
        self.with_pack_registry(pack_registry)
    }
}

