//! Golden Acceptance Test G06 — Notebook Document/Binding/Kernel Identity
//!
//! Validates:
//! 1. Two notebooks in the same session have isolated kernels & memory.
//! 2. Two panes opening the same notebook share the same kernel binding handle & memory.
//! 3. Cell edit invalidates previous output and increments cell_version.
//! 4. Duplicate execution request ID returns cached result without re-executing.
//! 5. Replacing cells in notebook A does not delete cells in notebook B in the same session.

use custos_adapters::notebook::LocalPythonDriver;
use custos_core::contracts::notebook::{CellExecutionStatus, ExecuteCellParams, NotebookCell, NotebookCellType};
use custos_persistence::SqliteTaskStore;
use custos_runtime::PythonKernelCoordinator;
use std::sync::Arc;

fn test_coordinator() -> PythonKernelCoordinator {
    PythonKernelCoordinator::new(Arc::new(LocalPythonDriver))
}

#[tokio::test]
async fn test_g06_two_notebooks_in_same_session_have_isolated_kernels() {
    let coordinator = test_coordinator();
    let session_id = "g06-sess-isolated";

    // 1. Notebook Alpha sets alpha_val = 1337
    let alpha_res = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: session_id.into(),
                notebook_id: Some("nb-alpha".into()),
                cell_id: "cell-alpha-1".into(),
                code: "alpha_val = 1337\nprint(f'alpha: {alpha_val}')".into(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("execution of alpha cell");

    assert_eq!(alpha_res.status, CellExecutionStatus::Success);
    assert_eq!(alpha_res.execution_count, 1);
    assert!(alpha_res.stdout.contains("alpha: 1337"));

    // 2. Notebook Beta attempts to access alpha_val in the same session
    let beta_res = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: session_id.into(),
                notebook_id: Some("nb-beta".into()),
                cell_id: "cell-beta-1".into(),
                code: "print(f'beta: {alpha_val}')".into(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("execution of beta cell");

    // Beta must fail with NameError because Python kernel memory is completely isolated
    assert_eq!(beta_res.status, CellExecutionStatus::Error);
    assert!(
        beta_res.stderr.contains("NameError") && beta_res.stderr.contains("alpha_val"),
        "Beta kernel must not have access to Alpha kernel memory: stderr={}",
        beta_res.stderr
    );
}

#[tokio::test]
async fn test_g06_two_panes_opening_same_notebook_share_kernel_memory() {
    let coordinator = test_coordinator();
    let session_id = "g06-sess-shared";
    let notebook_id = "nb-shared-doc";

    // Pane 1 executes code setting shared_var
    let pane1_res = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: session_id.into(),
                notebook_id: Some(notebook_id.into()),
                cell_id: "pane1-cell".into(),
                code: "shared_var = 999\nprint('stored shared_var')".into(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("pane 1 execution");

    assert_eq!(pane1_res.status, CellExecutionStatus::Success);
    assert_eq!(pane1_res.execution_count, 1);

    // Pane 2 executes code reading shared_var
    let pane2_res = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: session_id.into(),
                notebook_id: Some(notebook_id.into()),
                cell_id: "pane2-cell".into(),
                code: "print(f'read shared_var: {shared_var * 2}')".into(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("pane 2 execution");

    assert_eq!(pane2_res.status, CellExecutionStatus::Success);
    assert_eq!(pane2_res.execution_count, 2);
    assert!(pane2_res.stdout.contains("read shared_var: 1998"));
}

#[test]
fn test_g06_cell_source_edit_invalidates_output_and_bumps_version() {
    let mut cell = NotebookCell {
        id: "cell-version-test".into(),
        session_id: "sess-v".into(),
        notebook_id: Some("nb-v".into()),
        cell_index: 0,
        cell_type: NotebookCellType::Code,
        source: "print('original')".into(),
        execution_count: Some(1),
        status: CellExecutionStatus::Success,
        stdout: Some("original\n".into()),
        stderr: None,
        output_image: None,
        wall_ms: Some(15),
        epoch: 1,
        cell_version: Some(1),
        updated_at: 1000,
    };

    assert!(!cell.is_version_mismatch(Some(1)));
    assert!(cell.is_version_mismatch(Some(2)));

    // 1. Edit with unchanged source does NOT invalidate
    let changed_no_op = cell.invalidate_output_on_edit("print('original')", 1010);
    assert!(!changed_no_op);
    assert_eq!(cell.cell_version, Some(1));
    assert_eq!(cell.status, CellExecutionStatus::Success);
    assert!(cell.stdout.is_some());

    // 2. Edit with new source invalidates previous output and increments cell_version
    let changed = cell.invalidate_output_on_edit("print('modified')", 1020);
    assert!(changed);
    assert_eq!(cell.cell_version, Some(2));
    assert_eq!(cell.status, CellExecutionStatus::Idle);
    assert!(cell.stdout.is_none());
    assert!(cell.stderr.is_none());
    assert!(cell.execution_count.is_none());
    assert_eq!(cell.updated_at, 1020);

    // Version mismatch check: old expected version 1 is now rejected
    assert!(cell.is_version_mismatch(Some(1)));
    assert!(!cell.is_version_mismatch(Some(2)));
}

#[tokio::test]
async fn test_g06_duplicate_execution_request_id_returns_cached_result() {
    let coordinator = test_coordinator();
    let session_id = "g06-sess-idempotency";

    let params = ExecuteCellParams {
        session_id: session_id.into(),
        notebook_id: Some("nb-idemp".into()),
        cell_id: "cell-idemp-1".into(),
        code: "print('evaluating once')".into(),
        execution_request_id: Some("req-g06-unique-42".into()),
        ..Default::default()
    };

    // First execution
    let res1 = coordinator
        .execute_cell(params.clone(), None)
        .await
        .expect("first execution");
    assert_eq!(res1.execution_count, 1);
    assert!(res1.stdout.contains("evaluating once"));

    // Second execution with identical execution_request_id
    let res2 = coordinator
        .execute_cell(params, None)
        .await
        .expect("duplicate execution request");
    // Execution count must remain 1 (not re-executed)
    assert_eq!(res2.execution_count, 1);
    assert_eq!(
        res2.execution_request_id.as_deref(),
        Some("req-g06-unique-42")
    );
    assert!(res2.stdout.contains("evaluating once"));
}

#[tokio::test]
async fn test_g06_multi_notebook_persistence_isolation_in_same_session() {
    let store = SqliteTaskStore::new_in_memory().expect("in-memory sqlite store");
    let repo = store.shared_capabilities();
    let session_id = "g06-sess-persistence";

    // 1. Write cells for notebook A
    let nb_a_cell1 = NotebookCell {
        id: "cell-a1".into(),
        session_id: session_id.into(),
        notebook_id: Some("nb-alpha".into()),
        cell_index: 0,
        cell_type: NotebookCellType::Code,
        source: "x = 10".into(),
        cell_version: Some(1),
        ..Default::default()
    };
    let nb_a_cell2 = NotebookCell {
        id: "cell-a2".into(),
        session_id: session_id.into(),
        notebook_id: Some("nb-alpha".into()),
        cell_index: 1,
        cell_type: NotebookCellType::Code,
        source: "y = 20".into(),
        cell_version: Some(1),
        ..Default::default()
    };
    repo.replace_notebook_cells_scoped(session_id, Some("nb-alpha"), &[nb_a_cell1, nb_a_cell2])
        .expect("save nb-alpha cells");

    // 2. Write cells for notebook B in the same session
    let nb_b_cell1 = NotebookCell {
        id: "cell-b1".into(),
        session_id: session_id.into(),
        notebook_id: Some("nb-beta".into()),
        cell_index: 0,
        cell_type: NotebookCellType::Code,
        source: "msg = 'hello'".into(),
        cell_version: Some(1),
        ..Default::default()
    };
    repo.replace_notebook_cells_scoped(session_id, Some("nb-beta"), &[nb_b_cell1])
        .expect("save nb-beta cells");

    // Verify initial counts
    let cells_a = repo
        .list_notebook_cells_scoped(session_id, Some("nb-alpha"))
        .expect("list alpha");
    assert_eq!(cells_a.len(), 2);

    let cells_b = repo
        .list_notebook_cells_scoped(session_id, Some("nb-beta"))
        .expect("list beta");
    assert_eq!(cells_b.len(), 1);

    // 3. Now replace cells in notebook A with only 1 new cell
    let nb_a_new = NotebookCell {
        id: "cell-a-replacement".into(),
        session_id: session_id.into(),
        notebook_id: Some("nb-alpha".into()),
        cell_index: 0,
        cell_type: NotebookCellType::Code,
        source: "z = 30".into(),
        cell_version: Some(2),
        ..Default::default()
    };
    repo.replace_notebook_cells_scoped(session_id, Some("nb-alpha"), &[nb_a_new])
        .expect("replace alpha");

    // 4. Assert: Notebook B cells were NOT wiped!
    let cells_b_after = repo
        .list_notebook_cells_scoped(session_id, Some("nb-beta"))
        .expect("list beta after alpha replacement");
    assert_eq!(
        cells_b_after.len(),
        1,
        "Notebook B cells must be preserved!"
    );
    assert_eq!(cells_b_after[0].id, "cell-b1");

    // Notebook A now has only the replacement cell
    let cells_a_after = repo
        .list_notebook_cells_scoped(session_id, Some("nb-alpha"))
        .expect("list alpha after replacement");
    assert_eq!(cells_a_after.len(), 1);
    assert_eq!(cells_a_after[0].id, "cell-a-replacement");
    assert_eq!(cells_a_after[0].cell_version, Some(2));
}
