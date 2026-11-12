use super::super::*;
use custos_domain::session::WorkbenchLens;
use custos_core::CreateTask;
use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use std::sync::Arc;

async fn setup_assistant_only_env() -> (LocalApiDispatcher, String) {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    store
        .packages()
        .commit_installed("assistant", "0.1.0", "any", None, "packages/assistant/0.1.0")
        .unwrap();
    let service = Arc::new(TaskService::new(store.clone()));
    let task = service
        .execute_create(CreateTask {
            title: "Assistant Only Baseline Task".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;

    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let outbox = Arc::new(store.outbox().clone());

    LocalApiDispatcher::ensure_default_modules_registered();
    let pack_registry = Arc::new(custos_packs::create_pack_registry_from_inventory(&["assistant"]));
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store)
        .with_outbox(outbox)
        .with_pack_registry(pack_registry);

    (dispatcher, task.id)
}

#[cfg(feature = "custos-workbench-assistant")]
#[tokio::test]
async fn test_assistant_only_pack_registry_isolation() {
    let (dispatcher, _) = setup_assistant_only_env().await;

    assert!(dispatcher.workbenches.pack_registry.has_pack("assistant"));
    assert!(!dispatcher.workbenches.pack_registry.has_pack("quant"));
    assert!(!dispatcher.workbenches.pack_registry.has_pack("research"));
    assert!(!dispatcher.workbenches.pack_registry.has_pack("engineering"));
    assert!(!dispatcher.has_quant());
    assert!(!dispatcher.has_research());

    let packs = dispatcher.workbenches.pack_registry.list_packs();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].id, "assistant");
}

