use super::*;
use custos_domain::WorkspaceId;
use std::path::Path;
use tempfile::tempdir;

#[tokio::test]
async fn test_folder_workspace_provision_setup_teardown() {
    let tmp = tempdir().unwrap();
    let folder_path = tmp.path().join("research_run");
    let path_str = folder_path.to_str().unwrap().to_string();

    let provider = LocalWorkspaceProvider::new();
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "research-experiment-1",
        WorkspaceKind::Folder {
            path: path_str.clone(),
        },
        path_str.clone(),
    );

    // 1. Validate
    assert!(provider.validate(&ws.kind, &ws.path).await.is_ok());

    // 2. Provision
    assert!(!folder_path.exists());
    provider.provision(&ws).await.unwrap();
    assert!(folder_path.exists());
    assert!(folder_path.is_dir());

    // 3. Setup hook
    let setup_script = "echo 'ready' > status.txt";
    provider.setup(&ws, Some(setup_script)).await.unwrap();
    assert!(folder_path.join("status.txt").exists());

    // 4. Teardown
    provider.teardown(&ws).await.unwrap();
    assert!(!folder_path.exists());
}

#[tokio::test]
async fn test_validate_empty_path_fails() {
    let provider = LocalWorkspaceProvider::new();
    let kind = WorkspaceKind::Folder { path: "".into() };
    assert!(provider.validate(&kind, "  ").await.is_err());
}

async fn run_git_cmd(args: &[&str], cwd: &Path) {
    let mut c = Command::new("git");
    c.args(args).current_dir(cwd);
    c.output().await.unwrap();
}

#[tokio::test]
async fn existing_checkout_registration_checks_actual_branch_and_root() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).await.unwrap();
    run_git_cmd(&["init", "--initial-branch=actual"], &repo).await;
    let provider = LocalWorkspaceProvider::new();
    let make = |path: &Path, branch: &str| {
        ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "existing",
            WorkspaceKind::Git {
                repo_path: path.to_string_lossy().into_owned(),
                branch: branch.into(),
                base_commit: None,
            },
            path.to_string_lossy().into_owned(),
        )
    };
    assert!(provider.provision(&make(&repo, "actual")).await.is_ok());
    assert!(provider.provision(&make(&repo, "main")).await.is_err());
    let nested = repo.join("nested");
    fs::create_dir_all(&nested).await.unwrap();
    assert!(provider.provision(&make(&nested, "actual")).await.is_err());
    let folder = tmp.path().join("not-git");
    fs::create_dir_all(&folder).await.unwrap();
    assert!(provider.provision(&make(&folder, "main")).await.is_err());
}

#[tokio::test]
async fn linked_worktree_rejects_invalid_branch_and_base_without_creating_checkout() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).await.unwrap();
    run_git_cmd(&["init"], &repo).await;
    run_git_cmd(
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "base",
        ],
        &repo,
    )
    .await;
    let provider = LocalWorkspaceProvider::new();
    for (index, (branch, base)) in [
        ("--detach", None),
        ("bad..branch", None),
        ("@{-1}", None),
        ("feature", Some("--help")),
        ("feature", Some("missing-ref")),
    ]
    .into_iter()
    .enumerate()
    {
        let target = tmp.path().join(format!("candidate-{index}"));
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "candidate",
            WorkspaceKind::Git {
                repo_path: repo.to_string_lossy().into_owned(),
                branch: branch.into(),
                base_commit: base.map(str::to_owned),
            },
            target.to_string_lossy().into_owned(),
        );
        assert!(provider.provision(&ws).await.is_err());
        assert!(!target.exists());
    }
}

#[tokio::test]
async fn test_git_worktree_provision_and_teardown() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(&repo_dir).await.unwrap();

    run_git_cmd(&["init"], &repo_dir).await;
    run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
    run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
    let dummy_file = repo_dir.join("README.md");
    fs::write(&dummy_file, b"initial").await.unwrap();
    run_git_cmd(&["add", "README.md"], &repo_dir).await;
    run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

    let worktree_dir = tmp.path().join("worktree_1");
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "git-worktree-test",
        WorkspaceKind::Git {
            repo_path: repo_dir.to_str().unwrap().into(),
            branch: "feat-branch".into(),
            base_commit: None,
        },
        worktree_dir.to_str().unwrap(),
    );

    let provider = LocalWorkspaceProvider::new();
    provider.validate(&ws.kind, &ws.path).await.unwrap();
    provider.provision(&ws).await.unwrap();

    assert!(worktree_dir.exists());
    assert!(worktree_dir.join("README.md").exists());

    // Teardown
    provider.teardown(&ws).await.unwrap();
    assert!(!worktree_dir.exists());
}

