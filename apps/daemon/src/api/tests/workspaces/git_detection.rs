use custos_adapters::host::workspace::git_discovery::{
    github_auth_metadata, github_login_command, parse_github_remote, resolve_deep_git_paths,
};
use custos_core::contracts::TerminalPort;
use custos_runtime::terminal::TerminalCoordinator;

use crate::api::*;

#[test]
fn github_status_requires_successful_verified_viewer_and_never_exposes_secrets() {
    for (success, body) in [
        (false, br#"{"login":"someone","token":"secret"}"#.as_slice()),
        (true, b"not json".as_slice()),
        (true, br#"{"login":""}"#.as_slice()),
        (true, br#"{"message":"Bad credentials"}"#.as_slice()),
    ] {
        let result = github_auth_metadata(success, body);
        assert_eq!(result["authenticated"], false);
        assert!(result.get("token").is_none());
    }
    let result = github_auth_metadata(
        true,
        br#"{"login":"verified","name":"Human","token":"secret","other":"private"}"#,
    );
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
    let session = terminal
        .spawn("github-auth", directory.path(), Some(&command), 100, 28)
        .await
        .unwrap();
    let mut output = String::new();
    let mut seq = 0;
    for _ in 0..100 {
        let chunk = terminal
            .read_output(session.id.as_str(), seq, 16384)
            .await
            .unwrap();
        seq = chunk.next_seq;
        output.push_str(&chunk.data);
        if output.contains("Press Enter") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        output.contains("TEST-CODE"),
        "Real PTY should expose the device code"
    );
    terminal.write(session.id.as_str(), b"\r").await.unwrap();
    for _ in 0..100 {
        let chunk = terminal
            .read_output(session.id.as_str(), seq, 16384)
            .await
            .unwrap();
        seq = chunk.next_seq;
        output.push_str(&chunk.data);
        if output.contains("Browser approval pending") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(output.contains("Browser approval pending"));
    terminal.terminate(session.id.as_str()).await.unwrap();
    assert!(matches!(
        terminal
            .get_session(session.id.as_str())
            .await
            .unwrap()
            .status,
        custos_domain::TerminalSessionStatus::Terminated
    ));
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
    let (host, owner, repo, avatar) =
        parse_github_remote("https://github.com/civi0411/AgentHub.git");
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
    let (host, owner, repo, avatar) =
        parse_github_remote("git@ghe.corporate.com:ai-team/core-model.git");
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
    tokio::fs::write(wt_gitdir.join("commondir"), "../..\n")
        .await
        .unwrap();

    let dot_git_file = wt_path.join(".git");
    tokio::fs::write(&dot_git_file, format!("gitdir: {}\n", wt_gitdir.display()))
        .await
        .unwrap();

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
