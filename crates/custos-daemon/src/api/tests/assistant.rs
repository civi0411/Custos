use super::super::*;
use std::sync::Arc;
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
    #[tokio::test]
    async fn outbox_preserves_failure_uncertainty_and_task_scope() {
        use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let outbox = Arc::new(store.outbox().clone());
        let service = Arc::new(TaskService::new(store));
        let task = service.execute_create(CreateTask { title: "Outbox inspection".into(), metadata: None, contract: None }).await.unwrap().0;
        for (index, status) in [OutboxStatus::Failed, OutboxStatus::Uncertain].into_iter().enumerate() {
            outbox.enqueue(OutboxEntry::new(
                format!("entry-{index}"),
                task.id.clone(),
                "send_message",
                "permit",
                "sha256:payload",
                None,
                status,
                chrono::Utc::now() + chrono::Duration::seconds(index as i64),
            )).await.unwrap();
        }
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_outbox(outbox);
        let request = |method: &str, params| ApiRequest { id: "outbox-inspect".into(), method: method.into(), params };
        let response = dispatcher.handle_request(request(METHOD_ASSISTANT_OUTBOX_LIST, serde_json::json!({"task_id": task.id}))).await;
        assert!(response.is_success(), "{:?}", response.error);
        let items = response.result.unwrap();
        assert_eq!(items.as_array().unwrap().len(), 2);
        assert_eq!(items[0]["status"], "uncertain");
        assert_eq!(items[1]["status"], "failed");
        let empty = dispatcher.handle_request(request(METHOD_ASSISTANT_OUTBOX_LIST, serde_json::json!({"task_id": "other-task"}))).await;
        assert_eq!(empty.result.unwrap(), serde_json::json!([]));
        let capabilities = dispatcher.handle_request(request(METHOD_CAPABILITIES_LIST, serde_json::json!({}))).await.result.unwrap();
        let capability = capabilities.as_array().unwrap().iter().find(|item| item["resource_id"] == "outbox").unwrap();
        assert_eq!(capability["supported_operations"], serde_json::json!(["list", "create", "approve", "dispatch"]));
        assert!(!dispatcher.handle_request(request(METHOD_ASSISTANT_OUTBOX_DISPATCH, serde_json::json!({"id": "entry-1"}))).await.is_success());
    }

    #[tokio::test]
    async fn outbox_create_approve_and_dispatch_lifecycle() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let outbox = Arc::new(store.outbox().clone());
        let service = Arc::new(TaskService::new(store));
        let task = service
            .execute_create(CreateTask {
                title: "Outbox Full Pipeline".into(),
                metadata: None,
                contract: None,
            })
            .await
            .unwrap()
            .0;
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_outbox(outbox);
        let client = Arc::new(custos_adapters::mcp::CustosMcpClient::new("notifier"));
        dispatcher.mcp_hub().register_server("notifications", client).await.unwrap();
        let request = |method: &str, params| ApiRequest {
            id: "outbox-test".into(),
            method: method.into(),
            params,
        };

        // 1. Create Outbox entry
        let create_resp = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_CREATE,
                serde_json::json!({
                    "task_id": task.id,
                    "action_type": "email_dispatch",
                    "recipient": "mcp://notifications/email_dispatch",
                    "payload": { "subject": "Artifact Ready", "body": "Experiment completed successfully." }
                }),
            ))
            .await;
        assert!(create_resp.is_success(), "{:?}", create_resp.error);
        let created_item = create_resp.result.unwrap();
        let entry_id = created_item["id"].as_str().unwrap().to_string();
        assert_eq!(created_item["status"], "pending_approval");
        assert_eq!(created_item["recipient"], "mcp://notifications/email_dispatch");

        let denied = dispatcher.handle_request(request(
            METHOD_ASSISTANT_OUTBOX_DISPATCH,
            serde_json::json!({"id":entry_id}),
        )).await;
        assert!(denied.error.unwrap().contains("requires durable approval"));

        // 2. Approve Outbox entry
        let approve_resp = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_APPROVE,
                serde_json::json!({ "id": entry_id }),
            ))
            .await;
        assert!(approve_resp.is_success(), "{:?}", approve_resp.error);
        assert_eq!(approve_resp.result.unwrap()["status"], "approved");

        // 3. Dispatch Outbox entry
        let dispatch_resp = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_DISPATCH,
                serde_json::json!({ "id": entry_id }),
            ))
            .await;
        assert!(dispatch_resp.is_success(), "{:?}", dispatch_resp.error);
        let dispatch_res = dispatch_resp.result.unwrap();
        assert_eq!(dispatch_res["status"], "receipted");
        let receipt = &dispatch_res["receipt"];
        assert_eq!(receipt["status"], "success");
        assert!(receipt["output_digest"].as_str().unwrap().starts_with("sha256:"));

        // 4. Double dispatch must fail CAS fencing
        let double_dispatch = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_DISPATCH,
                serde_json::json!({ "id": entry_id }),
            ))
            .await;
        assert!(!double_dispatch.is_success());
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
    async fn test_copilot_connector_capability_exact_approval_and_outbox_rejections() {
        use custos_core::contracts::storage::{OutboxPort, OutboxStatus};
        use custos_domain::OAuthTokenRecord;

        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let providers_repo = store.providers();

        // 1. K1: Connector capability from OAuth scopes
        let token_with_email = OAuthTokenRecord {
            provider_id: "google-copilot".into(),
            service_type: "google".into(),
            access_token: "ya29.mock_token".into(),
            refresh_token: Some("1//refresh".into()),
            expires_at: chrono::Utc::now().timestamp_millis() + 3_600_000,
            token_type: "Bearer".into(),
            scope: Some("email.send https://www.googleapis.com/auth/userinfo.profile".into()),
            created_at: 100,
            updated_at: 100,
        };
        providers_repo.save_oauth_token(&token_with_email).unwrap();

        assert!(providers_repo.has_oauth_scope("google-copilot", "email.send").unwrap());
        assert!(!providers_repo.has_oauth_scope("google-copilot", "calendar.events").unwrap());

        let caps = providers_repo.get_connector_capabilities("google-copilot").unwrap();
        let email_cap = caps.iter().find(|c| c.id == "connector.email").unwrap();
        assert_eq!(email_cap.status, "available");
        let cal_cap = caps.iter().find(|c| c.id == "connector.calendar").unwrap();
        assert_eq!(cal_cap.status, "unavailable");
        assert!(cal_cap.reason.as_ref().unwrap().contains("Missing 'calendar.events'"));

        let unconfigured_caps = providers_repo.get_connector_capabilities("non-existent-provider").unwrap();
        assert!(unconfigured_caps.iter().all(|c| c.status == "unavailable"));

        // Setup daemon dispatcher with outbox
        let outbox = Arc::new(store.outbox().clone());
        let service = Arc::new(TaskService::new(store.clone()));
        let task = service
            .execute_create(CreateTask {
                title: "Copilot Test Task".into(),
                metadata: None,
                contract: None,
            })
            .await
            .unwrap()
            .0;
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_outbox(outbox.clone());

        let request = |method: &str, params| ApiRequest {
            id: "copilot-test".into(),
            method: method.into(),
            params,
        };

        // 2. K2: Exact approval with digest check & rejection of mismatched expected_digest
        let create_resp = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_CREATE,
                serde_json::json!({
                    "task_id": task.id,
                    "action_type": "email_dispatch",
                    "recipient": "http://127.0.0.1:9/send",
                    "payload": { "subject": "Hello", "body": "World" }
                }),
            ))
            .await;
        assert!(create_resp.is_success());
        let item = create_resp.result.unwrap();
        let entry_id = item["id"].as_str().unwrap();
        let valid_digest = item["payload_digest"].as_str().unwrap();

        // 2a. Reject approval with wrong expected_digest
        let bad_digest_approve = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_APPROVE,
                serde_json::json!({
                    "id": entry_id,
                    "expected_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                }),
            ))
            .await;
        assert!(!bad_digest_approve.is_success());
        assert!(bad_digest_approve.error.unwrap().contains("expected digest"));

        // 2b. Approve with correct expected_digest succeeds
        let good_approve = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_APPROVE,
                serde_json::json!({
                    "id": entry_id,
                    "expected_digest": valid_digest
                }),
            ))
            .await;
        assert!(good_approve.is_success());
        assert_eq!(good_approve.result.unwrap()["status"], "approved");

        // 3. K3: Unsupported recipient scheme fails closed
        let unsupported_create = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_CREATE,
                serde_json::json!({
                    "task_id": task.id,
                    "action_type": "legacy_ftp_dispatch",
                    "recipient": "ftp://unsupported.server/inbox",
                    "payload": { "data": "test" }
                }),
            ))
            .await;
        assert!(unsupported_create.is_success());
        let unsupported_id = unsupported_create.result.unwrap()["id"].as_str().unwrap().to_string();

        let approve_unsupported = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_APPROVE,
                serde_json::json!({ "id": unsupported_id }),
            ))
            .await;
        assert!(approve_unsupported.is_success());

        // Dispatch must fail closed with unsupported protocol
        let dispatch_unsupported = dispatcher
            .handle_request(request(
                METHOD_ASSISTANT_OUTBOX_DISPATCH,
                serde_json::json!({ "id": unsupported_id }),
            ))
            .await;
        assert!(!dispatch_unsupported.is_success());
        assert!(dispatch_unsupported.error.unwrap().contains("Unsupported outbox recipient protocol"));

        // Verify entry marked failed in store
        let stored_entry = outbox.get(&unsupported_id).await.unwrap().unwrap();
        assert_eq!(stored_entry.status, OutboxStatus::Failed);
    }

