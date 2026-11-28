use super::*;
use crate::store::SqliteTaskStore;

#[tokio::test]
async fn memory_repository_persists_fts5_and_enforces_proposal_governance() {
    let store = SqliteTaskStore::new_in_memory().expect("in-memory store");
    let repo = store.memory();

    // Low-confidence or no-evidence proposal stays Pending and is not recalled yet
    let pending = repo
        .propose_fact(
            FactProposal {
                subject: "workspace".into(),
                predicate: "uses_db".into(),
                object: "SQLite WAL".into(),
                confidence: 0.90,
                evidence_refs: vec![],
                persona_scope: None,
            },
            "run_1".into(),
        )
        .await
        .expect("propose pending");
    assert_eq!(pending.status, ProposalStatus::Pending);

    let empty_recall = repo
        .recall_context(
            &RecallQuery {
                text: "SQLite WAL".into(),
                limit: 5,
                token_budget: None,
            },
            &MemoryScope {
                include_personal: true,
                ..Default::default()
            },
        )
        .await
        .expect("recall empty");
    assert!(empty_recall.is_empty());

    // High-confidence + evidence auto-accepts and indexes into FTS5
    let accepted = repo
        .propose_fact(
            FactProposal {
                subject: "custos".into(),
                predicate: "storage_engine".into(),
                object: "rusqlite bundled FTS5".into(),
                confidence: 0.95,
                evidence_refs: vec!["cas:abc123".into()],
                persona_scope: None,
            },
            "run_2".into(),
        )
        .await
        .expect("propose accepted");
    assert_eq!(accepted.status, ProposalStatus::Accepted);

    let hits = repo
        .recall_context(
            &RecallQuery {
                text: "FTS5".into(),
                limit: 5,
                token_budget: Some(128),
            },
            &MemoryScope {
                include_personal: true,
                ..Default::default()
            },
        )
        .await
        .expect("recall hits");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].content.contains("rusqlite bundled FTS5"));

    // Approve pending proposal via governance API
    let resolved = repo
        .resolve_proposal(&pending.proposal_id, true)
        .expect("resolve pending");
    assert_eq!(resolved.status, ProposalStatus::Accepted);

    let temporal = repo
        .recall_temporal_fact("workspace", "uses_db", Utc::now() + chrono::Duration::seconds(1))
        .await
        .expect("temporal")
        .expect("fact present");
    assert_eq!(temporal.object, "SQLite WAL");
}

