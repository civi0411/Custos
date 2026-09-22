use crate::connection::DbConnection;
use custos_core_domain::{ContinuationPacket, DomainError};
use rusqlite::params;

#[derive(Clone)]
pub struct ContinuationRepository {
    db: DbConnection,
}

impl ContinuationRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO continuation_packets (task_id, from_span, to_span, provider, model, task_summary, current_state, integrity_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                packet.task_id,
                packet.from_span as i64,
                packet.to_span as i64,
                packet.provider,
                packet.model,
                packet.task_summary,
                packet.current_state.to_string(),
                packet.integrity_hash,
                packet.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT task_id, from_span, to_span, provider, model, task_summary, current_state, integrity_hash, created_at
                 FROM continuation_packets WHERE task_id = ?1 ORDER BY to_span DESC LIMIT 1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let packet = stmt.query_row(params![task_id], |row| {
            let state_str: String = row.get(6)?;
            let current_state = serde_json::from_str(&state_str).unwrap_or(serde_json::json!({}));

            let created_at_str: String = row.get(8)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            Ok(ContinuationPacket {
                task_id: row.get(0)?,
                from_span: row.get::<_, i64>(1)? as u32,
                to_span: row.get::<_, i64>(2)? as u32,
                provider: row.get(3)?,
                model: row.get(4)?,
                task_summary: row.get(5)?,
                current_state,
                integrity_hash: row.get(7)?,
                created_at,
            })
        });

        match packet {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }
}
