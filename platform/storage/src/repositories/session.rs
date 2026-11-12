use crate::connection::DbConnection;
use custos_domain::{
    DomainError, Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus,
};
use rusqlite::{params, Connection};

#[derive(Clone)]
pub struct SessionRepository {
    db: DbConnection,
}

impl SessionRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, DomainError> {
        let conn = self.db.lock()?;
        Self::read_session(&conn, session_id)
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT session_id, mode, status, current_goal, promotion_score,
                        attached_to, promoted_to, created_at, updated_at
                 FROM sessions
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_session_row)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(sessions)
    }

    pub fn save_session(&self, session: &Session) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        Self::write_session(&conn, session)
    }

    pub fn delete_session(&self, session_id: &SessionId) -> Result<bool, DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "DELETE FROM session_journal WHERE session_id = ?1",
            params![session_id.0],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let deleted = tx
            .execute(
                "DELETE FROM sessions WHERE session_id = ?1",
                params![session_id.0],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(deleted > 0)
    }

    pub fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO session_journal (session_id, entry_type, entry_data, occurred_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                entry.session_id.0,
                entry.entry_type,
                entry.entry_data,
                entry.occurred_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_journal(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<SessionJournalEntry>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT entry_id, session_id, entry_type, entry_data, occurred_at
                 FROM session_journal
                 WHERE session_id = ?1
                 ORDER BY entry_id ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![session_id.0], |row| {
                Ok(SessionJournalEntry {
                    entry_id: Some(row.get(0)?),
                    session_id: SessionId(row.get(1)?),
                    entry_type: row.get(2)?,
                    entry_data: row.get(3)?,
                    occurred_at: row.get(4)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(entries)
    }

    pub fn read_session(
        conn: &Connection,
        session_id: &SessionId,
    ) -> Result<Option<Session>, DomainError> {
        let mut stmt = conn
            .prepare(
                "SELECT session_id, mode, status, current_goal, promotion_score,
                        attached_to, promoted_to, created_at, updated_at
                 FROM sessions WHERE session_id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query(params![session_id.0])
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| DomainError::Validation(e.to_string()))?
        {
            Ok(Some(
                Self::map_session_row(row).map_err(|e| DomainError::Validation(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn write_session(conn: &Connection, session: &Session) -> Result<(), DomainError> {
        let mode_str = serde_json::to_string(&session.mode)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let status_str = serde_json::to_string(&session.status)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let attached_to_str = session.attached_to.as_ref().map(|t| t.to_string());
        let promoted_to_str = session.promoted_to.as_ref().map(|t| t.to_string());

        conn.execute(
            r#"
            INSERT INTO sessions (
                session_id, mode, status, current_goal, promotion_score,
                attached_to, promoted_to, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(session_id) DO UPDATE SET
                mode=excluded.mode,
                status=excluded.status,
                current_goal=excluded.current_goal,
                promotion_score=excluded.promotion_score,
                attached_to=excluded.attached_to,
                promoted_to=excluded.promoted_to,
                updated_at=excluded.updated_at
            "#,
            params![
                session.id.0,
                mode_str,
                status_str,
                session.current_goal,
                session.promotion_score,
                attached_to_str,
                promoted_to_str,
                session.created_at,
                session.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn insert_session(conn: &Connection, session: &Session) -> rusqlite::Result<()> {
        Self::write_session(conn, session).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e.to_string())))
        })
    }

    fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<Session> {
        let id: String = row.get(0)?;
        let mode_str: String = row.get(1)?;
        let status_str: String = row.get(2)?;
        let current_goal: Option<String> = row.get(3)?;
        let promotion_score: f32 = row.get(4)?;
        let attached_to: Option<String> = row.get(5)?;
        let promoted_to: Option<String> = row.get(6)?;
        let created_at: String = row.get(7)?;
        let updated_at: String = row.get(8)?;

        let mode: SessionMode = serde_json::from_str(&mode_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let status: SessionStatus = serde_json::from_str(&status_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Session {
            id: SessionId(id),
            mode,
            status,
            current_goal,
            promotion_score,
            attached_to,
            promoted_to,
            created_at,
            updated_at,
        })
    }
}
