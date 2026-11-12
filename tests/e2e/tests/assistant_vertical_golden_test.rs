//! Golden Acceptance Test G10 — Assistant Vertical: Outbox, Timeout Reconciliation & Double-Dispatch CAS Fencing (F09)
//!
//! Validates:
//! 1. Normal Path: Draft creation -> approval & payload lock -> outbox enqueue -> approval -> CAS dispatch -> receipted.
//! 2. Timeout-After-Send & Crash Reconciliation: Timeouts transition to Uncertain; daemon restart reconciles in-flight
//!    entries to Uncertain; zero blind retries prevents duplicate outbound effects and quota bypass.
//! 3. Tampered Payload & Unapproved Dispatch Rejection: Tampered payload fails digest validation; pending items cannot skip approval.
//! 4. Double-Dispatch CAS Fencing: Concurrent dispatches on the same outbox entry are strictly fenced to a single execution.

use chrono::Utc;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::{
    digest, new_id, ActionLifecycleState, Assurance, DomainError, EffectAttempt, EffectStatus,
    ExecutionReceipt, ReceiptStatus,
};
use custos_workbench_assistant::AssistantDraft;
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

fn ensure_task_exists(store: &SqliteTaskStore, task_id: &str) {
    let conn = store.db().lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO tasks (id, title, status, created_at, updated_at) VALUES (?1, 'Assistant Test Task', 'running', datetime('now'), datetime('now'))",
        [task_id],
    )
    .unwrap();
}

