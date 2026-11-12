use std::sync::{Arc, Mutex};

use custos_core::contracts::notebook::{
    CellExecutionStatus, ExecuteCellResult, NotebookCell, NotebookCellType, NotebookKernelState,
};
use custos_core::TaskService;
use custos_domain::*;
use custos_persistence::SharedCapabilitiesRepository;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;

use crate::api::tests::harness::MockStore;
use crate::api::*;

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
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_sqlite_store(store)
        .with_workspace(coordinator);
    let session_id = "test_nb_session_42";
    let path = temp_dir
        .path()
        .join("notebook-workspace")
        .to_string_lossy()
        .to_string();
    let created = dispatcher
        .handle_request(ApiRequest {
            id: "nb_workspace".into(),
            method: METHOD_WORKSPACES_CREATE.into(),
            params: serde_json::json!({
                "name": "notebook",
                "kind": { "type": "folder", "path": path },
                "path": path
            }),
        })
        .await;
    assert!(created.is_success(), "{:?}", created.error);
    let workspace_id = created.result.unwrap()["id"].as_str().unwrap().to_string();

    // 1. Initial kernel status
    let status_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_status_1".into(),
            method: METHOD_NOTEBOOK_STATUS.into(),
            params: serde_json::json!({ "session_id": session_id }),
        })
        .await;
    assert!(status_resp.is_success());
    let initial_state: NotebookKernelState =
        serde_json::from_value(status_resp.result.unwrap()).unwrap();
    assert_eq!(initial_state.epoch, 1);
    assert_eq!(initial_state.execution_counter, 0);

    // 2. Save cells
    let cell = NotebookCell {
        id: custos_domain::new_id("cell"),
        session_id: session_id.to_string(),
        cell_type: NotebookCellType::Code,
        source: "print(6 * 7)".to_string(),
        cell_index: 0,
        execution_count: None,
        status: CellExecutionStatus::Idle,
        stdout: None,
        stderr: None,
        output_image: None,
        wall_ms: None,
        epoch: 0,
        updated_at: chrono::Utc::now().timestamp(),
        ..Default::default()
    };
    let save_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_save_1".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({ "session_id": session_id, "cells": [cell.clone()] }),
        })
        .await;
    assert!(save_resp.is_success(), "Failed to save cells: {:?}", save_resp.error);

    let mut forged = cell.clone();
    forged.status = CellExecutionStatus::Success;
    forged.stdout = Some("fabricated output".into());
    forged.execution_count = Some(99);
    let forged_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_forged_result".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({ "session_id": session_id, "cells": [forged] }),
        })
        .await;
    assert!(forged_resp.is_success());

    let mut wrong_session = cell.clone();
    wrong_session.session_id = "other_session".into();
    let wrong_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_wrong_session".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({ "session_id": session_id, "cells": [wrong_session] }),
        })
        .await;
    assert!(!wrong_resp.is_success());

    // 3. List cells
    let list_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_list_1".into(),
            method: METHOD_NOTEBOOK_CELLS_LIST.into(),
            params: serde_json::json!({ "session_id": session_id }),
        })
        .await;
    assert!(list_resp.is_success());
    let loaded_cells: Vec<NotebookCell> =
        serde_json::from_value(list_resp.result.unwrap()).unwrap();
    assert_eq!(loaded_cells.len(), 1);
    assert_eq!(loaded_cells[0].source, "print(6 * 7)");
    assert_eq!(loaded_cells[0].status, CellExecutionStatus::Idle);
    assert_eq!(loaded_cells[0].execution_count, None);
    assert_eq!(loaded_cells[0].stdout, None);

    let unscoped = dispatcher
        .handle_request(ApiRequest {
            id: "nb_unscoped".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": cell.source
            }),
        })
        .await;
    assert!(!unscoped.is_success());
    let changed_source = dispatcher
        .handle_request(ApiRequest {
            id: "nb_changed".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": "print('not saved')",
                "workspace_id": workspace_id
            }),
        })
        .await;
    assert!(!changed_source.is_success());

    // 4. Execute cell
    let exec_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_exec_1".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": "print(6 * 7)",
                "workspace_id": workspace_id,
            }),
        })
        .await;
    assert!(exec_resp.is_success(), "Execution failed: {:?}", exec_resp.error);
    let exec_result: ExecuteCellResult = serde_json::from_value(exec_resp.result.unwrap()).unwrap();
    assert_eq!(exec_result.status, CellExecutionStatus::Success);
    assert_eq!(exec_result.stdout.trim(), "42");
    assert_eq!(exec_result.execution_count, 1);
    assert_eq!(exec_result.epoch, 1);

    // Changing code into notes must not retain executed Python output.
    let mut markdown = cell.clone();
    markdown.cell_type = NotebookCellType::Markdown;
    let save_markdown = dispatcher
        .handle_request(ApiRequest {
            id: "nb_markdown".into(),
            method: METHOD_NOTEBOOK_CELLS_SAVE.into(),
            params: serde_json::json!({"session_id": session_id, "cells": [markdown]}),
        })
        .await;
    assert!(save_markdown.is_success(), "{:?}", save_markdown.error);
    let notes = dispatcher
        .handle_request(ApiRequest {
            id: "nb_notes".into(),
            method: METHOD_NOTEBOOK_CELLS_LIST.into(),
            params: serde_json::json!({"session_id": session_id}),
        })
        .await
        .result
        .unwrap();
    assert_eq!(notes[0]["cell_type"], "markdown");
    assert!(notes[0]["stdout"].is_null());
    assert!(notes[0]["execution_count"].is_null());
    let execute_notes = dispatcher
        .handle_request(ApiRequest {
            id: "nb_notes_execute".into(),
            method: METHOD_NOTEBOOK_EXECUTE.into(),
            params: serde_json::json!({
                "session_id": session_id,
                "cell_id": cell.id,
                "code": cell.source,
                "workspace_id": workspace_id
            }),
        })
        .await;
    assert!(!execute_notes.is_success());

    // 5. Reset kernel
    let reset_resp = dispatcher
        .handle_request(ApiRequest {
            id: "nb_reset_1".into(),
            method: METHOD_NOTEBOOK_RESET.into(),
            params: serde_json::json!({ "session_id": session_id }),
        })
        .await;
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
    let db = custos_persistence::DbConnection::open_in_memory().expect("open memory db");
    let shared = Arc::new(SharedCapabilitiesRepository::new(db));
    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_shared_capabilities(shared);

    // 1. Record a review
    let record_resp = dispatcher
        .handle_request(ApiRequest {
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
        })
        .await;
    assert!(
        record_resp.is_success(),
        "Failed to record review: {:?}",
        record_resp.error
    );
    let record: ReviewerRecord = serde_json::from_value(record_resp.result.unwrap()).unwrap();
    assert_eq!(record.reviewer, "custos-verifier");
    assert_eq!(record.status, ReviewStatus::Approved);
    assert!(record.is_fresh);
    assert_eq!(record.findings.len(), 1);

    // 2. Get review by id
    let get_resp = dispatcher
        .handle_request(ApiRequest {
            id: "rev_get_1".into(),
            method: METHOD_REVIEWS_GET.into(),
            params: serde_json::json!({ "review_id": record.id }),
        })
        .await;
    assert!(get_resp.is_success());
    let fetched: Option<ReviewerRecord> = serde_json::from_value(get_resp.result.unwrap()).unwrap();
    assert_eq!(fetched.unwrap().id, record.id);

    // 3. List reviews by target
    let list_resp = dispatcher
        .handle_request(ApiRequest {
            id: "rev_list_1".into(),
            method: METHOD_REVIEWS_LIST.into(),
            params: serde_json::json!({
                "target_type": "artifact",
                "target_id": "artifacts/report.md"
            }),
        })
        .await;
    assert!(list_resp.is_success());
    let list: Vec<ReviewerRecord> = serde_json::from_value(list_resp.result.unwrap()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, record.id);

    // 4. Mark review stale
    let stale_resp = dispatcher
        .handle_request(ApiRequest {
            id: "rev_stale_1".into(),
            method: METHOD_REVIEWS_MARK_STALE.into(),
            params: serde_json::json!({ "review_id": record.id }),
        })
        .await;
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
    let verify_get_resp = dispatcher
        .handle_request(ApiRequest {
            id: "rev_get_2".into(),
            method: METHOD_REVIEWS_GET.into(),
            params: serde_json::json!({ "review_id": record.id }),
        })
        .await;
    assert!(verify_get_resp.is_success());
    let verified: Option<ReviewerRecord> =
        serde_json::from_value(verify_get_resp.result.unwrap()).unwrap();
    assert!(!verified.unwrap().is_fresh);
}

