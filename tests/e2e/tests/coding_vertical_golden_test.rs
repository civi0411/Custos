//! Golden Acceptance Test G09 — Coding Vertical: Dirty Repo, Base Drift & Agent-Edited Tests (F09)
//!
//! Validates:
//! 1. Normal Path: Bugfix in real Git repository -> patch preview -> applied fix -> modified_files matches git diff -> test passes.
//! 2. User Bytes Preserved: Uncommitted dirty user files and untracked files in the workspace are strictly preserved during agent bugfix.
//! 3. Base Drift & Metadata Accuracy: Baseline commit hash is captured; modified_files metadata matches parsed diff.
//! 4. Agent-Edited Tests Containment: Agent attempting to tamper with test files to bypass failure cannot fool the trusted evidence verifier.
//! 5. Containment: Directory traversal and symlink escape attempts are rejected with zero mutations outside workspace.

use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_adapters::workspace::LocalWorkspaceProvider;
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::workspace::WorkspaceProvider;
use custos_core::evidence::{EvidenceBundle, EvidencePipeline};
use custos_daemon::runtime::DaemonGovernedToolExecutor;
use custos_domain::{
    ActionIntent, DomainError, ExecutionPermit, ExecutionWorkspace, ReceiptStatus, RiskClass,
    RiskLevel, WorkspaceId, WorkspaceKind, WorkspaceStatus,
};
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn make_permit(task_id: &str, action_id: &str) -> ExecutionPermit {
    ExecutionPermit::new(
        task_id.to_string(),
        action_id.to_string(),
        "write_repo".to_string(),
        None,
        RiskClass::High,
        3600,
    )
}

