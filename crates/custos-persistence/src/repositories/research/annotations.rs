use super::ResearchRepository;
use chrono::{DateTime, Utc};
use custos_domain::{
    AnnotationRecord, AnnotationStatus, AnnotationTarget, DomainError,
};
use rusqlite::params;

impl ResearchRepository {
    // Annotations
    pub fn save_annotation(&self, ann: &AnnotationRecord) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn.transaction().map_err(|error| DomainError::Validation(error.to_string()))?;
        let target_json = serde_json::to_string(&ann.target)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let status_str = match ann.status {
            AnnotationStatus::Pending => "pending",
            AnnotationStatus::Submitted => "submitted",
            AnnotationStatus::Addressed => "addressed",
            AnnotationStatus::Dismissed => "dismissed",
        };

        let affected = tx.execute(
            "INSERT INTO research_annotations (
                id, artifact_path, artifact_version, target_json, note, actor,
                status, side_chat_session_id, submitted_turn_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                target_json = excluded.target_json,
                note = excluded.note,
                status = excluded.status,
                side_chat_session_id = excluded.side_chat_session_id,
                submitted_turn_id = excluded.submitted_turn_id
            WHERE research_annotations.artifact_path = excluded.artifact_path
                AND research_annotations.artifact_version = excluded.artifact_version",
            params![
                ann.id,
                ann.artifact_path,
                ann.artifact_version,
                target_json,
                ann.note,
                ann.actor,
                status_str,
                ann.side_chat_session_id,
                ann.submitted_turn_id,
                ann.created_at.timestamp(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        if affected == 0 {
            return Err(DomainError::Validation(
                "Annotation identity belongs to a different artifact/version".into(),
            ));
        }
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("ann_event"),
                "research.annotation.saved",
                serde_json::json!({"annotation_id": ann.id, "artifact_path": ann.artifact_path, "artifact_version": ann.artifact_version}).to_string(),
                Utc::now().timestamp()
            ],
        )
        .map_err(|error| DomainError::Validation(error.to_string()))?;
        tx.commit().map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(())
    }

    pub fn list_annotations_for_artifact(
        &self,
        path: &str,
        version: Option<u32>,
    ) -> Result<Vec<AnnotationRecord>, DomainError> {
        let conn = self.db.lock()?;
        let (query, has_version) = if version.is_some() {
            (
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE artifact_path = ?1 AND artifact_version = ?2 ORDER BY created_at ASC",
                true,
            )
        } else {
            (
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE artifact_path = ?1 ORDER BY created_at ASC",
                false,
            )
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = if has_version {
            stmt.query_map(params![path, version.unwrap()], Self::map_annotation_row)
        } else {
            stmt.query_map(params![path], Self::map_annotation_row)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn list_pending_annotations(&self) -> Result<Vec<AnnotationRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE status = 'pending' ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_annotation_row)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn update_annotation_status(
        &self,
        id: &str,
        status: AnnotationStatus,
        turn_id: Option<&str>,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let status_str = match status {
            AnnotationStatus::Pending => "pending",
            AnnotationStatus::Submitted => "submitted",
            AnnotationStatus::Addressed => "addressed",
            AnnotationStatus::Dismissed => "dismissed",
        };

        conn.execute(
            "UPDATE research_annotations SET status = ?1, submitted_turn_id = COALESCE(?2, submitted_turn_id) WHERE id = ?3",
            params![status_str, turn_id, id],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub(crate) fn map_annotation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnnotationRecord> {
        let target_json: String = row.get(3)?;
        let target: AnnotationTarget =
            serde_json::from_str(&target_json).unwrap_or(AnnotationTarget::Text {
                start_offset: 0,
                end_offset: 0,
                exact_text: String::new(),
            });
        let status_str: String = row.get(6)?;
        let status = match status_str.as_str() {
            "submitted" => AnnotationStatus::Submitted,
            "addressed" => AnnotationStatus::Addressed,
            "dismissed" => AnnotationStatus::Dismissed,
            _ => AnnotationStatus::Pending,
        };
        let ts: i64 = row.get(9)?;

        Ok(AnnotationRecord {
            id: row.get(0)?,
            artifact_path: row.get(1)?,
            artifact_version: row.get(2)?,
            target,
            note: row.get(4)?,
            actor: row.get(5)?,
            status,
            side_chat_session_id: row.get(7)?,
            submitted_turn_id: row.get(8)?,
            created_at: DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now),
        })
    }
}
