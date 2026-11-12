use std::sync::Arc;

use custos_core::contracts::WorkspaceRepository;
use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;

use crate::api::*;

#[tokio::test]
async fn test_candidate_race_api_validation() {
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        custos_persistence::SqliteTaskStore::new(&temp.path().join("test.db").to_string_lossy())
            .unwrap(),
    );
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let workflow = Arc::new(custos_runtime::workflow::TaskRuntime::new());
    let dispatcher = LocalApiDispatcher::new(service.clone(), sessions, bridge)
        .with_workspace(coordinator)
        .with_workflow(workflow);

    // Missing task_id
    let resp = dispatcher
        .handle_request(ApiRequest {
            id: "race-missing-task".into(),
            method: METHOD_WORKSPACES_CANDIDATES_RACE.into(),
            params: serde_json::json!({}),
        })
        .await;
    assert!(!resp.is_success());
    assert!(resp.error.as_deref().unwrap().contains("Missing 'task_id'"));

    // Non-existent task
    let resp = dispatcher
        .handle_request(ApiRequest {
            id: "race-nonexistent-task".into(),
            method: METHOD_WORKSPACES_CANDIDATES_RACE.into(),
            params: serde_json::json!({"task_id": "nonexistent"}),
        })
        .await;
    assert!(!resp.is_success());
    assert!(resp.error.as_deref().unwrap().contains("not found"));

    // Valid task with no candidates spawned yet
    let task = service
        .execute_create(CreateTask {
            title: "Candidate race test".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;
    let resp = dispatcher
        .handle_request(ApiRequest {
            id: "race-no-cands".into(),
            method: METHOD_WORKSPACES_CANDIDATES_RACE.into(),
            params: serde_json::json!({"task_id": task.id}),
        })
        .await;
    assert!(!resp.is_success());
    assert!(resp
        .error
        .as_deref()
        .unwrap()
        .contains("No ready candidate worktrees found"));
}

#[tokio::test]
async fn test_candidate_race_end_to_end_trial() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    let git = |dir: &std::path::Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.name", "Custos Tester"]);
    git(&repo, &["config", "user.email", "tester@custos.dev"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);

    // Create base file and a test script
    std::fs::write(repo.join("math.txt"), "0").unwrap();
    let test_script = "#!/bin/sh\nif grep -q \"10\" math.txt; then\n  echo \"test result: ok. 5 passed; 0 failed\"\n  exit 0\nelse\n  echo \"test result: FAILED. 0 passed; 1 failed\"\n  exit 1\nfi\n";
    std::fs::write(repo.join("run_tests.sh"), test_script).unwrap();
    let _ = std::process::Command::new("chmod")
        .args(["+x", repo.join("run_tests.sh").to_str().unwrap()])
        .output();

    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "initial commit"]);

    let store = Arc::new(
        custos_persistence::SqliteTaskStore::new(&temp.path().join("test.db").to_string_lossy())
            .unwrap(),
    );
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let workflow = Arc::new(
        custos_runtime::workflow::TaskRuntime::new()
            .with_workspace_coordinator(coordinator.clone())
            .with_command_runner(Arc::new(custos_adapters::operations::LocalCommandRunner)),
    );
    let dispatcher = LocalApiDispatcher::new(service.clone(), sessions, bridge)
        .with_workspace(coordinator.clone())
        .with_workflow(workflow);

    // 1. Create Task
    let task = service
        .execute_create(CreateTask {
            title: "E2E Race Trial".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;

    // 2. Register base workspace
    let mut base_ws = custos_domain::ExecutionWorkspace::new(
        custos_domain::WorkspaceId::new("base_ws"),
        "base_ws",
        custos_domain::WorkspaceKind::Git {
            repo_path: repo.to_string_lossy().into(),
            branch: "main".into(),
            base_commit: None,
        },
        repo.to_string_lossy().to_string(),
    );
    base_ws.status = custos_domain::WorkspaceStatus::Ready;
    store.save_workspace(&base_ws).await.unwrap();

    // 3. Spawn 2 Candidates via API
    let spawn_resp = dispatcher
        .handle_request(ApiRequest {
            id: "spawn_candidates".into(),
            method: METHOD_WORKSPACES_CANDIDATES_SPAWN.into(),
            params: serde_json::json!({
                "task_id": task.id,
                "base_workspace_id": "base_ws",
                "count": 2,
            }),
        })
        .await;
    assert!(spawn_resp.is_success(), "{:?}", spawn_resp.error);
    let spawned = spawn_resp.result.unwrap();
    let candidates_arr = spawned["candidates"].as_array().unwrap();
    assert_eq!(candidates_arr.len(), 2);

    let cand_1_path = std::path::PathBuf::from(candidates_arr[0]["path"].as_str().unwrap());
    let cand_2_path = std::path::PathBuf::from(candidates_arr[1]["path"].as_str().unwrap());

    // 4. Simulate Candidate 1 solving the task (writes "10" to math.txt)
    std::fs::write(cand_1_path.join("math.txt"), "10").unwrap();
    git(&cand_1_path, &["add", "math.txt"]);
    git(&cand_1_path, &["commit", "-m", "fix: math solved to 10"]);

    // Simulate Candidate 2 failing the task (writes "99" to math.txt)
    std::fs::write(cand_2_path.join("math.txt"), "99").unwrap();
    git(&cand_2_path, &["add", "math.txt"]);
    git(&cand_2_path, &["commit", "-m", "attempt: wrong value 99"]);

    // 5. Execute Candidate Race with Test Oracle
    let race_resp = dispatcher
        .handle_request(ApiRequest {
            id: "run_race".into(),
            method: METHOD_WORKSPACES_CANDIDATES_RACE.into(),
            params: serde_json::json!({
                "task_id": task.id,
                "base_workspace_id": "base_ws",
                "test_command": "./run_tests.sh",
                "timeout_seconds": 30,
            }),
        })
        .await;
    assert!(race_resp.is_success(), "{:?}", race_resp.error);
    let race_result = race_resp.result.unwrap();

    let reports = race_result["candidate_reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);

    // Candidate 1 passed tests (5 passed)
    assert_eq!(reports[0]["test_passed"], true);
    assert_eq!(reports[0]["tests_succeeded"], 5);
    assert_eq!(reports[0]["tests_failed"], 0);

    // Candidate 2 failed tests
    assert_eq!(reports[1]["test_passed"], false);
    assert_eq!(reports[1]["tests_failed"], 1);

    // Winner MUST be candidate 1
    let winner_id = race_result["winner_candidate_id"].as_str().unwrap();
    assert_eq!(winner_id, reports[0]["candidate_id"].as_str().unwrap());

    // 6. Integrate winner into main branch! (2-phase: preview then apply)
    let preview_resp = dispatcher
        .handle_request(ApiRequest {
            id: "preview_winner".into(),
            method: METHOD_WORKSPACES_INTEGRATE.into(),
            params: serde_json::json!({
                "winner_workspace_id": candidates_arr[0]["id"].as_str().unwrap(),
                "preview": true,
            }),
        })
        .await;
    assert!(preview_resp.is_success(), "{:?}", preview_resp.error);
    let preview = preview_resp.result.unwrap();

    let integrate_resp = dispatcher
        .handle_request(ApiRequest {
            id: "integrate_winner".into(),
            method: METHOD_WORKSPACES_INTEGRATE.into(),
            params: serde_json::json!({
                "winner_workspace_id": candidates_arr[0]["id"].as_str().unwrap(),
                "loser_workspace_ids": [candidates_arr[1]["id"].as_str().unwrap()],
                "expected_target_head": preview["current_target_head"],
                "expected_winner_head": preview["winner_head"],
            }),
        })
        .await;
    assert!(integrate_resp.is_success(), "{:?}", integrate_resp.error);
    let integrate_result = integrate_resp.result.unwrap();
    assert_eq!(integrate_result["merge_success"], true);

    // Verify base repository now contains Candidate 1's fix
    let merged_math = std::fs::read_to_string(repo.join("math.txt")).unwrap();
    assert_eq!(merged_math.trim(), "10");
}