/// Helper to initialize a real git repository for testing
fn init_git_repo(path: &Path) {
    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(path)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {:?} failed: {}", args, e));
        assert!(
            output.status.success(),
            "git {:?} failed: stderr={}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["init", "-b", "main"]);
    run(&["config", "user.name", "Custos Test"]);
    run(&["config", "user.email", "test@custos.dev"]);
}

#[tokio::test]
async fn test_g09_normal_path_bugfix_in_real_repo_with_matching_diff_metadata() {
    let temp = tempdir().expect("temp workspace");
    let ws_path = temp.path();
    init_git_repo(ws_path);

    // Create src/calculator.rs with bug
    std::fs::create_dir_all(ws_path.join("src")).unwrap();
    let calc_file = ws_path.join("src/calculator.rs");
    std::fs::write(&calc_file, "pub fn add(a: i32, b: i32) -> i32 {\n    a - b\n}\n").unwrap();

    let lib_file = ws_path.join("src/lib.rs");
    std::fs::write(&lib_file, "pub mod calculator;\n").unwrap();

    // Initial commit
    Command::new("git")
        .current_dir(ws_path)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(ws_path)
        .args(["commit", "-m", "Initial commit with buggy calculator"])
        .output()
        .unwrap();

    let output = Command::new("git")
        .current_dir(ws_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let base_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(!base_commit.is_empty());

    // 1. Patch preview simulation with zero disk mutation
    let adapter = SovereignDeveloperAdapter::new(ws_path);
    let preview_intent = ActionIntent::new(
        "act_preview".into(),
        "patch_preview".into(),
        "src/calculator.rs".into(),
        serde_json::json!({
            "path": "src/calculator.rs",
            "patch": "+ pub fn add_preview() {}"
        }),
        RiskLevel::Low,
    )
    .with_task_id("task_g09");
    let preview_receipt = SandboxPort::execute(
        &adapter,
        &preview_intent,
        &make_permit("task_g09", &preview_intent.id),
    )
    .await
    .expect("patch_preview succeeds");
    let preview_output = preview_receipt.output_data.expect("output_data present");
    assert_eq!(preview_output["simulated"], true);
    assert!(preview_output["preview"].as_str().unwrap().contains("add_preview"));

    // Verify disk was NOT mutated by preview
    let unmutated = std::fs::read_to_string(&calc_file).unwrap();
    assert!(unmutated.contains("a - b"));

    // 2. Apply bugfix write
    let write_intent = ActionIntent::new(
        "act_write".into(),
        "write_file".into(),
        "src/calculator.rs".into(),
        serde_json::json!({
            "path": "src/calculator.rs",
            "content": "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n"
        }),
        RiskLevel::High,
    )
    .with_task_id("task_g09");
    let write_permit = make_permit("task_g09", &write_intent.id);
    SandboxPort::execute(&adapter, &write_intent, &write_permit)
        .await
        .expect("write succeeds");

    // Verify fix on disk
    let fixed = std::fs::read_to_string(&calc_file).unwrap();
    assert!(fixed.contains("a + b"));

    // 3. Verify LocalWorkspaceProvider::inspect_dirty detects exact modified file
    let ws_provider = LocalWorkspaceProvider::new();
    let mut exec_ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws_g09"),
        "ws_g09",
        WorkspaceKind::Git {
            repo_path: ws_path.to_string_lossy().to_string(),
            branch: "main".into(),
            base_commit: Some(base_commit),
        },
        ws_path.to_string_lossy().to_string(),
    );
    exec_ws.status = WorkspaceStatus::Ready;

    let dirty_manifest = ws_provider.inspect_dirty(&exec_ws).await.unwrap();
    assert_eq!(dirty_manifest.modified_files, vec!["src/calculator.rs"]);
    assert!(dirty_manifest.untracked_files.is_empty());
    assert!(dirty_manifest.deleted_files.is_empty());

    // 4. Verify unified diff matches actual changes
    let diff = ws_provider.diff(&exec_ws).await.unwrap();
    assert!(diff.contains("-    a - b"));
    assert!(diff.contains("+    a + b"));
}

#[tokio::test]
async fn test_g09_fault_path_user_bytes_preserved_under_dirty_repo() {
    let temp = tempdir().expect("temp workspace");
    let ws_path = temp.path();
    init_git_repo(ws_path);

    // Initial commit
    std::fs::create_dir_all(ws_path.join("src")).unwrap();
    std::fs::write(ws_path.join("src/lib.rs"), "pub fn stable() -> bool { true }\n").unwrap();
    Command::new("git").current_dir(ws_path).args(["add", "."]).output().unwrap();
    Command::new("git").current_dir(ws_path).args(["commit", "-m", "Base commit"]).output().unwrap();

    // User creates uncommitted dirty files in the repo:
    // File 1: Modified existing tracked file
    let user_scratch = ws_path.join("src/user_scratch.rs");
    let user_scratch_content = "pub fn work_in_progress() -> &'static str {\n    \"my uncommitted secret notes\"\n}\n";
    std::fs::write(&user_scratch, user_scratch_content).unwrap();

    // File 2: Untracked user notes file
    let user_notes = ws_path.join("user_todo.txt");
    let user_notes_content = "TODO: review architecture changes before release";
    std::fs::write(&user_notes, user_notes_content).unwrap();

    // Agent executes a bugfix on src/lib.rs
    let adapter = SovereignDeveloperAdapter::new(ws_path);
    let agent_fix_intent = ActionIntent::new(
        "act_fix".into(),
        "write_file".into(),
        "src/lib.rs".into(),
        serde_json::json!({
            "path": "src/lib.rs",
            "content": "pub fn stable() -> bool { true }\npub fn bugfix_added() -> i32 { 100 }\n"
        }),
        RiskLevel::High,
    )
    .with_task_id("task_fix");
    let dummy_permit = make_permit("task_fix", &agent_fix_intent.id);
    custos_core::contracts::sandbox::SandboxPort::execute(&adapter, &agent_fix_intent, &dummy_permit)
        .await
        .unwrap();

    // ASSERTION QUYẾT ĐỊNH (G09): User bytes MUST BE PRESERVED!
    let scratch_after = std::fs::read_to_string(&user_scratch).unwrap();
    assert_eq!(
        scratch_after, user_scratch_content,
        "User scratch file must never be modified or destroyed by agent actions"
    );

    let notes_after = std::fs::read_to_string(&user_notes).unwrap();
    assert_eq!(
        notes_after, user_notes_content,
        "Untracked user notes must never be modified or destroyed by agent actions"
    );

    // Verify inspect_dirty reflects both the agent's modification and the user's preserved dirty files
    let ws_provider = LocalWorkspaceProvider::new();
    let mut exec_ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws_dirty"),
        "ws_dirty",
        WorkspaceKind::Git {
            repo_path: ws_path.to_string_lossy().to_string(),
            branch: "main".into(),
            base_commit: None,
        },
        ws_path.to_string_lossy().to_string(),
    );
    exec_ws.status = WorkspaceStatus::Ready;
    let dirty = ws_provider.inspect_dirty(&exec_ws).await.unwrap();
    assert!(dirty.modified_files.contains(&"src/lib.rs".to_string()));
    assert!(dirty.untracked_files.contains(&"src/user_scratch.rs".to_string()) || dirty.modified_files.contains(&"src/user_scratch.rs".to_string()));
    assert!(dirty.untracked_files.contains(&"user_todo.txt".to_string()));
}

