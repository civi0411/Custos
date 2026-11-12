use super::*;
use crate::connection::DbConnection;
use chrono::Utc;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::{Assurance, DomainError, EffectAttempt, EffectStatus, ExecutionReceipt, ReceiptStatus};

#[tokio::test]
async fn test_effect_idempotency_unique_constraint_enforcement() {
    let db = DbConnection::open_in_memory().unwrap();
    let repo = OutboxRepository::new(db);

    let effect1 = EffectAttempt {
        id: "eff_01".into(),
        node_attempt_id: "node_att_01".into(),
        permit_id: "pmt_01".into(),
        idempotency_key: "idemp_unique_key_01".into(),
        status: EffectStatus::InFlight,
        started_at: Utc::now(),
        ended_at: None,
        receipt: None,
    };

    // First insert must succeed
    repo.record_effect(&effect1)
        .expect("first effect insert succeeds");

    let effect2 = EffectAttempt {
        id: "eff_02".into(),
        node_attempt_id: "node_att_02".into(),
        permit_id: "pmt_02".into(),
        idempotency_key: "idemp_unique_key_01".into(), // Duplicate idempotency key!
        status: EffectStatus::InFlight,
        started_at: Utc::now(),
        ended_at: None,
        receipt: None,
    };

    // Second insert with same idempotency key must be rejected with Conflict error
    let duplicate_res = repo.record_effect(&effect2);
    assert!(duplicate_res.is_err());
    assert!(
        matches!(duplicate_res.unwrap_err(), DomainError::Conflict(msg) if msg.contains("Gate 3 Idempotency Enforcement")),
        "Duplicate effect insert must fail with Gate 3 Conflict error"
    );
}

#[tokio::test]
async fn test_outbox_cas_state_transitions() {
    let db = DbConnection::open_in_memory().unwrap();
    let repo = OutboxRepository::new(db);

    // First insert task row to satisfy foreign key constraint
    {
        let conn = repo.db.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_cas_01', 'CAS Task', 'running', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
    }

    let entry_id = "outbox_entry_cas_01";
    let entry = OutboxEntry::new(
        entry_id,
        "task_cas_01",
        "act_01",
        "pmt_01",
        "sha256:test_digest",
        Some("idemp_cas_01".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );

    repo.enqueue(entry).await.expect("enqueue succeeds");

    assert!(repo.mark_dispatching(entry_id).await.is_err());
    repo.mark_approved(entry_id).await.unwrap();
    assert_eq!(
        repo.get(entry_id).await.unwrap().unwrap().status,
        OutboxStatus::Approved
    );
    assert!(repo.mark_approved(entry_id).await.is_err());
    // 1. Approved -> Dispatching CAS succeeds
    repo.mark_dispatching(entry_id)
        .await
        .expect("mark dispatching succeeds");

    // 2. Calling mark_dispatching again on already Dispatching entry must fail CAS
    let err_double_dispatch = repo.mark_dispatching(entry_id).await.unwrap_err();
    assert!(
        matches!(err_double_dispatch, DomainError::InvalidStateTransition { ref from, ref to } if from == "dispatching" && to == "dispatching"),
        "Invalid CAS transition: {}",
        err_double_dispatch
    );

    // 3. Dispatching -> Receipted CAS succeeds
    let receipt = ExecutionReceipt {
        receipt_id: "rcpt_01".into(),
        permit_id: "pmt_01".into(),
        action_id: "act_01".into(),
        status: ReceiptStatus::Success,
        output_digest: "sha256:out".into(),
        output_data: None,
        error_message: None,
        duration_ms: Some(10),
        executed_at: Utc::now(),
        assurance: Assurance::CustosMediated,
    };

    repo.mark_receipted(entry_id, receipt)
        .await
        .expect("mark receipted succeeds");

    // 4. Calling mark_uncertain on already Receipted entry must fail CAS
    let err_uncertain_on_receipted = repo.mark_uncertain(entry_id).await.unwrap_err();
    assert!(
        matches!(err_uncertain_on_receipted, DomainError::InvalidStateTransition { ref from, ref to } if from == "receipted" && to == "uncertain"),
        "Invalid CAS transition: {}",
        err_uncertain_on_receipted
    );
}

#[tokio::test]
async fn test_outbox_mark_failed_and_get() {
    let db = DbConnection::open_in_memory().unwrap();
    let repo = OutboxRepository::new(db);

    {
        let conn = repo.db.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_fail_01', 'Fail Task', 'running', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
    }

    let entry_id = "outbox_entry_fail_01";
    let mut entry = OutboxEntry::new(
        entry_id,
        "task_fail_01",
        "act_fail",
        "pmt_fail",
        "sha256:fail_digest",
        Some("idemp_fail_01".into()),
        OutboxStatus::Pending,
        Utc::now(),
    );
    entry.recipient = Some("mcp://test-server/tool".into());
    entry.payload_json = Some("{\"param\":\"val\"}".into());

    repo.enqueue(entry).await.expect("enqueue succeeds");

    // Test get
    let retrieved = repo
        .get(entry_id)
        .await
        .expect("get succeeds")
        .expect("entry found");
    assert_eq!(
        retrieved.recipient.as_deref(),
        Some("mcp://test-server/tool")
    );
    assert_eq!(
        retrieved.payload_json.as_deref(),
        Some("{\"param\":\"val\"}")
    );
    assert_eq!(retrieved.status, OutboxStatus::Pending);

    // Mark failed
    repo.mark_failed(entry_id, "connection timeout")
        .await
        .expect("mark_failed succeeds");

    let failed_entry = repo
        .get(entry_id)
        .await
        .expect("get succeeds")
        .expect("entry found");
    assert_eq!(failed_entry.status, OutboxStatus::Failed);
    assert_eq!(
        failed_entry.error_message.as_deref(),
        Some("connection timeout")
    );
}

#[tokio::test]
async fn test_no_dispatch_before_approval_and_reconcile() {
    let db = DbConnection::open_in_memory().unwrap();
    let repo = OutboxRepository::new(db);

    // Verify that after reconcile_on_startup, any entries that were dispatching
    // are transitioned to uncertain, and zero unapproved entries can remain in dispatching state.
    let recovered = repo.reconcile_on_startup().expect("reconcile succeeds");
    assert_eq!(recovered, 0);

    let conn = repo.db.lock().unwrap();
    let dispatching_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outbox_entries WHERE status = 'dispatching'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        dispatching_count, 0,
        "No entries may remain dispatching without approval"
    );
}

