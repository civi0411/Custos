use crate::connection::DbConnection;
use custos_core::contracts::storage::CommandLedgerEntry;
use custos_domain::DomainError;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone)]
pub struct CommandLedgerRepository {
    db: DbConnection,
}

impl CommandLedgerRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn get_command(&self, command_id: &str) -> Result<Option<CommandLedgerEntry>, DomainError> {
        let conn = self.db.lock()?;
        Self::read_command(&conn, command_id)
    }

    pub fn record_command(&self, entry: &CommandLedgerEntry) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        Self::write_command(&conn, entry)
    }

    fn read_command(conn: &Connection, command_id: &str) -> Result<Option<CommandLedgerEntry>, DomainError> {
        let mut stmt = conn
            .prepare(
                "SELECT command_id, method, payload_digest, result_json, created_at, expires_at 
                 FROM command_ledger 
                 WHERE command_id = ?1 AND expires_at > datetime('now')",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let entry = stmt
            .query_row(params![command_id], |row| {
                let cid: String = row.get(0)?;
                let method: String = row.get(1)?;
                let payload_digest: String = row.get(2)?;
                let result_json: String = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let expires_at_str: String = row.get(5)?;

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                Ok(CommandLedgerEntry {
                    command_id: cid,
                    method,
                    payload_digest,
                    result_json,
                    created_at,
                    expires_at,
                })
            })
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(entry)
    }

    fn write_command(conn: &Connection, entry: &CommandLedgerEntry) -> Result<(), DomainError> {
        conn.execute(
            "INSERT OR REPLACE INTO command_ledger 
             (command_id, method, payload_digest, result_json, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.command_id,
                entry.method,
                entry.payload_digest,
                entry.result_json,
                entry.created_at.to_rfc3339(),
                entry.expires_at.to_rfc3339(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_command_ledger_crud_and_expiration() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = CommandLedgerRepository::new(db);

        let now = Utc::now();
        let entry = CommandLedgerEntry {
            command_id: "cmd_test_123".to_string(),
            method: "v1.tasks.create".to_string(),
            payload_digest: "digest_abc".to_string(),
            result_json: r#"{"task_id":"task_123"}"#.to_string(),
            created_at: now,
            expires_at: now + Duration::days(7),
        };

        repo.record_command(&entry).unwrap();

        let retrieved = repo.get_command("cmd_test_123").unwrap();
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.command_id, "cmd_test_123");
        assert_eq!(r.method, "v1.tasks.create");
        assert_eq!(r.payload_digest, "digest_abc");
        assert_eq!(r.result_json, r#"{"task_id":"task_123"}"#);

        // Expired entry should return None
        let expired_entry = CommandLedgerEntry {
            command_id: "cmd_expired".to_string(),
            method: "v1.tasks.create".to_string(),
            payload_digest: "digest_exp".to_string(),
            result_json: r#"{"task_id":"task_exp"}"#.to_string(),
            created_at: now - Duration::days(10),
            expires_at: now - Duration::days(1),
        };
        repo.record_command(&expired_entry).unwrap();
        let exp_retrieved = repo.get_command("cmd_expired").unwrap();
        assert!(exp_retrieved.is_none());
    }
}