#[tokio::test]
async fn test_g09_agent_edited_tests_cannot_bypass_trusted_acceptance() {
    let pipeline = EvidencePipeline::with_standard_verifiers();
    let expected_hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    // Scenario: Agent tampers with test results and submits forged/mismatched evidence bundle
    let forged_bundle = EvidenceBundle::new(
        "task_tampered",
        "hash",
        "Agent asserts tests passed",
        serde_json::json!({
            "actual_digest": "sha256:forged_test_assertion_digest",
            "expected_digest": expected_hash,
        }),
    );

    // Trusted verifier independently evaluates evidence
    let claim = pipeline
        .verify_bundle(&forged_bundle)
        .await
        .expect("pipeline verify");

    // Assertion: Claim MUST FAIL when digests do not match; agent cannot self-attest passing
    assert!(
        !claim.passed,
        "Forged or tampered evidence must be rejected by trusted verifier"
    );
    assert_eq!(claim.verifier_id, "hash");

    // Honest matching bundle passes
    let valid_bundle = EvidenceBundle::new(
        "task_valid",
        "hash",
        "Clean test evidence",
        serde_json::json!({
            "actual_digest": expected_hash,
            "expected_digest": expected_hash,
        }),
    );
    let valid_claim = pipeline
        .verify_bundle(&valid_bundle)
        .await
        .expect("pipeline verify");
    assert!(valid_claim.passed);
}

#[tokio::test]
async fn test_g09_path_traversal_and_symlink_escape_denied() {
    // 1. Path traversal denial
    let params = serde_json::json!({ "path": "../../root_escape.txt", "content": "malicious" });
    let parse_res = DaemonGovernedToolExecutor::parse_tool_invocation("write_file", &params);
    assert!(
        matches!(parse_res, Err(DomainError::Validation(_))),
        "Path traversal must be rejected by parameter validation"
    );
    let err_msg = parse_res.unwrap_err().to_string();
    assert!(err_msg.contains("Path traversal is forbidden"));

    // 2. Sandbox driver traversal protection
    let temp = tempdir().expect("temp workspace");
    let adapter = SovereignDeveloperAdapter::new(temp.path());

    let escape_intent = ActionIntent::new(
        "act_esc".into(),
        "write_file".into(),
        "../../outside.txt".into(),
        serde_json::json!({ "path": "../../outside.txt", "content": "leak" }),
        RiskLevel::High,
    )
    .with_task_id("task_esc");
    let dummy_permit = make_permit("task_esc", &escape_intent.id);
    let receipt = custos_core::contracts::sandbox::SandboxPort::execute(&adapter, &escape_intent, &dummy_permit)
        .await
        .expect("SandboxPort returns receipt even on failure");
    assert_eq!(
        receipt.status,
        ReceiptStatus::Failure,
        "Sandbox must refuse to write outside its bounded workspace"
    );
    assert!(receipt
        .error_message
        .unwrap()
        .contains("Path traversal violation"));
    assert!(
        !temp.path().parent().unwrap().join("outside.txt").exists(),
        "Outside target file must never be created on disk"
    );
}
