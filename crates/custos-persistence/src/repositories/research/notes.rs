use super::ResearchRepository;
use custos_domain::{DomainError, NoteRecord, NoteVersionRecord};
use rusqlite::params;

impl ResearchRepository {
    // Notes
    pub fn save_note(&self, note: &NoteRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let tags_json = serde_json::to_string(&note.tags)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO notes (
                id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                version = excluded.version,
                content_hash = excluded.content_hash,
                tags_json = excluded.tags_json,
                updated_at = excluded.updated_at",
            params![
                note.id,
                note.title,
                note.content,
                note.version as i64,
                note.content_hash,
                note.session_id,
                note.task_id,
                tags_json,
                note.created_at,
                note.updated_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn save_note_version(&self, ver: &NoteVersionRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO note_versions (
                id, note_id, version, content, content_hash, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO NOTHING",
            params![
                ver.id,
                ver.note_id,
                ver.version as i64,
                ver.content,
                ver.content_hash,
                ver.created_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_note(&self, id: &str) -> Result<Option<NoteRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
                 FROM notes WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let v: i64 = row.get(3)?;
                let tags_raw: String = row.get(7)?;
                let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                Ok(NoteRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    version: v as u32,
                    content_hash: row.get(4)?,
                    session_id: row.get(5)?,
                    task_id: row.get(6)?,
                    tags,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?)),
            None => Ok(None),
        }
    }

    pub fn list_notes(&self, session_id: Option<&str>) -> Result<Vec<NoteRecord>, DomainError> {
        let conn = self.db.lock()?;
        let query = if session_id.is_some() {
            "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
             FROM notes WHERE session_id = ?1 ORDER BY updated_at DESC"
        } else {
            "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
             FROM notes ORDER BY updated_at DESC"
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let param_vec: Vec<&dyn rusqlite::ToSql> = if let Some(ref sid) = session_id {
            vec![sid]
        } else {
            vec![]
        };

        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_vec), |row| {
                let v: i64 = row.get(3)?;
                let tags_raw: String = row.get(7)?;
                let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                Ok(NoteRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    version: v as u32,
                    content_hash: row.get(4)?,
                    session_id: row.get(5)?,
                    task_id: row.get(6)?,
                    tags,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn list_note_versions(&self, note_id: &str) -> Result<Vec<NoteVersionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, note_id, version, content, content_hash, created_at
                 FROM note_versions WHERE note_id = ?1 ORDER BY version DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![note_id], |row| {
                let v: i64 = row.get(2)?;
                Ok(NoteVersionRecord {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    version: v as u32,
                    content: row.get(3)?,
                    content_hash: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }
}
