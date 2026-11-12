use super::SharedCapabilitiesRepository;
use crate::connection::DbConnection;
use custos_core::contracts::notebook::{
    CellExecutionStatus, KernelStatus, NotebookCell, NotebookCellType, NotebookKernelState,
};
use custos_domain::{
    AnnotationRecord, AnnotationStatus, ArtifactLineageNode, FindingSeverity, NoteRecord,
    ReviewFinding, ReviewMethod, ReviewStatus, ReviewTargetType, ReviewerRecord,
};

#[test]
fn test_shared_capabilities_full_lifecycle() {
    let db = DbConnection::open_in_memory().expect("open db");
    let repo = SharedCapabilitiesRepository::new(db);

    // 1. Artifact Lineage & DAG
    let lineage = ArtifactLineageNode {
        artifact_path: "eval_table.csv".to_string(),
        version: 1,
        content_hash: "hash_csv".to_string(),
        produced_by_run_id: Some("run_exp_99".to_string()),
        parent_version_hash: None,
        timestamp: 1700000400,
    };
    repo.record_artifact_lineage(&lineage)
        .expect("record lineage");
    let lineage_list = repo
        .list_artifact_lineage("eval_table.csv")
        .expect("list lineage");
    assert_eq!(lineage_list.len(), 1);
    assert_eq!(lineage_list[0].content_hash, "hash_csv");

    let all_artifacts = repo.list_all_artifacts().expect("list all artifacts");
    assert_eq!(all_artifacts.len(), 1);
    assert_eq!(all_artifacts[0].latest_version, 1);

    let graph = repo
        .get_artifact_lineage_graph(None)
        .expect("get lineage graph");
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "artifact:eval_table.csv:v1"));

    // 2. Annotations
    let ann_img = AnnotationRecord::new_image_pin(
        "figures/loss_curve.png",
        1,
        0.35,
        0.75,
        1,
        "Add error bars at epoch 50",
        "alice",
    )
    .with_side_chat("session_side_01");
    let ann_img_id = ann_img.id.clone();
    repo.save_annotation(&ann_img).expect("save annotation");

    assert_eq!(repo.list_pending_annotations().unwrap().len(), 1);
    repo.update_annotation_status(&ann_img_id, AnnotationStatus::Submitted, Some("turn_101"))
        .expect("update annotation status");
    assert_eq!(repo.list_pending_annotations().unwrap().len(), 0);

    // 3. Notes & Version History
    let mut note = NoteRecord::new(
        "Convergence Architecture Notes",
        "# Custos SADE Convergence",
        Some("sess_res_01".into()),
        None,
        vec!["sade".into(), "architecture".into()],
    )
    .expect("create note");
    let note_id = note.id.clone();
    repo.save_note(&note).expect("save note");

    let prev_ver = note
        .update(
            Some("Convergence Architecture Notes (Updated)".into()),
            "# Updated".into(),
            None,
        )
        .expect("update note");
    repo.save_note(&note).expect("save updated note");
    repo.save_note_version(&prev_ver)
        .expect("save note version");
    assert_eq!(repo.list_note_versions(&note_id).unwrap().len(), 1);
    assert_eq!(repo.list_notes(Some("sess_res_01")).unwrap()[0].version, 2);

    // 4. Reviewer Records
    let review = ReviewerRecord::new(
        ReviewTargetType::Diff,
        "patch_hash_987",
        "verifier:security-linter",
        ReviewMethod::AutomatedVerifier,
        ReviewStatus::Approved,
        "No vulnerabilities found.",
        Some("sha256:fedcba".into()),
        vec![ReviewFinding {
            severity: FindingSeverity::Info,
            criterion: "no_command_injection".to_string(),
            message: "Passed fail-closed validation".into(),
            file_path: Some("platform/runtime/src/lib.rs".into()),
            line_number: Some(42),
        }],
    )
    .expect("create review");
    repo.save_review(&review).expect("save review");
    assert!(repo.get_review(&review.id).unwrap().unwrap().is_fresh);
    repo.mark_review_stale(&review.id).expect("mark stale");
    assert!(!repo.get_review(&review.id).unwrap().unwrap().is_fresh);
}

#[test]
fn test_scoped_notebook_cell_and_kernel_persistence() {
    let db = DbConnection::open_in_memory().expect("open db");
    let repo = SharedCapabilitiesRepository::new(db);

    let cell1 = NotebookCell {
        id: "c_alpha_1".into(),
        session_id: "sess_multi".into(),
        cell_type: NotebookCellType::Code,
        source: "x = 1".into(),
        cell_index: 0,
        status: CellExecutionStatus::Success,
        notebook_id: Some("nb_alpha".into()),
        ..Default::default()
    };
    let cell2 = NotebookCell {
        id: "c_beta_1".into(),
        session_id: "sess_multi".into(),
        cell_type: NotebookCellType::Code,
        source: "y = 2".into(),
        cell_index: 0,
        status: CellExecutionStatus::Success,
        notebook_id: Some("nb_beta".into()),
        ..Default::default()
    };
    repo.save_notebook_cell(&cell1).unwrap();
    repo.save_notebook_cell(&cell2).unwrap();

    assert_eq!(
        repo.list_notebook_cells_scoped("sess_multi", Some("nb_alpha"))
            .unwrap()[0]
            .id,
        "c_alpha_1"
    );
    assert_eq!(
        repo.list_notebook_cells_scoped("sess_multi", Some("nb_beta"))
            .unwrap()[0]
            .id,
        "c_beta_1"
    );

    repo.save_notebook_kernel_state(&NotebookKernelState {
        session_id: "sess_multi".into(),
        epoch: 2,
        status: KernelStatus::Idle,
        python_version: "3.11".into(),
        execution_counter: 5,
        created_at: 100,
        updated_at: 200,
        notebook_id: Some("nb_alpha".into()),
        kernel_id: Some("k_alpha".into()),
    })
    .unwrap();
    repo.save_notebook_kernel_state(&NotebookKernelState {
        session_id: "sess_multi".into(),
        epoch: 3,
        status: KernelStatus::Busy,
        python_version: "3.11".into(),
        execution_counter: 8,
        created_at: 100,
        updated_at: 300,
        notebook_id: Some("nb_beta".into()),
        kernel_id: Some("k_beta".into()),
    })
    .unwrap();

    assert_eq!(
        repo.get_notebook_kernel_state_scoped("sess_multi", Some("nb_alpha"))
            .unwrap()
            .notebook_id
            .as_deref(),
        Some("nb_alpha")
    );
    assert_eq!(
        repo.get_notebook_kernel_state_scoped("sess_multi", Some("nb_beta"))
            .unwrap()
            .notebook_id
            .as_deref(),
        Some("nb_beta")
    );
}

