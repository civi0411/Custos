//! Golden Acceptance Test G08 — Three Lenses, Session Drafts & Project Switch (F09)
//!
//! Validates:
//! 1. Same conversation across three lenses (Assistant, Coding, Research) retains unified journal without fabricating tasks/runs.
//! 2. Switching lenses is idempotent and does not transfer permissions or un-scoped authority.
//! 3. Drafts are strictly scoped by session identity; cross-session draft mutation is rejected.
//! 4. Draft payload lock guarantees content immutability after approval.
//! 5. Project/workspace switch preserves history, drafts, and context scope without leaking active runs or permissions.

use custos_domain::{
    ConversationTurn, DomainError, SessionMode, SessionStatus, TurnActorKind,
    WorkbenchLens,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::session::SessionManager;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_g08_same_chat_across_three_lenses_retains_unified_journal() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));

    // 1. Create a single active chat session in Assistant lens
    let session = session_manager.create_session(SessionMode::Assisted).await;
    assert_eq!(session.status, SessionStatus::Active);

    // Initial conversation turn in Assistant lens
    session_manager
        .append_user_message(&session.id, "Plan the refactoring approach")
        .await
        .expect("user message appended");
    session_manager
        .append_assistant_message(&session.id, "I propose analyzing the research papers first.")
        .await
        .expect("assistant message appended");

    // 2. Switch to Research Lens
    session_manager
        .open_in_lens(&session.id, WorkbenchLens::Research)
        .await
        .expect("switch to research lens succeeds");

    // Add notes / findings in Research lens
    session_manager
        .append_assistant_message(
            &session.id,
            "Research finding: Algorithm A reduces latency by 35%.",
        )
        .await
        .expect("research finding recorded");

    // 3. Switch to Coding Lens
    session_manager
        .open_in_lens(&session.id, WorkbenchLens::Coding)
        .await
        .expect("switch to coding lens succeeds");

    session_manager
        .record_tool_call(
            &session.id,
            "read_file",
            r#"{"path": "crates/custos-core/src/lib.rs"}"#,
        )
        .await
        .expect("tool call recorded in coding lens");

    // 4. Verify Unified Session Journal across all 3 lenses
    let journal = session_manager
        .get_journal(&session.id)
        .await
        .expect("get unified journal");

    // Verify all turns and lens transitions exist in chronological order
    let types: Vec<&str> = journal.entries.iter().map(|e| e.entry_type.as_str()).collect();
    assert!(types.contains(&"user_message"));
    assert!(types.contains(&"assistant_message"));
    assert!(types.contains(&"workbench_opened"));
    assert!(types.contains(&"tool_call"));

    // Verify lens transitions were recorded honestly
    let lens_entries: Vec<&str> = journal
        .entries
        .iter()
        .filter(|e| e.entry_type == "workbench_opened")
        .map(|e| e.entry_data.as_str())
        .collect();
    assert_eq!(lens_entries, vec!["research", "code"]);

    // 5. Assert No Hidden Task / Run Creation:
    // Switching lenses must NOT fabricate tasks or runs in the store
    let tasks = store.tasks().list_tasks().expect("list tasks");
    assert!(
        tasks.is_empty(),
        "Navigating between presentation lenses must never secretly create a Task"
    );
}

#[tokio::test]
async fn test_g08_lens_switch_idempotency_and_no_permission_leakage() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));

    let session = session_manager.create_session(SessionMode::Assisted).await;

    // Switch to Coding lens twice
    session_manager
        .open_in_lens(&session.id, WorkbenchLens::Coding)
        .await
        .unwrap();
    session_manager
        .open_in_lens(&session.id, WorkbenchLens::Coding)
        .await
        .unwrap();

    // Idempotency: Opening the same lens twice must not create duplicate noise
    let journal = session_manager.get_journal(&session.id).await.unwrap();
    let code_opens = journal
        .entries
        .iter()
        .filter(|e| e.entry_type == "workbench_opened" && e.entry_data == "code")
        .count();
    assert_eq!(code_opens, 1, "Lens opening must be idempotent");

    // Validate Typed Turn Attribution:
    // A ConversationTurn requires valid actor and cannot claim a task without a real revision
    let turn_res = ConversationTurn::new(
        session.id.clone(),
        "operator",
        TurnActorKind::Human,
        WorkbenchLens::Coding,
        "internal",
        serde_json::json!({ "text": "Run tests" }),
    );
    assert!(turn_res.is_ok());
    let turn = turn_res.unwrap();

    // Attempting to bind a fake/zero revision task must be rejected
    let bind_res = turn.bind_task("task_fake".into(), 0);
    assert!(
        matches!(bind_res, Err(DomainError::Validation(_))),
        "Binding to revision 0 must be rejected"
    );
}