#[tokio::test]
async fn test_assistant_only_blocks_quant_mutation() {
    let (dispatcher, task_id) = setup_assistant_only_env().await;

    let req = ApiRequest {
        id: "quant_backtest".into(),
        method: METHOD_QUANT_BACKTEST_SUBMIT.into(),
        params: serde_json::json!({
            "strategy_id": "strat_alpha_0",
            "task_id": task_id,
            "formula": "close / delay(close, 1)",
            "annualized_sharpe": 1.5,
            "max_drawdown": 0.1,
            "total_return": 0.2,
            "trials_count": 10,
            "sample_length": 100,
            "volatility": 0.15,
            "win_rate": 0.55,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(!resp.is_success());
    let err = resp.error.unwrap();
    assert!(
        err.contains("pack_disabled") || err.contains("not installed"),
        "Expected pack_disabled rejection, got: {}",
        err
    );
}

#[tokio::test]
async fn test_assistant_only_blocks_research_mutation() {
    let (dispatcher, task_id) = setup_assistant_only_env().await;

    let req = ApiRequest {
        id: "research_source_save".into(),
        method: METHOD_RESEARCH_SOURCES_SAVE.into(),
        params: serde_json::json!({
            "task_id": task_id,
            "title": "Uninstalled Research Source",
            "authors": ["Test Author"],
            "url": "https://example.com/paper.pdf",
            "abstract_text": "Sample abstract",
            "source_type": "paper",
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(!resp.is_success());
    let err = resp.error.unwrap();
    assert!(
        err.contains("pack_disabled") || err.contains("not installed"),
        "Expected pack_disabled rejection, got: {}",
        err
    );
}

#[tokio::test]
async fn test_assistant_only_tools_disclosure_isolation() {
    let (dispatcher, _) = setup_assistant_only_env().await;

    // Lens Quant with uninstalled quant pack -> should NOT disclose any quant domain tools
    let quant_tools = custos_packs::compile_governed_tools(WorkbenchLens::Quant, &dispatcher.workbenches.pack_registry);
    assert_eq!(quant_tools.len(), 2, "Only read_file and list_dir should be disclosed");

    let quant_tool_names: Vec<&str> = quant_tools
        .iter()
        .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
        .collect();
    assert_eq!(quant_tool_names, vec!["read_file", "list_dir"]);

    // Lens Assistant with installed assistant pack -> should disclose assistant domain tools if any
    let assistant_tools = custos_packs::compile_governed_tools(WorkbenchLens::Assistant, &dispatcher.workbenches.pack_registry);
    assert!(assistant_tools.len() >= 2);
}

#[tokio::test]
async fn test_assistant_only_rejects_quant_read_queries() {
    let (dispatcher, _) = setup_assistant_only_env().await;

    // A read-only request to Quant Tearsheet
    let req = ApiRequest {
        id: "quant_tearsheet_get".into(),
        method: METHOD_QUANT_TEARSHEET_GET.into(),
        params: serde_json::json!({
            "strategy_id": "nonexistent_strat"
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    // Under strict Zero-Mock & Target Architecture:
    // If Quant workbench is not installed, ALL quant endpoints (even read-only) MUST return pack_disabled.
    assert!(!resp.is_success(), "Uninstalled workbench must reject read queries as well");
    let err = resp.error.unwrap();
    assert!(
        err.contains("pack_disabled") || err.contains("not installed"),
        "Expected pack_disabled rejection for uninstalled workbench query, but got: {}",
        err
    );
}

#[cfg(feature = "custos-workbench-assistant")]
#[tokio::test]
async fn test_assistant_only_blocks_uninstalled_workbench_tool_authorization() {
    LocalApiDispatcher::ensure_default_modules_registered();
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let kernel: Arc<dyn custos_core::contracts::kernel::KernelPort> =
        Arc::new(custos_core::contracts::kernel::TrustedKernel::new(store.clone()));
    let pack_registry =
        Arc::new(custos_packs::create_pack_registry_from_inventory(&["assistant"]));
    let temp_workspace = tempfile::tempdir().unwrap();
    let developer_adapter =
        Arc::new(custos_adapters::sandbox::SovereignDeveloperAdapter::new(temp_workspace.path()));

    let executor = crate::tool_executor::DaemonGovernedToolExecutor::new(
        developer_adapter,
        kernel,
        store.outbox().clone(),
    )
    .with_pack_registry(pack_registry);

    use custos_runtime::agent::runtime_port::{GovernedToolExecutor, InvocationContext};

    let ctx = InvocationContext {
        task_id: "test_task".into(),
        run_id: "test_run".into(),
        actor: "test_worker".into(),
        scope: None,
    };

    // 1. Uninstalled quant tool -> Rejected!
    let quant_res = executor
        .authorize(
            &ctx,
            "run_backtest",
            &serde_json::json!({ "strategy_id": "alpha_1" }),
        )
        .await;
    assert!(quant_res.is_err());
    match quant_res.unwrap_err() {
        custos_domain::DomainError::Unauthorized(msg) => {
            assert!(
                msg.contains("not installed or enabled"),
                "Expected uninstalled/disabled message, got: {msg}"
            );
        }
        other => panic!("Expected DomainError::Unauthorized, got {other:?}"),
    }

    // 2. Uninstalled research tool -> Rejected!
    let research_res = executor
        .authorize(
            &ctx,
            "search_literature",
            &serde_json::json!({ "query": "attention is all you need" }),
        )
        .await;
    assert!(research_res.is_err());
    match research_res.unwrap_err() {
        custos_domain::DomainError::Unauthorized(msg) => {
            assert!(
                msg.contains("not installed or enabled"),
                "Expected uninstalled/disabled message, got: {msg}"
            );
        }
        other => panic!("Expected DomainError::Unauthorized, got {other:?}"),
    }

    // 3. Installed assistant tool -> Accepted!
    let assistant_res = executor
        .authorize(
            &ctx,
            "schedule_task",
            &serde_json::json!({ "title": "Check system metrics" }),
        )
        .await;
    assert!(assistant_res.is_ok());

    // 4. Safe base tool -> Accepted!
    let safe_res = executor
        .authorize(
            &ctx,
            "read_file",
            &serde_json::json!({ "path": "notes.txt" }),
        )
        .await;
    assert!(safe_res.is_ok());
}

#[cfg(all(
    feature = "custos-workbench-assistant",
    feature = "custos-workbench-quant",
    feature = "custos-workbench-research"
))]
#[tokio::test]
async fn test_capability_validation_blocks_enabling_pack_without_required_capability() {
    let (mut dispatcher, _) = setup_assistant_only_env().await;
    // In assistant-only profile, no python kernel coordinator is configured
    dispatcher.env.python_kernel = None;

    // 1. Enabling quant without notebook capability -> Rejected cleanly!
    let quant_enable_req = ApiRequest {
        id: "enable_quant".into(),
        method: "packages.enable".into(),
        params: serde_json::json!({ "component_id": "quant" }),
    };
    let resp = dispatcher.handle_request(quant_enable_req).await;
    assert!(!resp.is_success());
    let err = resp.error.unwrap();
    assert!(
        err.contains("missing required capability 'notebook'"),
        "Expected missing notebook capability error, got: {err}"
    );

    // 2. Enabling research without notebook capability -> Rejected cleanly!
    let research_enable_req = ApiRequest {
        id: "enable_research".into(),
        method: "packages.enable".into(),
        params: serde_json::json!({ "component_id": "research" }),
    };
    let resp = dispatcher.handle_request(research_enable_req).await;
    assert!(!resp.is_success());
    let err = resp.error.unwrap();
    assert!(
        err.contains("missing required capability 'notebook'"),
        "Expected missing notebook capability error, got: {err}"
    );

    // 3. Enabling assistant (requires no capabilities) -> Succeeds!
    let assistant_enable_req = ApiRequest {
        id: "enable_assistant".into(),
        method: "packages.enable".into(),
        params: serde_json::json!({ "component_id": "assistant" }),
    };
    let resp = dispatcher.handle_request(assistant_enable_req).await;
    assert!(resp.is_success(), "Failed to enable assistant: {:?}", resp.error);

    // 4. If python kernel capability is attached, enabling quant succeeds!
    dispatcher.env.python_kernel = Some(Arc::new(custos_runtime::PythonKernelCoordinator::new(
        Arc::new(custos_adapters::notebook::LocalPythonDriver),
    )));
    let quant_enable_req2 = ApiRequest {
        id: "enable_quant_2".into(),
        method: "packages.enable".into(),
        params: serde_json::json!({ "component_id": "quant" }),
    };
    let resp = dispatcher.handle_request(quant_enable_req2).await;
    assert!(resp.is_success(), "Expected successful enablement once capability is present: {:?}", resp.error);
}

#[tokio::test]
async fn test_ring2_shared_capabilities_decoupled_from_research_pack() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let pack_registry = Arc::new(custos_packs::create_pack_registry_from_inventory(&["assistant"]));

    let mut dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_sqlite_store(store)
        .with_pack_registry(pack_registry);
    dispatcher.env.python_kernel = Some(Arc::new(custos_runtime::PythonKernelCoordinator::new(
        Arc::new(custos_adapters::notebook::LocalPythonDriver),
    )));

    // Ring 2 shared capabilities (notes, reviews, artifacts, notebook) must succeed without 'research' pack.
    for (id, method, params) in [
        ("r2_notes", METHOD_NOTES_LIST, serde_json::json!({})),
        ("r2_reviews", METHOD_REVIEWS_LIST, serde_json::json!({})),
        ("r2_artifacts", METHOD_ARTIFACTS_LIST, serde_json::json!({})),
        ("r2_nb", METHOD_NOTEBOOK_STATUS, serde_json::json!({ "session_id": "sess_shared" })),
    ] {
        let resp = dispatcher
            .handle_request(ApiRequest {
                id: id.into(),
                method: method.into(),
                params,
            })
            .await;
        assert!(resp.is_success(), "Expected shared capability '{method}' to succeed without research pack: {:?}", resp.error);
    }

    // Research-specific pipeline & synthesis remain pack-gated.
    for (id, method) in [
        ("r3_src", METHOD_RESEARCH_SOURCES_LIST),
        ("r3_syn", METHOD_SYNTHESIS_PROPOSALS_LIST),
    ] {
        let resp = dispatcher
            .handle_request(ApiRequest {
                id: id.into(),
                method: method.into(),
                params: serde_json::json!({}),
            })
            .await;
        assert!(!resp.is_success());
        assert!(resp.error.as_deref().unwrap_or("").contains("pack_disabled"));
    }

    let caps: Vec<custos_domain::CapabilityDescriptor> = serde_json::from_value(
        dispatcher
            .handle_request(ApiRequest {
                id: "caps".into(),
                method: METHOD_CAPABILITIES_LIST.into(),
                params: serde_json::json!({}),
            })
            .await
            .result
            .unwrap(),
    )
    .unwrap();
    assert!(caps.iter().find(|c| c.id == "personal.notes").unwrap().status.is_available());
    assert!(caps.iter().find(|c| c.id == "evidence.criteria").unwrap().status.is_available());
    assert!(!caps.iter().find(|c| c.id == "research.sources").unwrap().status.is_available());
    assert!(!caps.iter().find(|c| c.id == "synthesis.handoff").unwrap().status.is_available());
}


