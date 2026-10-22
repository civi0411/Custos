use super::super::*;
use super::harness::*;
use std::sync::{Arc, Mutex};
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
use custos_domain::*;
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