#[tokio::test]
async fn test_git_worktree_teardown_refuses_when_dirty() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(&repo_dir).await.unwrap();

    run_git_cmd(&["init"], &repo_dir).await;
    run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
    run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
    let dummy_file = repo_dir.join("README.md");
    fs::write(&dummy_file, b"initial").await.unwrap();
    run_git_cmd(&["add", "README.md"], &repo_dir).await;
    run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

    let worktree_dir = tmp.path().join("worktree_dirty");
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "git-worktree-dirty-test",
        WorkspaceKind::Git {
            repo_path: repo_dir.to_str().unwrap().into(),
            branch: "dirty-branch".into(),
            base_commit: None,
        },
        worktree_dir.to_str().unwrap(),
    );

    let provider = LocalWorkspaceProvider::new();
    provider.validate(&ws.kind, &ws.path).await.unwrap();
    provider.provision(&ws).await.unwrap();

    assert!(worktree_dir.exists());

    // Introduce untracked dirty file
    let dirty_file = worktree_dir.join("scratch.txt");
    fs::write(&dirty_file, b"uncommitted changes")
        .await
        .unwrap();

    // Teardown must be refused safely
    let res = provider.teardown(&ws).await;
    assert!(res.is_err());
    assert!(worktree_dir.exists());
    assert!(dirty_file.exists());
}

#[tokio::test]
async fn test_git_worktree_provision_refuses_resetting_existing_branch_with_base_commit() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(&repo_dir).await.unwrap();

    run_git_cmd(&["init"], &repo_dir).await;
    run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
    run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
    let dummy_file = repo_dir.join("README.md");
    fs::write(&dummy_file, b"initial").await.unwrap();
    run_git_cmd(&["add", "README.md"], &repo_dir).await;
    run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

    // Create an existing branch "feature-existing"
    run_git_cmd(&["branch", "feature-existing"], &repo_dir).await;

    let worktree_dir = tmp.path().join("worktree_clobber");
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "git-worktree-clobber-test",
        WorkspaceKind::Git {
            repo_path: repo_dir.to_str().unwrap().into(),
            branch: "feature-existing".into(),
            base_commit: Some("HEAD".into()),
        },
        worktree_dir.to_str().unwrap(),
    );

    let provider = LocalWorkspaceProvider::new();
    // Allocation must be refused because feature-existing already exists and base_commit was specified
    let res = provider.provision(&ws).await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(err_str.contains("refusing to reset branch to base_commit"));
}

#[tokio::test]
async fn test_git_worktree_diff_and_porcelain_z() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(&repo_dir).await.unwrap();

    run_git_cmd(&["init"], &repo_dir).await;
    run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
    run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
    let dummy_file = repo_dir.join("README.md");
    fs::write(&dummy_file, b"line 1\n").await.unwrap();
    run_git_cmd(&["add", "README.md"], &repo_dir).await;
    run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

    let worktree_dir = tmp.path().join("worktree_diff");
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "git-worktree-diff-test",
        WorkspaceKind::Git {
            repo_path: repo_dir.to_str().unwrap().into(),
            branch: "diff-branch".into(),
            base_commit: None,
        },
        worktree_dir.to_str().unwrap(),
    );

    let provider = LocalWorkspaceProvider::new();
    provider.provision(&ws).await.unwrap();

    // Modify README in worktree
    let wt_file = worktree_dir.join("README.md");
    fs::write(&wt_file, b"line 1\nline 2 added\n")
        .await
        .unwrap();

    // Inspect dirty
    let dirty = provider.inspect_dirty(&ws).await.unwrap();
    assert!(dirty.is_dirty);
    assert_eq!(dirty.modified_files, vec!["README.md"]);

    // Diff
    let diff = provider.diff(&ws).await.unwrap();
    assert!(diff.contains("+line 2 added"));
}

#[tokio::test]
async fn test_folder_workspace_teardown_refuses_shallow_path() {
    let provider = LocalWorkspaceProvider::new();
    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "root-dir-test",
        WorkspaceKind::Folder {
            path: "/tmp".into(),
        },
        "/tmp",
    );

    let res = provider.teardown(&ws).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("shallow"));
}
