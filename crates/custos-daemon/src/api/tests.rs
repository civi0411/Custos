    use super::*;
    use async_trait::async_trait;
    use custos_core::{TaskEvent, TaskStore};
    use custos_domain::{
        ContinuationPacket, DomainError, ExecuteCellResult, NotebookKernelState, Session, Span,
        Task, TaskStatus,
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
            ..Default::default()
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

    #[tokio::test]
    async fn command_admission_failure_denies_mutation_and_errors_are_replayed() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::with_store(store.clone()));
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_sqlite_store(store.clone());
        let request = ApiRequest::new("first", "v1.tasks.create", serde_json::json!({"command_id":"error_once"}));
        let first = dispatcher.handle_request(request.clone()).await;
        assert!(first.error.is_some());
        let mut retry = request;
        retry.id = "retry".into();
        let replay = dispatcher.handle_request(retry).await;
        assert_eq!(first.error, replay.error);
        assert_eq!(replay.id, "retry");
        assert_eq!(store.command_ledger().get_command("error_once").unwrap().unwrap().result_json.contains("custos_command_response"), true);

        store.db().lock().unwrap().execute("DROP TABLE command_ledger", []).unwrap();
        let denied = dispatcher.handle_request(ApiRequest::new("denied", "v1.tasks.create", serde_json::json!({"title":"Must not exist","command_id":"deny_me"}))).await;
        assert!(denied.error.unwrap().contains("Command admission failed"));
        assert!(store.list_tasks().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_compute_targets_and_experiment_lifecycle() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
            .with_sqlite_store(store.clone());

        // 1. List compute targets -> must return Colab, Kaggle, Rented GPU / SSH
        let targets_resp = dispatcher
            .handle_request(ApiRequest {
                id: "targets_1".into(),
                method: METHOD_COMPUTE_TARGETS_LIST.into(),
                params: serde_json::json!({}),
            })
            .await;
        assert!(targets_resp.is_success());
        let targets = targets_resp.result.unwrap();
        let target_arr = targets.as_array().unwrap();
        assert!(target_arr.iter().any(|t| t["backend"] == "colab"));
        assert!(target_arr.iter().any(|t| t["backend"] == "kaggle"));
        assert!(target_arr.iter().any(|t| t["backend"] == "runpod"));

        // 2. Preflight on Colab
        let preflight_colab = dispatcher
            .handle_request(ApiRequest {
                id: "preflight_1".into(),
                method: METHOD_EXPERIMENT_PREFLIGHT.into(),
                params: serde_json::json!({ "backend": "colab", "recipe_id": "test_recipe" }),
            })
            .await;
        assert!(preflight_colab.is_success());

        // 3. Submit experiment attempt
        let submit_resp = dispatcher
            .handle_request(ApiRequest {
                id: "submit_1".into(),
                method: METHOD_EXPERIMENT_SUBMIT.into(),
                params: serde_json::json!({
                    "experiment_id": "exp_research_1",
                    "recipe_id": "recipe_finetune",
                    "backend": "colab",
                    "mock": true,
                }),
            })
            .await;
        assert!(submit_resp.is_success(), "{:?}", submit_resp.error);
        let submit_res = submit_resp.result.unwrap();
        let attempt_id = submit_res["attempt"]["id"].as_str().unwrap().to_string();

        // 4. Observe attempt
        let observe_resp = dispatcher
            .handle_request(ApiRequest {
                id: "obs_1".into(),
                method: METHOD_EXPERIMENT_OBSERVE.into(),
                params: serde_json::json!({ "attempt_id": attempt_id, "mock": true }),
            })
            .await;
        assert!(observe_resp.is_success(), "{:?}", observe_resp.error);

        // 5. Stop attempt
        let stop_resp = dispatcher
            .handle_request(ApiRequest {
                id: "stop_1".into(),
                method: METHOD_EXPERIMENT_STOP.into(),
                params: serde_json::json!({ "attempt_id": attempt_id, "mock": true }),
            })
            .await;
        assert!(stop_resp.is_success(), "{:?}", stop_resp.error);

        // 6. Release compute
        let release_resp = dispatcher
            .handle_request(ApiRequest {
                id: "release_1".into(),
                method: METHOD_EXPERIMENT_RELEASE.into(),
                params: serde_json::json!({ "attempt_id": attempt_id, "mock": true }),
            })
            .await;
        assert!(release_resp.is_success(), "{:?}", release_resp.error);

        // 7. Collect artifacts
        let collect_resp = dispatcher
            .handle_request(ApiRequest {
                id: "collect_1".into(),
                method: METHOD_EXPERIMENT_COLLECT.into(),
                params: serde_json::json!({ "attempt_id": attempt_id, "mock": true }),
            })
            .await;
        assert!(collect_resp.is_success(), "{:?}", collect_resp.error);
    }

    #[tokio::test]
    async fn test_outbox_mcp_dispatch_flow() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(TaskService::new(store.clone()));
        let sessions = Arc::new(SessionManager::new());
        let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
        let outbox_repo = Arc::new(store.outbox().clone());
        let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
            .with_sqlite_store(store.clone())
            .with_outbox(outbox_repo);

        // Register MCP server in federation hub
        let client = Arc::new(custos_adapters::mcp::CustosMcpClient::new("kaggle-cluster"));
        dispatcher.mcp_hub().register_server("kaggle", client).await.unwrap();

        // 1. Create task first
        let create_task_resp = dispatcher
            .handle_request(ApiRequest {
                id: "task_create_mcp".into(),
                method: "v1.tasks.create".into(),
                params: serde_json::json!({ "title": "Run Kaggle Job" }),
            })
            .await;
        assert!(create_task_resp.is_success());
        let task_id = create_task_resp.result.unwrap()["id"].as_str().unwrap().to_string();

        // 2. Enqueue outbox item targeting mcp://kaggle/run_notebook
        let create_outbox_resp = dispatcher
            .handle_request(ApiRequest {
                id: "ob_create_1".into(),
                method: METHOD_ASSISTANT_OUTBOX_CREATE.into(),
                params: serde_json::json!({
                    "task_id": task_id,
                    "action_type": "run_notebook",
                    "recipient": "mcp://kaggle/run_notebook",
                    "payload": { "notebook_id": "kernel_123", "gpu": "t4" },
                }),
            })
            .await;
        assert!(create_outbox_resp.is_success(), "{:?}", create_outbox_resp.error);
        let outbox_entry_id = create_outbox_resp.result.unwrap()["id"].as_str().unwrap().to_string();

        let approved = dispatcher.handle_request(ApiRequest {
            id: "ob_approve_1".into(),
            method: METHOD_ASSISTANT_OUTBOX_APPROVE.into(),
            params: serde_json::json!({"id":outbox_entry_id}),
        }).await;
        assert!(approved.is_success(), "{:?}", approved.error);

        // 3. Dispatch outbox item
        let dispatch_resp = dispatcher
            .handle_request(ApiRequest {
                id: "ob_dispatch_1".into(),
                method: METHOD_ASSISTANT_OUTBOX_DISPATCH.into(),
                params: serde_json::json!({ "id": outbox_entry_id }),
            })
            .await;
        assert!(dispatch_resp.is_success(), "{:?}", dispatch_resp.error);
        let dispatch_val = dispatch_resp.result.unwrap();
        assert_eq!(dispatch_val["status"], "receipted");
        assert!(dispatch_val["receipt"]["receipt_id"].is_string());
    }
}

