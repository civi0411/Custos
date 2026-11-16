use super::*;
use std::collections::HashMap;
use custos_core::contracts::notebook::{CellExecutionStatus, ExecuteCellParams, NotebookCell};

#[derive(Default)]
struct InMemoryNotebookStorage {
    states: std::sync::Mutex<HashMap<String, NotebookKernelState>>,
}

#[async_trait::async_trait]
impl NotebookStoragePort for InMemoryNotebookStorage {
    async fn list_notebook_cells(&self, _s: &str) -> Result<Vec<NotebookCell>, DomainError> {
        Ok(vec![])
    }
    async fn replace_notebook_cells(&self, _s: &str, _c: &[NotebookCell]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn save_notebook_cell(&self, _c: &NotebookCell) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_notebook_kernel_state(&self, s: &str) -> Result<NotebookKernelState, DomainError> {
        self.states.lock().unwrap().get(s).cloned().ok_or_else(|| DomainError::NotFound {
            kind: "NotebookKernelState".to_string(),
            id: s.to_string(),
        })
    }
    async fn save_notebook_kernel_state(&self, state: &NotebookKernelState) -> Result<(), DomainError> {
        self.states.lock().unwrap().insert(state.session_id.clone(), state.clone());
        Ok(())
    }
}

fn test_coordinator() -> PythonKernelCoordinator {
    PythonKernelCoordinator::new(Arc::new(custos_adapters::notebook::LocalPythonDriver))
}

#[tokio::test]
async fn restarting_coordinator_advances_persisted_worker_epoch() {
    let storage = InMemoryNotebookStorage::default();
    storage
        .save_notebook_kernel_state(&NotebookKernelState {
            session_id: "restart".into(),
            epoch: 4,
            status: KernelStatus::Busy,
            python_version: "Python".into(),
            execution_counter: 7,
            created_at: 0,
            updated_at: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    let coordinator = test_coordinator();
    let status = coordinator
        .get_status("restart", Some(&storage as &dyn NotebookStoragePort))
        .await
        .unwrap();
    assert_eq!(status.epoch, 5);
    assert_eq!(status.status, KernelStatus::Idle);
    assert_eq!(status.execution_counter, 0);
    assert_eq!(
        coordinator
            .get_status("restart", Some(&storage as &dyn NotebookStoragePort))
            .await
            .unwrap()
            .epoch,
        5
    );
}

#[tokio::test]
async fn interrupt_releases_busy_worker_and_rejects_stale_result() {
    let coordinator = test_coordinator();
    let temp = tempfile::tempdir().unwrap();
    let params = ExecuteCellParams {
        session_id: "interrupt-session".into(),
        cell_id: "cell".into(),
        code: "import time; time.sleep(30)".into(),
        workspace_id: None,
        cwd: Some(temp.path().to_string_lossy().into()),
        task_id: None,
        ..Default::default()
    };
    let running = {
        let coordinator = coordinator.clone();
        let params = params.clone();
        tokio::spawn(async move { coordinator.execute_cell(params, None).await })
    };
    for _ in 0..100 {
        if coordinator
            .kernels
            .read()
            .await
            .get("interrupt-session")
            .is_some_and(|kernel| kernel.status == KernelStatus::Busy)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        coordinator
            .execute_cell(params.clone(), None)
            .await
            .is_err(),
        "concurrent writer must not queue another execution"
    );
    assert!(coordinator.interrupt("interrupt-session").await.unwrap());
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), running)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    let mut next = params;
    next.code = "print(42)".into();
    let output = coordinator.execute_cell(next, None).await.unwrap();
    assert_eq!(output.stdout.trim(), "42");
    assert_eq!(output.epoch, 2);
    coordinator.reset("interrupt-session", None).await.unwrap();
}

#[tokio::test]
async fn test_python_kernel_execution_and_epoch_reset() {
    let coordinator = test_coordinator();

    // 1. Initial status
    let initial_state = coordinator.get_status("test_sess", None).await.unwrap();
    assert_eq!(initial_state.epoch, 1);
    assert_eq!(initial_state.execution_counter, 0);

    // 2. Execute simple print cell
    let res = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: "test_sess".into(),
                cell_id: "cell_1".into(),
                code: "print('Custos Kernel Epoch Test')".into(),
                workspace_id: None,
                cwd: None,
                task_id: None,
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.status, CellExecutionStatus::Success);
    assert_eq!(res.execution_count, 1);
    assert_eq!(res.epoch, 1);
    assert!(res.stdout.contains("Custos Kernel Epoch Test"));