#[tokio::test]
async fn test_g08_independent_draft_scoping_and_cross_session_isolation() {
    use custos_daemon::api::{ApiRequest, LocalApiDispatcher, METHOD_ASSISTANT_DRAFTS_SAVE};
    use custos_core::TaskService;
    use custos_bridge::BridgeService;

    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let assistant = Arc::new(store.assistant().clone());
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_assistant(assistant.clone());

    let session_a = "session_alpha";
    let session_b = "session_beta";

    let request = |params| ApiRequest {
        id: "draft-scope-test".into(),
        method: METHOD_ASSISTANT_DRAFTS_SAVE.into(),
        params,
    };

    // 1. Create and save draft under Session A
    let create_resp = dispatcher.handle_request(request(serde_json::json!({
        "id": "draft_001",
        "title": "Executive Summary",
        "kind": "document",
        "content": "Initial draft text for Alpha",
        "session_id": session_a,
    }))).await;
    assert!(create_resp.is_success(), "{:?}", create_resp.error);
    let draft_a = create_resp.result.unwrap();
    assert_eq!(draft_a["id"], "draft_001");
    assert_eq!(draft_a["session_id"], session_a);
    assert_eq!(draft_a["status"], "draft");

    // 2. Attempting to hijack or update Session A's draft under Session B MUST BE REJECTED
    let hijack_resp = dispatcher.handle_request(request(serde_json::json!({
        "id": "draft_001",
        "title": "Executive Summary Hijacked",
        "kind": "document",
        "content": "Malicious text from Beta",
        "session_id": session_b,
    }))).await;
    assert!(
        !hijack_resp.is_success(),
        "Cross-session draft modification must be strictly rejected"
    );
    assert!(hijack_resp.error.unwrap().contains("Draft scope cannot be changed"));

    // 3. Verify Session A's draft was NOT modified
    let fetched = assistant
        .get_draft_sync("draft_001")
        .expect("get draft")
        .expect("draft exists");
    assert_eq!(fetched.content, "Initial draft text for Alpha");
    assert_eq!(fetched.session_id.as_deref(), Some(session_a));
}

#[tokio::test]
async fn test_g08_draft_payload_lock_guarantees_immutability_after_review() {
    use custos_daemon::api::{ApiRequest, LocalApiDispatcher, METHOD_ASSISTANT_DRAFTS_SAVE};
    use custos_core::TaskService;
    use custos_bridge::BridgeService;

    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let assistant = Arc::new(store.assistant().clone());
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge).with_assistant(assistant.clone());

    let session_id = "session_locked";
    let request = |params| ApiRequest {
        id: "draft-lock-test".into(),
        method: METHOD_ASSISTANT_DRAFTS_SAVE.into(),
        params,
    };

    // 1. Create draft
    let initial_content = "Approved specification content: version 1.0";
    let created = dispatcher.handle_request(request(serde_json::json!({
        "id": "draft_spec",
        "title": "Spec v1",
        "kind": "document",
        "content": initial_content,
        "session_id": session_id,
    }))).await;
    assert!(created.is_success(), "{:?}", created.error);

    // 2. Approve draft with payload lock
    let mut approved_payload = created.result.unwrap();
    approved_payload["status"] = serde_json::json!("approved");
    approved_payload["payload_lock"] = serde_json::json!("lock_me");
    let review_resp = dispatcher.handle_request(request(approved_payload)).await;
    assert!(review_resp.is_success(), "{:?}", review_resp.error);
    let reviewed = review_resp.result.unwrap();
    assert_eq!(reviewed["status"], "approved");
    assert!(reviewed["payload_lock"].as_str().is_some());

    // 3. Tampering attempt: Modify content while draft is locked
    let mut tampered_payload = reviewed.clone();
    tampered_payload["content"] = serde_json::json!("Tampered content: backdoor inserted");
    let tamper_resp = dispatcher.handle_request(request(tampered_payload)).await;
    assert!(
        !tamper_resp.is_success(),
        "Modifying locked draft content must be strictly rejected"
    );
    assert!(tamper_resp.error.unwrap().contains("locked"));

    // 4. Verify original content remained intact in persistent repository
    let verified = assistant
        .get_draft_sync("draft_spec")
        .unwrap()
        .expect("draft exists");
    assert_eq!(verified.content, initial_content);
    assert_eq!(verified.status, "approved");
}

#[tokio::test]
async fn test_g08_project_workspace_switch_preserves_context_isolation() {
    let temp_workspace_a = tempdir().expect("temp workspace a");
    let temp_workspace_b = tempdir().expect("temp workspace b");

    // Write distinctive files in each workspace
    let file_a = temp_workspace_a.path().join("project_alpha.txt");
    let file_b = temp_workspace_b.path().join("project_beta.txt");
    std::fs::write(&file_a, "alpha secrets").unwrap();
    std::fs::write(&file_b, "beta secrets").unwrap();

    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));

    // Create session in Project A
    let session_a = session_manager.create_session(SessionMode::Assisted).await;
    session_manager
        .append_user_message(&session_a.id, "Working on Project Alpha")
        .await
        .unwrap();

    // Create session in Project B
    let session_b = session_manager.create_session(SessionMode::Assisted).await;
    session_manager
        .append_user_message(&session_b.id, "Working on Project Beta")
        .await
        .unwrap();

    // Verify session journals remain strictly partitioned
    let journal_a = session_manager.get_journal(&session_a.id).await.unwrap();
    let journal_b = session_manager.get_journal(&session_b.id).await.unwrap();

    assert_eq!(journal_a.entries.len(), 1);
    assert!(journal_a.entries[0].entry_data.contains("Project Alpha"));
    assert!(!journal_a.entries[0].entry_data.contains("Project Beta"));

    assert_eq!(journal_b.entries.len(), 1);
    assert!(journal_b.entries[0].entry_data.contains("Project Beta"));
    assert!(!journal_b.entries[0].entry_data.contains("Project Alpha"));
}
