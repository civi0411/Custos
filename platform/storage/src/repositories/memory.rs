use crate::connection::DbConnection;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_contracts::contracts::memory::MemoryPort;
use custos_contracts::{
    new_id, DomainError, FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact,
    ProposalReceipt, ProposalStatus, RecallQuery, WorkerRunId,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

const AUTO_ACCEPT_CONFIDENCE: f32 = 0.85;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProposalRecord {
    pub proposal_id: String,
    pub worker_run_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub evidence_refs: Vec<String>,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct MemoryRepository {
    db: DbConnection,
}

impl MemoryRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn list_proposals(
        &self,
        status_filter: Option<ProposalStatus>,
        limit: usize,
    ) -> Result<Vec<MemoryProposalRecord>, DomainError> {
        let conn = self.db.lock()?;
        let limit_i64 = i64::try_from(limit.max(1)).unwrap_or(50);
        let status_str = status_filter.map(status_to_str);
        let mut stmt = conn
            .prepare(
                "SELECT proposal_id, worker_run_id, subject, predicate, object, confidence,
                        evidence_refs_json, status, created_at, resolved_at
                 FROM memory_fact_proposals
                 WHERE (?1 IS NULL OR status = ?1)
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare proposal query: {e}")))?;
        let rows = stmt
            .query_map(params![status_str, limit_i64], |row| {
                let refs_json: String = row.get(6)?;
                let status_raw: String = row.get(7)?;
                let created_raw: String = row.get(8)?;
                let resolved_raw: Option<String> = row.get(9)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, f32>(5)?,
                    refs_json,
                    status_raw,
                    created_raw,
                    resolved_raw,
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
        let row: Option<(String, String, String, String, f32, String, String)> = tx
            .query_row(
                "SELECT worker_run_id, subject, predicate, object, confidence, evidence_refs_json, status
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
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to load proposal: {e}")))?;

        let Some((worker_run_id, subject, predicate, object, confidence, evidence_refs_json, current_status)) =
            row
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
}

#[async_trait]
impl MemoryPort for MemoryRepository {
    async fn recall_context(
        &self,
        query: &RecallQuery,
        scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| MemoryError::Backend(e.to_string()))?;
        let limit_i64 = i64::try_from(query.limit.max(1)).unwrap_or(20);
        let fts_expr = build_fts_query(&query.text);

        let mut rows_out: Vec<MemoryEntry> = Vec::new();
        if let Some(expr) = fts_expr {
            let mut stmt = conn
                .prepare(
                    "SELECT f.id, f.content, f.tier, f.recorded_at, (-bm25(memory_facts_fts)) AS rank_score
                     FROM memory_facts_fts
                     JOIN memory_facts f ON f.id = memory_facts_fts.id
                     WHERE memory_facts_fts MATCH ?1
                       AND f.valid_to IS NULL
                       AND (
                         (?2 IS NOT NULL AND f.task_id = ?2)
                         OR (?3 IS NOT NULL AND f.project_id = ?3 AND f.task_id IS NULL)
                         OR (?4 = 1 AND f.task_id IS NULL AND f.project_id IS NULL)
                       )
                     ORDER BY rank_score DESC, f.recorded_at DESC
                     LIMIT ?5",
                )
                .map_err(|e| MemoryError::Backend(format!("FTS prepare failed: {e}")))?;
            let mapped = stmt
                .query_map(
                    params![
                        expr,
                        scope.task_id.as_deref(),
                        scope.project.as_deref(),
                        i32::from(scope.include_personal),
                        limit_i64
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, f64>(4)?,
                        ))
                    },
                )
                .map_err(|e| MemoryError::Backend(format!("FTS query failed: {e}")))?;

            for row in mapped {
                let (id, content, tier, recorded_raw, rank_score) =
                    row.map_err(|e| MemoryError::Backend(e.to_string()))?;
                let recorded_at = parse_rfc3339(&recorded_raw)
                    .map_err(|e| MemoryError::Backend(e.to_string()))?;
                rows_out.push(MemoryEntry {
                    id,
                    content,
                    source: format!("sqlite:{tier}"),
                    score: (rank_score as f32).max(0.01),
                    recorded_at,
                });
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, content, tier, recorded_at, confidence
                     FROM memory_facts
                     WHERE valid_to IS NULL
                       AND (
                         (?1 IS NOT NULL AND task_id = ?1)
                         OR (?2 IS NOT NULL AND project_id = ?2 AND task_id IS NULL)
                         OR (?3 = 1 AND task_id IS NULL AND project_id IS NULL)
                       )
                     ORDER BY recorded_at DESC
                     LIMIT ?4",
                )
                .map_err(|e| MemoryError::Backend(format!("Recall prepare failed: {e}")))?;
            let mapped = stmt
                .query_map(
                    params![
                        scope.task_id.as_deref(),
                        scope.project.as_deref(),
                        i32::from(scope.include_personal),
                        limit_i64
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, f32>(4)?,
                        ))
                    },
                )
                .map_err(|e| MemoryError::Backend(format!("Recall query failed: {e}")))?;

            for row in mapped {
                let (id, content, tier, recorded_raw, confidence) =
                    row.map_err(|e| MemoryError::Backend(e.to_string()))?;
                let recorded_at = parse_rfc3339(&recorded_raw)
                    .map_err(|e| MemoryError::Backend(e.to_string()))?;
                rows_out.push(MemoryEntry {
                    id,
                    content,
                    source: format!("sqlite:{tier}"),
                    score: confidence,
                    recorded_at,
                });
            }
        }

        if let Some(budget) = query.token_budget {
            let mut used = 0usize;
            let mut budgeted = Vec::with_capacity(rows_out.len());
            for entry in rows_out {
                let est = (entry.content.len() / 4).max(1);
                if used + est > budget && !budgeted.is_empty() {
                    break;
                }
                if est <= budget {
                    used += est;
                    budgeted.push(entry);
                }
            }
            return Ok(budgeted);
        }

        Ok(rows_out)
    }

    async fn recall_temporal_fact(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| MemoryError::Backend(e.to_string()))?;
        let at_str = at_time.to_rfc3339();
        let row: Option<(String, String, String, f32, String, Option<String>)> = conn
            .query_row(
                "SELECT subject, predicate, object, confidence, valid_from, valid_to
                 FROM memory_facts
                 WHERE subject = ?1 AND predicate = ?2
                   AND valid_from <= ?3
                   AND (valid_to IS NULL OR valid_to > ?3)
                 ORDER BY valid_from DESC, confidence DESC
                 LIMIT 1",
                params![subject, predicate, at_str],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| MemoryError::Backend(format!("Temporal recall failed: {e}")))?;

        row.map(
            |(subject, predicate, object, confidence, valid_from_raw, valid_to_raw)| {
                let valid_from = parse_rfc3339(&valid_from_raw)
                    .map_err(|e| MemoryError::Backend(e.to_string()))?;
                let valid_to = valid_to_raw
                    .as_deref()
                    .map(parse_rfc3339)
                    .transpose()
                    .map_err(|e| MemoryError::Backend(e.to_string()))?;
                Ok(PersonalFact {
                    subject,
                    predicate,
                    object,
                    confidence,
                    valid_from,
                    valid_to,
                })
            },
        )
        .transpose()
    }

    async fn propose_fact(
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
             (proposal_id, worker_run_id, subject, predicate, object, confidence, evidence_refs_json, status, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &proposal_id,
                &worker_run_id,
                &proposal.subject,
                &proposal.predicate,
                &proposal.object,
                proposal.confidence,
                &evidence_refs_json,
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

#[allow(clippy::too_many_arguments)]
fn insert_fact_tx(
    tx: &rusqlite::Transaction<'_>,
    tier: &str,
    project_id: Option<&str>,
    task_id: Option<&str>,
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
         (id, tier, project_id, task_id, subject, predicate, object, content, confidence, valid_from, valid_to, recorded_at, source_run_id, evidence_refs_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?10, ?11, ?12)",
        params![
            &fact_id,
            tier,
            project_id,
            task_id,
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

fn build_fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{s}\""))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" OR "))
    }
}

fn status_to_str(status: ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Accepted => "accepted",
        ProposalStatus::Rejected => "rejected",
    }
}

fn str_to_status(s: &str) -> ProposalStatus {
    match s {
        "accepted" => ProposalStatus::Accepted,
        "rejected" => ProposalStatus::Rejected,
        _ => ProposalStatus::Pending,
    }
}

fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>, DomainError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DomainError::Validation(format!("Invalid RFC3339 timestamp '{raw}': {e}")))
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
