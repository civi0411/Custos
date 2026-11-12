use crate::connection::DbConnection;
use custos_domain::{DomainError, ReplanBrief, ReplanRecord};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct ReplanRepository {
    db: DbConnection,
}

impl ReplanRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_replan(&self, record: &ReplanRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let brief_json = serde_json::to_string(&record.brief)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let trigger_str =
            serde_json::to_string(&record.brief.trigger).unwrap_or_else(|_| "unknown".to_string());

        conn.execute(
            "INSERT INTO replan_records (
                id, task_id, created_at, trigger, failed_node_id, reason, brief_json, new_proposal_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                task_id = excluded.task_id,
                created_at = excluded.created_at,
                trigger = excluded.trigger,
                failed_node_id = excluded.failed_node_id,
                reason = excluded.reason,
                brief_json = excluded.brief_json,
                new_proposal_id = excluded.new_proposal_id",
            params![
                record.id,
                record.task_id,
                record.created_at.to_rfc3339(),
                trigger_str,
                record.brief.failed_node_id,
                record.brief.reason,
                brief_json,
                record.new_proposal_id,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_replan(&self, id: &str) -> Result<Option<ReplanRecord>, DomainError> {
        let reader = self.db.reader()?;
        let mut stmt = reader
            .prepare(
                "SELECT id, task_id, created_at, brief_json, new_proposal_id
                 FROM replan_records WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let record = stmt
            .query_row(params![id], Self::map_replan)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(record)
    }

    pub fn list_replans_for_task(&self, task_id: &str) -> Result<Vec<ReplanRecord>, DomainError> {
        let reader = self.db.reader()?;
        let mut stmt = reader
            .prepare(
                "SELECT id, task_id, created_at, brief_json, new_proposal_id
                 FROM replan_records WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], Self::map_replan)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    fn map_replan(row: &rusqlite::Row) -> rusqlite::Result<ReplanRecord> {
        let created_at_str: String = row.get(2)?;
        let brief_str: String = row.get(3)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let brief: ReplanBrief = serde_json::from_str(&brief_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(ReplanRecord {
            id: row.get(0)?,
            task_id: row.get(1)?,
            created_at,
            brief,
            new_proposal_id: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::ReplanTrigger;

    #[test]
    fn test_save_and_get_replan_record() {
        let db = DbConnection::open_in_memory().unwrap();
        // Insert task for foreign key
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('task_replan_1', 'Replan Task', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let repo = ReplanRepository::new(db);

        let brief = ReplanBrief::new(
            "task_replan_1",
            ReplanTrigger::TestFailure,
            Some("node_fail_1".into()),
            "Unit test assertions failed in verification phase",
        );

        let record = ReplanRecord::new("task_replan_1", brief, "prop_new_02");

        repo.save_replan(&record).unwrap();

        let fetched = repo.get_replan(&record.id).unwrap().unwrap();
        assert_eq!(fetched.id, record.id);
        assert_eq!(fetched.task_id, "task_replan_1");
        assert_eq!(fetched.brief.trigger, ReplanTrigger::TestFailure);
        assert_eq!(fetched.brief.failed_node_id, Some("node_fail_1".into()));
        assert_eq!(fetched.new_proposal_id, "prop_new_02");

        let list = repo.list_replans_for_task("task_replan_1").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, record.id);

        // Test idempotency: re-save same record with updated proposal
        let mut updated = record.clone();
        updated.new_proposal_id = "prop_new_03".into();
        repo.save_replan(&updated).unwrap();

        let re_fetched = repo.get_replan(&record.id).unwrap().unwrap();
        assert_eq!(re_fetched.new_proposal_id, "prop_new_03");
    }
}