#[tokio::test]
async fn test_artifact_and_notes_api_lifecycle() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));

    let dispatcher =
        LocalApiDispatcher::new(task_service, session_manager, bridge_service).with_sqlite_store(store);

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
    let graph: ArtifactLineageGraph = serde_json::from_value(dag_resp.result.unwrap()).unwrap();
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
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let shared = store.shared_capabilities();
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store.clone())
        .with_workspace(coordinator);
    let path = temp.path().join("workspace").to_string_lossy().to_string();
    let created = dispatcher
        .handle_request(ApiRequest {
            id: "artifact-workspace".into(),
            method: METHOD_WORKSPACES_CREATE.into(),
            params: serde_json::json!({
                "name": "artifacts",
                "kind": { "type": "folder", "path": path },
                "path": path
            }),
        })
        .await;
    assert!(created.is_success(), "{:?}", created.error);
    let workspace_id = created.result.unwrap()["id"].as_str().unwrap().to_string();
    let file = std::path::Path::new(&path).join("analysis.py");
    std::fs::write(&file, "print(42)").unwrap();
    shared
        .record_artifact_lineage(&ArtifactLineageNode {
            artifact_path: "analysis.py".into(),
            version: 1,
            content_hash: blake3::hash(b"print(42)").to_hex().to_string(),
            produced_by_run_id: None,
            parent_version_hash: None,
            timestamp: 0,
        })
        .unwrap();
    let read = |params| ApiRequest {
        id: "artifact-read".into(),
        method: METHOD_ARTIFACTS_GET.into(),
        params,
    };
    let scoped = dispatcher
        .handle_request(read(
            serde_json::json!({"artifact_path": "analysis.py", "workspace_id": workspace_id}),
        ))
        .await;
    assert!(scoped.is_success(), "{:?}", scoped.error);
    assert_eq!(scoped.result.as_ref().unwrap()["content_version"], 1);
    assert_eq!(scoped.result.as_ref().unwrap()["content"], "print(42)");
    std::fs::write(&file, "print('changed')").unwrap();
    let changed = dispatcher
        .handle_request(read(
            serde_json::json!({"artifact_path": "analysis.py", "workspace_id": workspace_id}),
        ))
        .await;
    assert!(changed.is_success());
    assert!(changed
        .result
        .as_ref()
        .unwrap()
        .get("content_version")
        .is_none());
    assert!(changed.result.as_ref().unwrap()["content"].is_null());
    let escaped = dispatcher
        .handle_request(read(
            serde_json::json!({"artifact_path": "../secret.txt", "workspace_id": workspace_id}),
        ))
        .await;
    assert!(!escaped.is_success());
    let unscoped = dispatcher
        .handle_request(read(
            serde_json::json!({"artifact_path": file.to_string_lossy()}),
        ))
        .await;
    assert!(unscoped.is_success());
    assert!(unscoped.result.as_ref().unwrap()["content"].is_null());
    let annotation = AnnotationRecord::new_text(
        "analysis.py",
        1,
        0,
        5,
        "print",
        "Check this calculation",
        "researcher",
    );
    let save = |annotation: &AnnotationRecord| ApiRequest {
        id: "annotation".into(),
        method: "v1.research.annotations.save".into(),
        params: serde_json::to_value(annotation).unwrap(),
    };
    assert!(dispatcher.handle_request(save(&annotation)).await.is_success());
    let mut forged = annotation.clone();
    forged.artifact_version = 999;
    assert!(!dispatcher.handle_request(save(&forged)).await.is_success());
    forged = annotation.clone();
    forged.status = AnnotationStatus::Addressed;
    assert!(!dispatcher.handle_request(save(&forged)).await.is_success());
    assert_eq!(
        shared
            .list_annotations_for_artifact("analysis.py", Some(1))
            .unwrap()
            .len(),
        1
    );
}

