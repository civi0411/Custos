//! Workbenches, Packs, and Governed Harness Setup (SADE R8 §6.8)

use std::path::Path;
use std::sync::Arc;

use custos_adapters::harness::{ClaudeCodeHarnessAdapter, HarnessRegistry};
use custos_adapters::mcp::McpFederationHub;
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::TrustedKernel;
use custos_packs::DomainPackRegistry;
use custos_persistence::SqliteTaskStore;
use custos_provider::ModelPort;

use crate::tool_executor::DaemonGovernedToolExecutor;

pub struct WorkbenchHarnessSetup {
    pub pack_registry: Arc<DomainPackRegistry>,
    pub mcp_hub: Arc<McpFederationHub>,
    pub harnesses: Arc<HarnessRegistry>,
    pub harness: Arc<dyn AgentRuntimePort>,
}

pub fn init_workbenches_and_harness(
    store: &Arc<SqliteTaskStore>,
    workspace_root: &Path,
    kernel: Arc<TrustedKernel>,
    model: Arc<dyn ModelPort>,
    sandbox: Arc<SovereignDeveloperAdapter>,
    supervision: Option<Arc<dyn custos_core::contracts::supervision::SupervisionTraceSink>>,
) -> WorkbenchHarnessSetup {
    let mut harness_reg = HarnessRegistry::default_with_workspace(workspace_root.to_path_buf());

    // Auto-probe pre-existing local packages from disk (~/.custos/packages) into inventory
    let packages_dir = dirs::home_dir()
        .map(|h| h.join(".custos").join("packages"))
        .unwrap_or_else(|| std::path::PathBuf::from(".custos/packages"));
    if let Ok(discovered) = custos_adapters::host::packages::inventory::scan_local_packages(&packages_dir) {
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

    // Register only workbench modules actually installed/builtin in package inventory
    let installed_inventory = store.packages().list_inventory().unwrap_or_default();
    if installed_inventory.is_empty() {
        crate::api::LocalApiDispatcher::ensure_module_registered("assistant");
    } else {
        for item in &installed_inventory {
            if item.state == "builtin" || item.state == "installed" || item.state == "disabled" {
                crate::api::LocalApiDispatcher::ensure_module_registered(&item.component_id);
            }
        }
    }

    let enabled_components = match store.packages().list_enabled_components() {
        Ok(comps) => comps,
        Err(e) => {
            tracing::error!(
                "Failed to query enabled packages from store: {e}; falling closed to safe built-in 'assistant' only"
            );
            vec!["assistant".into()]
        }
    };
    let enabled_refs: Vec<&str> = enabled_components.iter().map(String::as_str).collect();
    let pack_registry = Arc::new(custos_packs::create_pack_registry_from_inventory(
        &enabled_refs,
    ));

    let mcp_hub = Arc::new(McpFederationHub::new());

    // Dynamic Tool Disclosure & Governed Tool Authorization (R4 / F4)
    let default_tools = custos_packs::compile_governed_tools(
        custos_domain::WorkbenchLens::Assistant,
        &pack_registry,
    );

    let mut exec_builder = DaemonGovernedToolExecutor::new(sandbox, kernel, store.outbox().clone())
        .with_mcp_hub(mcp_hub.clone())
        .with_pack_registry(pack_registry.clone());
    if let Some(ref sink) = supervision {
        exec_builder = exec_builder.with_supervision(sink.clone());
    }
    let tool_executor = Arc::new(exec_builder);
    mcp_hub.set_gate_sync(tool_executor.clone());

    let governed = Arc::new(
        custos_runtime::agent::runtime_port::GovernedAgentRuntime::new(
            model,
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

    // Gate 4 & Zero-Scatter Supervision: Wrap all registered CLI agents & internal runtimes
    // with SupervisedAgentRunner to guarantee telemetry spans, logging, and watchdog coverage.
    if let Some(ref sink) = supervision {
        for (id, runtime) in harness_reg.runtimes() {
            let supervised = Arc::new(custos_runtime::agent::SupervisedAgentRunner::new(
                runtime,
                sink.clone(),
            ));
            harness_reg.replace_runtime(&id, supervised);
        }
    }

    let harnesses = Arc::new(harness_reg);
    let harness = harnesses
        .get("claude-code")
        .unwrap_or_else(|| {
            let fallback: Arc<dyn AgentRuntimePort> =
                Arc::new(ClaudeCodeHarnessAdapter::new(workspace_root.to_path_buf()));
            if let Some(ref sink) = supervision {
                Arc::new(custos_runtime::agent::SupervisedAgentRunner::new(
                    fallback,
                    sink.clone(),
                ))
            } else {
                fallback
            }
        });

    WorkbenchHarnessSetup {
        pack_registry,
        mcp_hub,
        harnesses,
        harness,
    }
}
