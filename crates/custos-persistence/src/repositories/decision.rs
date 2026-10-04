use crate::connection::DbConnection;
use custos_domain::{DecisionRecord, DomainError};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct DecisionRepository {
    db: DbConnection,
}

impl DecisionRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_decision(&self, record: &DecisionRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let snapshot_json = serde_json::to_string(&record.snapshot)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let proposal_json = serde_json::to_string(&record.proposal)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO decision_records (id, task_id, created_at, snapshot_json, proposal_json, overhead_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                task_id = excluded.task_id,
                created_at = excluded.created_at,
                snapshot_json = excluded.snapshot_json,
                proposal_json = excluded.proposal_json,
                overhead_ms = excluded.overhead_ms",
            params![
                record.id,
                record.task_id,
                record.created_at.to_rfc3339(),
                snapshot_json,
                proposal_json,
                record.overhead_ms as i64,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_decision(&self, id: &str) -> Result<Option<DecisionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, created_at, snapshot_json, proposal_json, overhead_ms
                 FROM decision_records WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let record = stmt
            .query_row(params![id], |row| Self::map_decision(row))
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(record)
    }

    pub fn list_decisions_for_task(&self, task_id: &str) -> Result<Vec<DecisionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, created_at, snapshot_json, proposal_json, overhead_ms
                 FROM decision_records WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], |row| Self::map_decision(row))
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    fn map_decision(row: &rusqlite::Row) -> rusqlite::Result<DecisionRecord> {
        let created_at_str: String = row.get(2)?;
        let snapshot_str: String = row.get(3)?;
        let proposal_str: String = row.get(4)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let snapshot = serde_json::from_str(&snapshot_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;
        let proposal = serde_json::from_str(&proposal_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;

        Ok(DecisionRecord {
            id: row.get(0)?,
            task_id: row.get(1)?,
            created_at,
            snapshot,
            proposal,
            overhead_ms: row.get::<_, i64>(5)? as u64,
        })
    }
}
