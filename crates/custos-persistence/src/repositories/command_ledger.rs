use crate::connection::DbConnection;
use custos_core::contracts::storage::{CommandAdmission, CommandLedgerEntry};
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

    pub fn admit(
        &self,
        command_id: &str,
        method: &str,
        payload_digest: &str,
        scope: Option<&str>,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Result<CommandAdmission, DomainError> {
        let conn = self.db.lock()?;
        let expires_at_ms = now_ms + ttl_ms;
        let created_at_dt = chrono::DateTime::from_timestamp_millis(now_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
        let expires_at_dt = chrono::DateTime::from_timestamp_millis(expires_at_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();

        let inserted = conn.execute(
            "INSERT INTO command_ledger (command_id, method, payload_digest, result_json, created_at, expires_at, state, scope, expires_at_ms)
             VALUES (?1, ?2, ?3, '', ?4, ?5, 'pending', ?6, ?7)
             ON CONFLICT(command_id) DO NOTHING",
            params![
                command_id,
                method,
                payload_digest,
                created_at_dt,
                expires_at_dt,
                scope,
                expires_at_ms
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        if inserted == 1 {
            return Ok(CommandAdmission::New);
        }

        let row_opt: Option<(String, String, String, String, Option<i64>)> = conn
            .query_row(
                "SELECT method, payload_digest, state, result_json, expires_at_ms FROM command_ledger WHERE command_id = ?1",
                params![command_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some((stored_method, stored_digest, state, result_json, exp_ms)) = row_opt {
            if let Some(exp) = exp_ms {
                if exp > 0 && exp <= now_ms {
                    return Ok(CommandAdmission::Conflict(format!(
                        "Command ID '{command_id}' has expired"
                    )));
                }
            }

            if stored_method != method || stored_digest != payload_digest {
                return Ok(CommandAdmission::Conflict(format!(
                    "Command ID conflict: '{command_id}' already registered with different method or payload"
                )));
            }

            if state == "completed" {
                return Ok(CommandAdmission::Replay(result_json));
            } else if state == "pending" {
                return Ok(CommandAdmission::InFlight);
            } else {
                return Ok(CommandAdmission::Conflict(format!(
                    "Command ID '{command_id}' in invalid state '{state}'"
                )));
            }
        }

        Ok(CommandAdmission::New)
    }

    pub fn complete(&self, command_id: &str, result_json: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let changed = conn.execute(
            "UPDATE command_ledger SET state = 'completed', result_json = ?2 WHERE command_id = ?1 AND state = 'pending'",
            params![command_id, result_json],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        if changed != 1 {
            return Err(DomainError::Conflict(format!("Command '{command_id}' is not pending")));
        }
        Ok(())
    }

    pub fn abandon(&self, command_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "DELETE FROM command_ledger WHERE command_id = ?1 AND state = 'pending'",
            params![command_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    fn read_command(conn: &Connection, command_id: &str) -> Result<Option<CommandLedgerEntry>, DomainError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut stmt = conn
            .prepare(
                "SELECT command_id, method, payload_digest, result_json, created_at, expires_at 
                 FROM command_ledger 
                 WHERE command_id = ?1 AND (expires_at_ms IS NULL OR expires_at_ms > ?2)",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let entry = stmt
            .query_row(params![command_id, now_ms], |row| {
                let cid: String = row.get(0)?;
                let method: String = row.get(1)?;
                let payload_digest: String = row.get(2)?;
                let result_json: String = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let expires_at_str: String = row.get(5)?;

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;

                let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?;

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
        let expires_at_ms = entry.expires_at.timestamp_millis();
        conn.execute(
            "INSERT INTO command_ledger 
             (command_id, method, payload_digest, result_json, created_at, expires_at, state, expires_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'completed', ?7)
             ON CONFLICT(command_id) DO UPDATE SET
                method = excluded.method,
                payload_digest = excluded.payload_digest,
                result_json = excluded.result_json,
                state = 'completed',
                expires_at = excluded.expires_at,
                expires_at_ms = excluded.expires_at_ms",
            params![
                entry.command_id,
                entry.method,
                entry.payload_digest,
                entry.result_json,
                entry.created_at.to_rfc3339(),
                entry.expires_at.to_rfc3339(),
                expires_at_ms,
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

    #[test]
    fn test_command_admission_lifecycle() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = CommandLedgerRepository::new(db);

        let t0 = 1_700_000_000_000i64;
        let ttl = 60_000i64; // 60s

        // 1. Admit new command
        let res = repo.admit("cmd-1", "v1.tasks.create", "hash-a", Some("task"), t0, ttl).unwrap();
        assert_eq!(res, CommandAdmission::New);

        // 2. Re-admit identical while in-flight -> InFlight
        let res2 = repo.admit("cmd-1", "v1.tasks.create", "hash-a", Some("task"), t0 + 1000, ttl).unwrap();
        assert_eq!(res2, CommandAdmission::InFlight);

        // 3. Re-admit with conflicting payload or method -> Conflict
        let res_conf = repo.admit("cmd-1", "v1.tasks.create", "hash-diff", None, t0 + 2000, ttl).unwrap();
        match res_conf {
            CommandAdmission::Conflict(_) => {}
            other => panic!("Expected Conflict, got {:?}", other),
        }

        // 4. Complete command -> subsequent admit returns Replay
        repo.complete("cmd-1", r#"{"task_id":"task-xyz"}"#).unwrap();
        let res_replay = repo.admit("cmd-1", "v1.tasks.create", "hash-a", None, t0 + 3000, ttl).unwrap();
        assert_eq!(res_replay, CommandAdmission::Replay(r#"{"task_id":"task-xyz"}"#.to_string()));

        // 5. Expiration check
        let res_expired = repo.admit("cmd-1", "v1.tasks.create", "hash-a", None, t0 + ttl + 1, ttl).unwrap();
        match res_expired {
            CommandAdmission::Conflict(msg) => assert!(msg.contains("expired")),
            other => panic!("Expected Conflict with expired msg, got {:?}", other),
        }

        // 6. Abandon lifecycle
        let res_abandon = repo.admit("cmd-2", "v1.tasks.cancel", "hash-b", None, t0, ttl).unwrap();
        assert_eq!(res_abandon, CommandAdmission::New);
        repo.abandon("cmd-2").unwrap();
        // After abandon, cmd-2 can be admitted again as New
        let res_re_admit = repo.admit("cmd-2", "v1.tasks.cancel", "hash-b", None, t0 + 500, ttl).unwrap();
        assert_eq!(res_re_admit, CommandAdmission::New);
    }
}

