use super::{
    insert_fact_tx, parse_rfc3339, status_to_str, str_to_status, MemoryRepository,
    AUTO_ACCEPT_CONFIDENCE,
};
use chrono::{DateTime, Utc};
use custos_contracts::{
    new_id, DomainError, FactProposal, MemoryError, ProposalReceipt, ProposalStatus, WorkerRunId,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProposalRecord {
    pub proposal_id: String,
    pub worker_run_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub evidence_refs: Vec<String>,
    pub persona_scope: Option<String>,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl MemoryRepository {
    pub fn list_proposals(
        &self,
        status_filter: Option<ProposalStatus>,
        limit: usize,
    ) -> Result<Vec<MemoryProposalRecord>, DomainError> {
        let conn = self.db.reader()?;
        let limit_i64 = i64::try_from(limit.max(1)).unwrap_or(50);
        let status_str = status_filter.map(status_to_str);
        let mut stmt = conn
            .prepare(
                "SELECT proposal_id, worker_run_id, subject, predicate, object, confidence,
                        evidence_refs_json, persona_scope, status, created_at, resolved_at
                 FROM memory_fact_proposals
                 WHERE (?1 IS NULL OR status = ?1)
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare proposal query: {e}")))?;
        let rows = stmt
            .query_map(params![status_str, limit_i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, f32>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                ))
            })
            .map_err(|e| DomainError::Validation(format!("Failed to query proposals: {e}")))?;

        let mut out = Vec::new();
        for row in rows {
            let (
                proposal_id,
                worker_run_id,
                subject,
                predicate,
                object,
                confidence,
                refs_json,
                persona_scope,
                status_raw,
                created_raw,
                resolved_raw,
            ) = row.map_err(|e| DomainError::Validation(format!("Proposal row decode error: {e}")))?;
            let evidence_refs: Vec<String> = serde_json::from_str(&refs_json).unwrap_or_default();
            let created_at = parse_rfc3339(&created_raw)?;
            let resolved_at = resolved_raw.as_deref().map(parse_rfc3339).transpose()?;
            out.push(MemoryProposalRecord {
                proposal_id,
                worker_run_id,
                subject,
                predicate,
                object,
                confidence,
                evidence_refs,
                persona_scope,
                status: str_to_status(&status_raw),
                created_at,
                resolved_at,
            });
        }
        Ok(out)
    }

    pub fn resolve_proposal(
        &self,
        proposal_id: &str,
        accept: bool,
    ) -> Result<ProposalReceipt, DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(format!("Failed to start transaction: {e}")))?;
        let row: Option<(String, String, String, String, f32, String, Option<String>, String)> = tx
            .query_row(
                "SELECT worker_run_id, subject, predicate, object, confidence, evidence_refs_json, persona_scope, status
                 FROM memory_fact_proposals WHERE proposal_id = ?1",
                params![proposal_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to load proposal: {e}")))?;

        let Some((
            worker_run_id,
            subject,
            predicate,
            object,
            confidence,
            evidence_refs_json,
            persona_scope,
            current_status,
        )) = row
        else {
            return Err(DomainError::NotFound {
                kind: "memory_proposal".into(),
                id: proposal_id.to_string(),
            });
        };

        if current_status != "pending" {
            return Err(DomainError::Validation(format!(
                "Memory proposal '{proposal_id}' is already {current_status}"
            )));
        }

        let now = Utc::now().to_rfc3339();
        let next_status = if accept {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Rejected
        };
        tx.execute(
            "UPDATE memory_fact_proposals SET status = ?1, resolved_at = ?2 WHERE proposal_id = ?3",
            params![status_to_str(next_status), &now, proposal_id],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to update proposal: {e}")))?;

        if accept {
            insert_fact_tx(
                &tx,
                "semantic",
                None,
                None,
                persona_scope.as_deref(),
                &subject,
                &predicate,
                &object,
                confidence,
                &now,
                Some(&worker_run_id),
                &evidence_refs_json,
            )?;
        }

        tx.commit()
            .map_err(|e| DomainError::Validation(format!("Failed to commit proposal: {e}")))?;
        Ok(ProposalReceipt {
            proposal_id: proposal_id.to_string(),
            status: next_status,
        })
    }

    pub(super) fn propose_fact_inner(
        &self,
        proposal: FactProposal,
        worker_run_id: WorkerRunId,
    ) -> Result<ProposalReceipt, MemoryError> {
        if proposal.subject.trim().is_empty()
            || proposal.predicate.trim().is_empty()
            || proposal.object.trim().is_empty()
        {
            return Err(MemoryError::Rejected(
                "Subject, predicate, and object must be non-empty".into(),
            ));
        }
        if !proposal.confidence.is_finite() || !(0.0..=1.0).contains(&proposal.confidence) {
            return Err(MemoryError::Rejected(
                "Confidence must be finite and within [0.0, 1.0]".into(),
            ));
        }

        let mut conn = self
            .db
            .lock()
            .map_err(|e| MemoryError::Backend(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| MemoryError::Backend(format!("Transaction start failed: {e}")))?;

        let proposal_id = new_id("mprop");
        let now = Utc::now().to_rfc3339();
        let evidence_refs_json = serde_json::to_string(&proposal.evidence_refs)
            .map_err(|e| MemoryError::Backend(e.to_string()))?;
        let auto_accept =
            proposal.confidence >= AUTO_ACCEPT_CONFIDENCE && !proposal.evidence_refs.is_empty();
        let status = if auto_accept {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Pending
        };
        let resolved_at: Option<&str> = if auto_accept { Some(&now) } else { None };

        tx.execute(
            "INSERT INTO memory_fact_proposals
             (proposal_id, worker_run_id, subject, predicate, object, confidence, evidence_refs_json, persona_scope, status, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &proposal_id,
                &worker_run_id,
                &proposal.subject,
                &proposal.predicate,
                &proposal.object,
                proposal.confidence,
                &evidence_refs_json,
                proposal.persona_scope.as_deref(),
                status_to_str(status),
                &now,
                resolved_at,
            ],
        )
        .map_err(|e| MemoryError::Backend(format!("Insert proposal failed: {e}")))?;

        if auto_accept {
            insert_fact_tx(
                &tx,
                "semantic",
                None,
                None,
                proposal.persona_scope.as_deref(),
                &proposal.subject,
                &proposal.predicate,
                &proposal.object,
                proposal.confidence,
                &now,
                Some(&worker_run_id),
                &evidence_refs_json,
            )
            .map_err(|e| MemoryError::Backend(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| MemoryError::Backend(format!("Transaction commit failed: {e}")))?;

        Ok(ProposalReceipt {
            proposal_id,
            status,
        })
    }
}

