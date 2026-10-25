use super::super::*;
use std::sync::Arc;
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
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
        store.save_run(&mut run).await.unwrap();
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
        store.save_run(&mut run).await.unwrap();
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