    // 3. Reset kernel increments epoch and clears execution count
    let reset_state = coordinator.reset("test_sess", None).await.unwrap();
    assert_eq!(reset_state.epoch, 2);
    assert_eq!(reset_state.execution_counter, 0);

    // 4. Next execution runs in epoch 2 with execution count 1
    let res2 = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: "test_sess".into(),
                cell_id: "cell_2".into(),
                code: "x = 42\nprint(f'x is {x}')".into(),
                workspace_id: None,
                cwd: None,
                task_id: None,
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(res2.status, CellExecutionStatus::Success);
    assert_eq!(res2.epoch, 2);
    assert_eq!(res2.execution_count, 1);
    assert!(res2.stdout.contains("x is 42"));

    // 5. Test variable persistence in subsequent cell
    let res3 = coordinator
        .execute_cell(
            ExecuteCellParams {
                session_id: "test_sess".into(),
                cell_id: "cell_3".into(),
                code: "print(f'persisted x: {x * 2}')".into(),
                workspace_id: None,
                cwd: None,
                task_id: None,
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(res3.status, CellExecutionStatus::Success);
    assert_eq!(res3.epoch, 2);
    assert_eq!(res3.execution_count, 2);
    assert!(res3.stdout.contains("persisted x: 84"));
}

#[tokio::test]
async fn test_multiple_notebooks_in_same_session_are_isolated() {
    let coordinator = test_coordinator();

    let s1 = coordinator
        .get_status_scoped("sess-1", Some("nb-a"), None)
        .await
        .unwrap();
    assert_eq!(s1.notebook_id.as_deref(), Some("nb-a"));
    assert_eq!(s1.execution_counter, 0);

    let s2 = coordinator
        .get_status_scoped("sess-1", Some("nb-b"), None)
        .await
        .unwrap();
    assert_eq!(s2.notebook_id.as_deref(), Some("nb-b"));
    assert_eq!(s2.execution_counter, 0);

    // Reset nb-a does not affect nb-b
    coordinator
        .reset_scoped("sess-1", Some("nb-a"), None)
        .await
        .unwrap();
    let s1_after = coordinator
        .get_status_scoped("sess-1", Some("nb-a"), None)
        .await
        .unwrap();
    let s2_after = coordinator
        .get_status_scoped("sess-1", Some("nb-b"), None)
        .await
        .unwrap();
    assert_eq!(s1_after.epoch, 2);
    assert_eq!(s2_after.epoch, 1);
}

#[tokio::test]
async fn test_duplicate_execution_request_id_returns_cached_result() {
    let coordinator = test_coordinator();
    let params = ExecuteCellParams {
        session_id: "idempotent_sess".into(),
        cell_id: "cell_1".into(),
        code: "print('computed_value')".into(),
        execution_request_id: Some("req-12345".into()),
        ..Default::default()
    };

    let res1 = coordinator
        .execute_cell(params.clone(), None)
        .await
        .unwrap();
    assert_eq!(res1.execution_count, 1);
    assert!(res1.stdout.contains("computed_value"));

    // Second call with same execution_request_id returns cached result without incrementing counter
    let res2 = coordinator.execute_cell(params, None).await.unwrap();
    assert_eq!(res2.execution_count, 1);
    assert_eq!(res2.execution_request_id.as_deref(), Some("req-12345"));
    assert!(res2.stdout.contains("computed_value"));
}
