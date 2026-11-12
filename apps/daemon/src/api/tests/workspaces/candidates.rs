use std::sync::Arc;

use custos_core::contracts::WorkspaceRepository;
use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;

use crate::api::*;

#[tokio::test]
async fn candidate_creation_and_integration_replay_use_recorded_workspace_state() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repository");
    std::fs::create_dir(&repo).unwrap();
    let git = |path: &std::path::Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args([
                "-c",
                "user.name=Custos Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    git(&repo, &["init", "-b", "main"]);
    std::fs::write(repo.join("file.txt"), "base").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "base"]);
    // Merge commands inherit only fixture-local identity, never the user's repo configuration.
    git(&repo, &["config", "user.name", "Custos Test"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    let store = Arc::new(
        custos_persistence::SqliteTaskStore::new(&temp.path().join("test.db").to_string_lossy())
            .unwrap(),
    );
    let service = Arc::new(TaskService::new(store.clone()));
    let task = service
        .execute_create(CreateTask {
            title: "Candidate integration test".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let mut base = custos_domain::ExecutionWorkspace::new(
        custos_domain::WorkspaceId::new("base"),
        "base",
        custos_domain::WorkspaceKind::Git {
            repo_path: repo.to_string_lossy().into(),
            branch: "main".into(),
            base_commit: None,
        },
        repo.to_string_lossy().to_string(),
    );
    base.status = custos_domain::WorkspaceStatus::Ready;
    store.save_workspace(&base).await.unwrap();
    let make_dispatcher = || {
        LocalApiDispatcher::new(service.clone(), sessions.clone(), bridge.clone())
            .with_workspace(coordinator.clone())
    };
    let dispatcher = make_dispatcher();
    let spawn_params = serde_json::json!({"task_id": task.id, "base_workspace_id": "base", "count": 1, "command_id": "create-candidates-once"});
    let request = |method: &str, params: serde_json::Value| ApiRequest {
        id: "fixture-request".into(),
        method: method.into(),
        params,
    };
    let first = dispatcher
        .handle_request(request(
            METHOD_WORKSPACES_CANDIDATES_SPAWN,
            spawn_params.clone(),
        ))
        .await;
    assert!(first.is_success(), "{:?}", first.error);
    let first = first.result.unwrap();
    let winner_id = first["candidates"][0]["id"].as_str().unwrap().to_string();
    let candidate = std::path::PathBuf::from(first["candidates"][0]["path"].as_str().unwrap());
    let second = make_dispatcher()
        .handle_request(request(
            METHOD_WORKSPACES_CANDIDATES_SPAWN,
            spawn_params.clone(),
        ))
        .await;
    assert!(second.is_success(), "{:?}", second.error);
    assert_eq!(second.result.unwrap()["candidates"][0]["id"], winner_id);
    assert_eq!(store.list_workspaces().await.unwrap().len(), 2);
    let mut different = spawn_params;
    different["count"] = serde_json::json!(2);
    assert!(!dispatcher
        .handle_request(request(METHOD_WORKSPACES_CANDIDATES_SPAWN, different))
        .await
        .is_success());
    std::fs::write(candidate.join("file.txt"), "candidate").unwrap();
    git(&candidate, &["add", "."]);
    git(&candidate, &["commit", "-m", "candidate"]);
    let before = git(&repo, &["rev-parse", "HEAD"]);
    assert!(!dispatcher
        .handle_request(request(
            METHOD_WORKSPACES_INTEGRATE,
            serde_json::json!({"winner_workspace_id": winner_id})
        ))
        .await
        .is_success());
    let preview = dispatcher
        .handle_request(request(
            METHOD_WORKSPACES_INTEGRATE,
            serde_json::json!({"winner_workspace_id": winner_id, "preview": true}),
        ))
        .await;
    assert!(preview.is_success(), "{:?}", preview.error);
    let preview = preview.result.unwrap();
    assert_eq!(
        git(&repo, &["rev-parse", "HEAD"]),
        before,
        "preview must not merge"
    );
    let apply = serde_json::json!({"winner_workspace_id": winner_id, "expected_target_head": preview["current_target_head"], "expected_winner_head": preview["winner_head"]});
    let mut stale = apply.clone();
    stale["expected_winner_head"] = serde_json::json!("stale");
    assert!(!dispatcher
        .handle_request(request(METHOD_WORKSPACES_INTEGRATE, stale))
        .await
        .is_success());
    let integrated = dispatcher
        .handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply.clone()))
        .await;
    assert!(integrated.is_success(), "{:?}", integrated.error);
    assert_eq!(integrated.result.unwrap()["merge_success"], true);
    let merged_head = git(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(merged_head, before);
    let repeated = make_dispatcher()
        .handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply.clone()))
        .await;
    assert!(repeated.is_success());
    assert_eq!(repeated.result.unwrap()["merge_success"], true);
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), merged_head);
    let mut interrupted = store
        .get_workspace(&custos_domain::WorkspaceId::new(&winner_id))
        .await
        .unwrap()
        .unwrap();
    interrupted.metadata["integration"]
        .as_object_mut()
        .unwrap()
        .remove("result");
    store.save_workspace(&interrupted).await.unwrap();
    // Recovery follows the recorded target even when the user switches checkout.
    git(&repo, &["checkout", "-b", "unrelated", &before]);
    let reconciled = make_dispatcher()
        .handle_request(request(METHOD_WORKSPACES_INTEGRATE, apply))
        .await;
    assert!(reconciled.is_success());
    assert_eq!(
        reconciled.result.unwrap()["merge_status"],
        "merged_reconciled"
    );
    assert_eq!(git(&repo, &["rev-parse", "main"]), merged_head);
    assert_eq!(
        git(&repo, &["rev-parse", "HEAD"]),
        before,
        "recovery must not change the user's checkout"
    );
    assert!(
        candidate.exists(),
        "integration never removes candidate checkout"
    );
}

