use super::super::*;
use super::harness::*;
use std::sync::{Arc, Mutex};
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
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

