use super::{parse_rfc3339, MemoryRepository};
use chrono::{DateTime, Utc};
use custos_contracts::{MemoryEntry, MemoryError, MemoryScope, PersonalFact, RecallQuery};
use rusqlite::{params, OptionalExtension};

impl MemoryRepository {
    pub(super) fn recall_context_inner(
        &self,
        query: &RecallQuery,
        scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let conn = self
            .db
            .reader()
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
                         (?5 IS NOT NULL AND f.persona_scope = ?5)
                         OR (?5 IS NULL AND f.persona_scope IS NULL AND (
                           (?2 IS NOT NULL AND f.task_id = ?2)
                           OR (?3 IS NOT NULL AND f.project_id = ?3 AND f.task_id IS NULL)
                           OR (?4 = 1 AND f.task_id IS NULL AND f.project_id IS NULL)
                         ))
                       )
                     ORDER BY rank_score DESC, f.recorded_at DESC
                     LIMIT ?6",
                )
                .map_err(|e| MemoryError::Backend(format!("FTS prepare failed: {e}")))?;
            let mapped = stmt
                .query_map(
                    params![
                        expr,
                        scope.task_id.as_deref(),
                        scope.project.as_deref(),
                        i32::from(scope.include_personal),
                        scope.persona_scope.as_deref(),
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
                         (?4 IS NOT NULL AND persona_scope = ?4)
                         OR (?4 IS NULL AND persona_scope IS NULL AND (
                           (?1 IS NOT NULL AND task_id = ?1)
                           OR (?2 IS NOT NULL AND project_id = ?2 AND task_id IS NULL)
                           OR (?3 = 1 AND task_id IS NULL AND project_id IS NULL)
                         ))
                       )
                     ORDER BY recorded_at DESC
                     LIMIT ?5",
                )
                .map_err(|e| MemoryError::Backend(format!("Recall prepare failed: {e}")))?;
            let mapped = stmt
                .query_map(
                    params![
                        scope.task_id.as_deref(),
                        scope.project.as_deref(),
                        i32::from(scope.include_personal),
                        scope.persona_scope.as_deref(),
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

    pub(super) fn recall_temporal_fact_inner(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError> {
        let conn = self
            .db
            .reader()
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