#[tokio::test]
async fn two_worktree_concurrent_isolation_and_stale_base_rejection() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("main_repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |path: &std::path::Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args([
                "-c",
                "user.name=Custos Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    git(&repo, &["init", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# Initial Main\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "initial commit"]);
    git(&repo, &["config", "user.name", "Custos Test"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);

    let store = Arc::new(
        custos_persistence::SqliteTaskStore::new(&temp.path().join("two_wt.db").to_string_lossy())
            .unwrap(),
    );
    let service = Arc::new(TaskService::new(store.clone()));
    let task = service
        .execute_create(CreateTask {
            title: "Two worktree isolation test".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let mut base = custos_domain::ExecutionWorkspace::new(
        custos_domain::WorkspaceId::new("base"),
        "base",
        custos_domain::WorkspaceKind::Git {
            repo_path: repo.to_string_lossy().into(),
            branch: "main".into(),
            base_commit: None,
        },
        repo.to_string_lossy().to_string(),
    );
    base.status = custos_domain::WorkspaceStatus::Ready;
    store.save_workspace(&base).await.unwrap();

    let dispatcher = LocalApiDispatcher::new(service.clone(), sessions.clone(), bridge.clone())
        .with_workspace(coordinator.clone());

    // Spawn 2 candidate worktrees concurrently
    let spawn_params = serde_json::json!({
        "task_id": task.id,
        "base_workspace_id": "base",
        "count": 2,
        "command_id": "two-worktree-spawn"
    });
    let spawn_res = dispatcher
        .handle_request(ApiRequest {
            id: "spawn-two-wt".into(),
            method: METHOD_WORKSPACES_CANDIDATES_SPAWN.into(),
            params: spawn_params,
        })
        .await;
    assert!(spawn_res.is_success(), "{:?}", spawn_res.error);
    let candidates = spawn_res.result.unwrap()["candidates"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        candidates.len(),
        2,
        "Must spawn exactly 2 candidate worktrees"
    );

    let wt_a_id = candidates[0]["id"].as_str().unwrap().to_string();
    let wt_a_path = std::path::PathBuf::from(candidates[0]["path"].as_str().unwrap());
    let wt_b_id = candidates[1]["id"].as_str().unwrap().to_string();
    let wt_b_path = std::path::PathBuf::from(candidates[1]["path"].as_str().unwrap());

    assert_ne!(wt_a_id, wt_b_id, "Worktree IDs must be distinct");
    assert_ne!(wt_a_path, wt_b_path, "Worktree file paths must be isolated");

    // 1. Mutate Worktree A
    std::fs::write(wt_a_path.join("feature_a.txt"), "implemented A").unwrap();
    git(&wt_a_path, &["add", "."]);
    git(&wt_a_path, &["commit", "-m", "feature A"]);

    // Verify Worktree B is completely isolated from A's changes
    assert!(
        !wt_b_path.join("feature_a.txt").exists(),
        "Worktree B must not contain Worktree A uncommitted/committed files"
    );
    let wt_b_status = git(&wt_b_path, &["status", "--porcelain"]);
    assert!(
        wt_b_status.is_empty(),
        "Worktree B must remain completely clean after Worktree A commit"
    );

    // 2. Mutate Worktree B
    std::fs::write(wt_b_path.join("feature_b.txt"), "implemented B").unwrap();
    git(&wt_b_path, &["add", "."]);
    git(&wt_b_path, &["commit", "-m", "feature B"]);

    // Verify Worktree A is isolated from B's changes
    assert!(
        !wt_a_path.join("feature_b.txt").exists(),
        "Worktree A must not contain Worktree B files"
    );

    // 3. Integrate Worktree A into main
    let prev_target_head = git(&repo, &["rev-parse", "HEAD"]);
    let preview_a = dispatcher
        .handle_request(ApiRequest {
            id: "preview-a".into(),
            method: METHOD_WORKSPACES_INTEGRATE.into(),
            params: serde_json::json!({
                "winner_workspace_id": wt_a_id,
                "preview": true
            }),
        })
        .await;
    assert!(preview_a.is_success(), "{:?}", preview_a.error);
    let preview_a_val = preview_a.result.unwrap();

    let integrate_a = dispatcher
        .handle_request(ApiRequest {
            id: "integrate-a".into(),
            method: METHOD_WORKSPACES_INTEGRATE.into(),
            params: serde_json::json!({
                "winner_workspace_id": wt_a_id,
                "expected_target_head": preview_a_val["current_target_head"],
                "expected_winner_head": preview_a_val["winner_head"]
            }),
        })
        .await;
    assert!(integrate_a.is_success(), "{:?}", integrate_a.error);
    assert_eq!(integrate_a.result.unwrap()["merge_success"], true);

    let new_target_head = git(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(
        new_target_head, prev_target_head,
        "Target HEAD must advance after merging Worktree A"
    );
    assert!(repo.join("feature_a.txt").exists());

    // 4. Attempt to integrate Worktree B directly: MUST fail due to stale base commit!
    // Worktree B was branched from prev_target_head, but repo is now at new_target_head.
    let integrate_b_preview = dispatcher
        .handle_request(ApiRequest {
            id: "preview-b".into(),
            method: METHOD_WORKSPACES_INTEGRATE.into(),
            params: serde_json::json!({
                "winner_workspace_id": wt_b_id,
                "preview": true
            }),
        })
        .await;
    assert!(
        !integrate_b_preview.is_success(),
        "Integrating Worktree B with stale base must fail closed"
    );
    let error_msg = integrate_b_preview.error.unwrap_or_default();
    assert!(
        error_msg.contains("stale") || error_msg.contains("does not match target HEAD"),
        "Error message must clearly identify stale base drift: {error_msg}"
    );
}

#[tokio::test]
async fn candidate_comparison_rejects_mixed_scope_and_reports_inspection_failure() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new()),
    ));
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_workspace(coordinator);
    let make = |id: &str, task: &str, base: &str, repo: &str| {
        let mut workspace = custos_domain::ExecutionWorkspace::new(
            custos_domain::WorkspaceId::new(id),
            id.to_string(),
            custos_domain::WorkspaceKind::Git {
                repo_path: repo.into(),
                branch: id.into(),
                base_commit: Some(base.into()),
            },
            format!("/missing/candidate/{id}"),
        );
        workspace.status = custos_domain::WorkspaceStatus::Ready;
        workspace.owner_task_id = Some(task.into());
        workspace.base_commit_hash = Some(base.into());
        workspace
    };
    for workspace in [
        make("a", "task-a", "base-a", "/repo-a"),
        make("b", "task-b", "base-a", "/repo-a"),
        make("c", "task-a", "base-c", "/repo-a"),
        make("d", "task-a", "base-a", "/repo-d"),
    ] {
        store.save_workspace(&workspace).await.unwrap();
    }
    for ids in [
        serde_json::json!(["a", "missing"]),
        serde_json::json!(["a", "b"]),
        serde_json::json!(["a", "c"]),
        serde_json::json!(["a", "d"]),
        serde_json::json!(["a", "a"]),
        serde_json::json!(["a", 42]),
    ] {
        let response = dispatcher
            .handle_request(ApiRequest {
                id: "compare".into(),
                method: METHOD_WORKSPACES_CANDIDATES_COMPARE.into(),
                params: serde_json::json!({"workspace_ids": ids}),
            })
            .await;
        assert!(
            !response.is_success(),
            "comparison must reject mismatched or invalid candidates"
        );
    }
    let response = dispatcher
        .handle_request(ApiRequest {
            id: "inspect".into(),
            method: METHOD_WORKSPACES_CANDIDATES_COMPARE.into(),
            params: serde_json::json!({"workspace_ids": ["a"]}),
        })
        .await;
    assert!(response.is_success());
    let candidate = &response.result.unwrap()["candidates"][0];
    assert_eq!(candidate["is_ready_for_integration"], false);
    assert!(candidate["inspection_error"].is_string());
}
