use super::super::*;
use std::sync::Arc;
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
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
    async fn two_worktree_concurrent_isolation_and_stale_base_rejection() {
        use custos_core::contracts::WorkspaceRepository;
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
            custos_persistence::SqliteTaskStore::new(
                &temp.path().join("two_wt.db").to_string_lossy(),
            )
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
        assert_eq!(candidates.len(), 2, "Must spawn exactly 2 candidate worktrees");

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
        let terminal = TerminalCoordinator::new(std::sync::Arc::new(
            custos_adapters::terminal::NativePtyDriver,
        ));
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