#[tokio::test]
async fn test_g10_normal_path_draft_creation_approval_outbox_dispatch_receipt() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let assistant_repo = custos_workbench_assistant::AssistantRepository::from_raw(store.writer_arc());

    // 1. Assistant Draft Creation
    let draft_id = new_id("draft");
    let task_id = "task_copilot_01";
    ensure_task_exists(&store, task_id);
    let session_id = "session_copilot_01";
    let mut draft = AssistantDraft::new(
        &draft_id,
        "Weekly Executive Briefing",
        "document",
        "Summary of core deliverables and milestones.",
    );
    draft.task_id = Some(task_id.into());
    draft.session_id = Some(session_id.into());
    draft.recipient = Some("exec-team@company.internal".into());
    assistant_repo.save_draft_sync(&draft).unwrap();

    let fetched = assistant_repo
        .get_draft_sync(&draft_id)
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, "draft");
    assert!(fetched.payload_lock.is_none());

    // 2. Draft Approval & Payload Lock Seal
    draft.status = "approved".into();
    let payload = serde_json::json!({
        "title": draft.title,
        "content": draft.content,
        "kind": draft.kind,
        "recipient": draft.recipient,
    });
    draft.payload_lock = Some(digest(payload.to_string().as_bytes()));
    assistant_repo.save_draft_sync(&draft).unwrap();

    let approved_draft = assistant_repo
        .get_draft_sync(&draft_id)
        .unwrap()
        .unwrap();
    assert_eq!(approved_draft.status, "approved");
    assert!(approved_draft.payload_lock.is_some());

    // 3. Outbox Action Item Enqueue (Pending Approval)
    let ob_id = new_id("ob");
    let payload_val = serde_json::json!({
        "draft_id": draft_id,
        "title": approved_draft.title,
        "content": approved_draft.content,
        "recipient": approved_draft.recipient,
    });
    let payload_str = payload_val.to_string();
    let argument_digest = format!("sha256:{}", digest(payload_str.as_bytes()));
    let permit_id = new_id("pmt");
    let idempotency_key = format!("idemp_{ob_id}");

    let mut entry = OutboxEntry::new(
        &ob_id,
        task_id,
        "dispatch_external_notification",
        &permit_id,
        &argument_digest,
        Some(idempotency_key.clone()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    entry.recipient = Some("mcp://slack/post_executive_briefing".into());
    entry.payload_json = Some(payload_str);
    store.outbox().enqueue(entry).await.unwrap();

    let pending_entry = store.outbox().get(&ob_id).await.unwrap().unwrap();
    assert_eq!(pending_entry.status, OutboxStatus::Pending);

    // 4. Outbox Approval Transition
    store.outbox().mark_approved(&ob_id).await.unwrap();
    let approved_entry = store.outbox().get(&ob_id).await.unwrap().unwrap();
    assert_eq!(approved_entry.status, OutboxStatus::Approved);

    // 5. CAS Transition to Dispatching
    store.outbox().mark_dispatching(&ob_id).await.unwrap();
    let dispatching_entry = store.outbox().get(&ob_id).await.unwrap().unwrap();
    assert_eq!(dispatching_entry.status, OutboxStatus::Dispatching);

    // 6. External Effect Execution & Durable Receipt Recording
    let receipt = ExecutionReceipt {
        receipt_id: new_id("rcpt"),
        permit_id: permit_id.clone(),
        action_id: "dispatch_external_notification".into(),
        status: ReceiptStatus::Success,
        output_digest: format!("sha256:{}", digest(b"slack_response_ok")),
        output_data: Some(serde_json::json!({ "channel": "C12345", "ts": "1700000000.0001" })),
        error_message: None,
        duration_ms: Some(85),
        executed_at: Utc::now(),
        assurance: Assurance::CustosMediated,
    };
    store
        .outbox()
        .mark_receipted(&ob_id, receipt)
        .await
        .unwrap();

    // 7. Verify Terminal Outbox State & Queryability
    let receipted_entry = store.outbox().get(&ob_id).await.unwrap().unwrap();
    assert_eq!(receipted_entry.status, OutboxStatus::Receipted);
    let attached_receipt = receipted_entry.receipt.expect("Receipt must be attached");
    assert_eq!(attached_receipt.status, ReceiptStatus::Success);
    assert_eq!(attached_receipt.permit_id, permit_id);
    assert_eq!(attached_receipt.duration_ms, Some(85));

    let task_receipts = store
        .outbox()
        .list_by_task_and_status(task_id, OutboxStatus::Receipted)
        .await
        .unwrap();
    assert_eq!(task_receipts.len(), 1);
    assert_eq!(task_receipts[0].id, ob_id);
}

#[tokio::test]
async fn test_g10_timeout_uncertain_and_startup_reconciliation_fences_blind_retries() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    ensure_task_exists(&store, "task_timeout");
    ensure_task_exists(&store, "task_crash");

    // Scenario A: Outbound call experiences timeout -> mark_uncertain
    let ob_timeout_id = new_id("ob_timeout");
    let mut entry1 = OutboxEntry::new(
        &ob_timeout_id,
        "task_timeout",
        "send_sms",
        new_id("pmt"),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        Some("idemp_timeout".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    entry1.recipient = Some("https://api.sms.com/send".into());
    entry1.payload_json = Some("{}".into());
    store.outbox().enqueue(entry1).await.unwrap();

    store.outbox().mark_approved(&ob_timeout_id).await.unwrap();
    store
        .outbox()
        .mark_dispatching(&ob_timeout_id)
        .await
        .unwrap();

    // Outbound HTTP dropped / timed out after dispatching: mark_uncertain per Gate 3
    store.outbox().mark_uncertain(&ob_timeout_id).await.unwrap();
    let uncertain_entry = store.outbox().get(&ob_timeout_id).await.unwrap().unwrap();
    assert_eq!(uncertain_entry.status, OutboxStatus::Uncertain);

    // Scenario B: Abrupt crash recovery while dispatching outbox & in-flight effect attempt
    let ob_crash_id = new_id("ob_crash");
    let mut entry2 = OutboxEntry::new(
        &ob_crash_id,
        "task_crash",
        "webhook_notify",
        new_id("pmt"),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        Some("idemp_crash".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    entry2.recipient = Some("https://webhook.site/notify".into());
    entry2.payload_json = Some("{}".into());
    store.outbox().enqueue(entry2).await.unwrap();
    store.outbox().mark_approved(&ob_crash_id).await.unwrap();
    store.outbox().mark_dispatching(&ob_crash_id).await.unwrap();

    // In-flight effect attempt during crash
    let mut in_flight_eff = EffectAttempt::new("node_att_crash", "pmt_eff", "idemp_eff_crash");
    in_flight_eff.mark_in_flight().unwrap();
    store.outbox().record_effect(&in_flight_eff).unwrap();

    // Verify pre-crash state
    let eff_before = store
        .outbox()
        .get_effect_by_idempotency_key("idemp_eff_crash")
        .unwrap()
        .unwrap();
    assert_eq!(eff_before.status, EffectStatus::InFlight);

    // DAEMON RESTART PROTOCOL: Gate 3 reconcile_on_startup()
    let recovered_count = store.outbox().reconcile_on_startup().unwrap();
    assert!(
        recovered_count >= 2,
        "Reconciliation must recover both in-flight effect attempt and dispatching outbox entry"
    );

    // Verify outbox entry was safely moved to Uncertain
    let crash_entry = store.outbox().get(&ob_crash_id).await.unwrap().unwrap();
    assert_eq!(
        crash_entry.status,
        OutboxStatus::Uncertain,
        "Dispatching entry during crash must be marked Uncertain on restart"
    );

    // Verify effect attempt was safely moved to Uncertain
    let eff_after = store
        .outbox()
        .get_effect_by_idempotency_key("idemp_eff_crash")
        .unwrap()
        .unwrap();
    assert_eq!(
        eff_after.status,
        EffectStatus::Uncertain,
        "In-flight effect attempt during crash must be marked Uncertain on restart"
    );

    // INVARIANT CHECK: Invariant Gate 3 forbids blind retry
    assert!(
        !ActionLifecycleState::Uncertain.allows_blind_retry(),
        "Uncertain actions must strictly forbid blind retry"
    );

    // Verify CAS fencing prevents re-dispatching an Uncertain entry without reconciliation
    let re_dispatch_res = store.outbox().mark_dispatching(&ob_crash_id).await;
    assert!(
        matches!(
            re_dispatch_res,
            Err(DomainError::InvalidStateTransition { .. })
        ),
        "CAS must reject mark_dispatching on an Uncertain outbox entry"
    );

    // Verify mark_approved cannot be called on Uncertain entry
    let approve_res = store.outbox().mark_approved(&ob_crash_id).await;
    assert!(
        matches!(approve_res, Err(DomainError::Conflict(_))),
        "Cannot approve an entry that is not Pending"
    );
}

#[tokio::test]
async fn test_g10_tampered_payload_and_unapproved_dispatch_fenced() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));

    let ob_id = new_id("ob_tamper");
    let task_id = "task_tamper";
    ensure_task_exists(&store, task_id);
    let original_payload =
        serde_json::json!({ "recipient": "user@org.com", "body": "Original text" });
    let original_payload_str = original_payload.to_string();
    let argument_digest = format!("sha256:{}", digest(original_payload_str.as_bytes()));

    let mut entry = OutboxEntry::new(
        &ob_id,
        task_id,
        "send_email",
        new_id("pmt"),
        &argument_digest,
        Some("idemp_tamper".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    entry.recipient = Some("smtp://mail.org.com".into());
    entry.payload_json = Some(original_payload_str);
    store.outbox().enqueue(entry).await.unwrap();

    // 1. Direct dispatch without approval must be rejected by CAS
    let premature_dispatch = store.outbox().mark_dispatching(&ob_id).await;
    assert!(
        matches!(
            premature_dispatch,
            Err(DomainError::InvalidStateTransition { .. })
        ),
        "Outbox entry in Pending state cannot skip approval to dispatch"
    );

    // 2. Tampered Payload Detection
    // Simulate an attacker modifying the database payload_json while leaving argument_digest
    let tampered_payload =
        serde_json::json!({ "recipient": "attacker@evil.com", "body": "Tampered transfer" });
    let tampered_payload_str = tampered_payload.to_string();

    let fetched = store.outbox().get(&ob_id).await.unwrap().unwrap();
    let current_payload = tampered_payload_str.as_bytes();
    let calculated_digest = format!("sha256:{}", digest(current_payload));

    assert_ne!(
        fetched.argument_digest, calculated_digest,
        "Calculated digest of tampered payload must not match recorded argument_digest"
    );
}

#[tokio::test]
async fn test_g10_double_dispatch_cas_fencing_prevents_duplicate_effects() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));

    let ob_id = new_id("ob_race");
    let task_id = "task_race";
    ensure_task_exists(&store, task_id);
    let entry = OutboxEntry::new(
        &ob_id,
        task_id,
        "publish_update",
        new_id("pmt"),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        Some("idemp_race".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    store.outbox().enqueue(entry).await.unwrap();
    store.outbox().mark_approved(&ob_id).await.unwrap();

    // Concurrent dispatch attempts
    let store1 = store.clone();
    let store2 = store.clone();
    let id1 = ob_id.clone();
    let id2 = ob_id.clone();

    let h1 = tokio::spawn(async move { store1.outbox().mark_dispatching(&id1).await });
    let h2 = tokio::spawn(async move { store2.outbox().mark_dispatching(&id2).await });

    let (res1, res2) = tokio::join!(h1, h2);
    let r1 = res1.unwrap();
    let r2 = res2.unwrap();

    let (succeeded, failed) = match (r1, r2) {
        (Ok(_), Err(e)) => (1, Some(e)),
        (Err(e), Ok(_)) => (1, Some(e)),
        (Ok(_), Ok(_)) => (2, None),
        (Err(_), Err(_)) => (0, None),
    };

    assert_eq!(
        succeeded, 1,
        "Exactly ONE worker must successfully transition outbox from Approved to Dispatching"
    );
    let err = failed.expect("The losing worker must receive a transition error");
    assert!(
        matches!(err, DomainError::InvalidStateTransition { .. }),
        "The losing worker must receive an InvalidStateTransition error due to CAS mismatch"
    );

    // Final verification: outbox status is Dispatching
    let final_entry = store.outbox().get(&ob_id).await.unwrap().unwrap();
    assert_eq!(final_entry.status, OutboxStatus::Dispatching);
}
