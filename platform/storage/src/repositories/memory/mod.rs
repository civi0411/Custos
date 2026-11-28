mod proposals;
mod recall;

use crate::connection::DbConnection;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_contracts::contracts::memory::MemoryPort;
use custos_contracts::{
    new_id, ConsolidationPolicy, ConsolidationReport, DomainError, FactProposal,
    MemoryConsolidationDraft, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
use rusqlite::params;

pub use proposals::MemoryProposalRecord;

const AUTO_ACCEPT_CONFIDENCE: f32 = 0.85;

#[derive(Clone)]
pub struct MemoryRepository {
    pub(super) db: DbConnection,
}

impl MemoryRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn apply_consolidation(
        &self,
        worker_run_id: &str,
        expected_active_count: usize,
        draft: &MemoryConsolidationDraft,
        policy: &ConsolidationPolicy,
    ) -> Result<ConsolidationReport, DomainError> {
        if draft.new_facts.len() > policy.max_new_facts {
            return Err(DomainError::Validation(
                "Consolidation exceeds max_new_facts".into(),
            ));
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(format!("Consolidation tx failed: {e}")))?;
        let active_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM memory_facts WHERE valid_to IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if usize::try_from(active_count).unwrap_or(0) != expected_active_count {
            return Err(DomainError::Validation(
                "Concurrent memory mutation detected".into(),
            ));
        }
        let now = Utc::now().to_rfc3339();
        let mut archived = 0usize;
        for id in &draft.archive_fact_ids {
            archived += tx
                .execute(
                    "UPDATE memory_facts SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
                    params![&now, id],
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        for fact in &draft.new_facts {
            let refs = serde_json::to_string(&fact.evidence_refs).unwrap_or_else(|_| "[]".into());
            insert_fact_tx(
                &tx,
                "semantic",
                None,
                None,
                fact.persona_scope.as_deref(),
                &fact.subject,
                &fact.predicate,
                &fact.object,
                fact.confidence,
                &now,
                Some(worker_run_id),
                &refs,
            )?;
        }
        tx.commit().map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(ConsolidationReport {
            accepted_facts: draft.new_facts.len(),
            archived_facts: archived,
        })
    }
}

#[async_trait]
impl MemoryPort for MemoryRepository {
    async fn recall_context(
        &self,
        query: &RecallQuery,
        scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.recall_context_inner(query, scope)
    }

    async fn recall_temporal_fact(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError> {
        self.recall_temporal_fact_inner(subject, predicate, at_time)
    }

    async fn propose_fact(
        &self,
        proposal: FactProposal,
        worker_run_id: WorkerRunId,
    ) -> Result<ProposalReceipt, MemoryError> {
        self.propose_fact_inner(proposal, worker_run_id)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn insert_fact_tx(
    tx: &rusqlite::Transaction<'_>,
    tier: &str,
    project_id: Option<&str>,
    task_id: Option<&str>,
    persona_scope: Option<&str>,
    subject: &str,
    predicate: &str,
    object: &str,
    confidence: f32,
    now_rfc3339: &str,
    source_run_id: Option<&str>,
    evidence_refs_json: &str,
) -> Result<String, DomainError> {
    let fact_id = new_id("mfact");
    let content = format!("{subject} {predicate}: {object}");
    tx.execute(
        "INSERT INTO memory_facts
         (id, tier, project_id, task_id, persona_scope, subject, predicate, object, content, confidence, valid_from, valid_to, recorded_at, source_run_id, evidence_refs_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?11, ?12, ?13)",
        params![
            &fact_id,
            tier,
            project_id,
            task_id,
            persona_scope,
            subject,
            predicate,
            object,
            &content,
            confidence,
            now_rfc3339,
            source_run_id,
            evidence_refs_json,
        ],
    )
    .map_err(|e| DomainError::Validation(format!("Failed to insert memory fact: {e}")))?;

    tx.execute(
        "INSERT INTO memory_facts_fts (id, content, subject, predicate, object)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![&fact_id, &content, subject, predicate, object],
    )
    .map_err(|e| DomainError::Validation(format!("Failed to index memory fact FTS5: {e}")))?;

    Ok(fact_id)
}

pub(super) fn status_to_str(status: ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Accepted => "accepted",
        ProposalStatus::Rejected => "rejected",
    }
}

pub(super) fn str_to_status(s: &str) -> ProposalStatus {
    match s {
        "accepted" => ProposalStatus::Accepted,
        "rejected" => ProposalStatus::Rejected,
        _ => ProposalStatus::Pending,
    }
}

pub(super) fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>, DomainError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DomainError::Validation(format!("Invalid RFC3339 timestamp '{raw}': {e}")))
}

#[cfg(test)]
#[path = "../memory_tests.rs"]
mod tests;

