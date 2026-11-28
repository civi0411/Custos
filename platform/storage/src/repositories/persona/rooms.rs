use super::PersonaRepository;
use custos_contracts::oi::{PersonaRoom, PersonaRoomMessage, PersonaRoomSenderKind};
use custos_contracts::{DomainError, WorkbenchId};
use rusqlite::{params, OptionalExtension};

impl PersonaRepository {
    pub fn create_room_sync(&self, room: &PersonaRoom) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let members_json = serde_json::to_string(&room.member_persona_ids)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        conn.execute(
            "INSERT INTO persona_rooms
             (id, title, member_persona_ids_json, default_workbench, active_session_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &room.id,
                &room.title,
                &members_json,
                room.default_workbench.as_ref().map(|w| w.as_str()),
                room.active_session_id.as_deref(),
                &room.created_at,
                &room.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to create persona room: {e}")))?;
        Ok(())
    }

    pub fn get_room_sync(&self, room_id: &str) -> Result<Option<PersonaRoom>, DomainError> {
        let conn = self.db.reader()?;
        let row = conn
            .query_row(
                "SELECT id, title, member_persona_ids_json, default_workbench, active_session_id, created_at, updated_at
                 FROM persona_rooms WHERE id = ?1",
                params![room_id],
                decode_room_row,
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to get persona room: {e}")))?;
        row.transpose()
    }

    pub fn list_rooms_sync(&self) -> Result<Vec<PersonaRoom>, DomainError> {
        let conn = self.db.reader()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, member_persona_ids_json, default_workbench, active_session_id, created_at, updated_at
                 FROM persona_rooms ORDER BY updated_at DESC",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare persona rooms: {e}")))?;
        let rows = stmt
            .query_map([], decode_room_row)
            .map_err(|e| DomainError::Validation(format!("Failed to list persona rooms: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| DomainError::Validation(e.to_string()))??);
        }
        Ok(out)
    }

    pub fn append_room_message_sync(&self, msg: &PersonaRoomMessage) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO persona_room_messages
             (id, room_id, sender, sender_kind, target_persona_id, root_request_id, content, task_id, run_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &msg.id,
                &msg.room_id,
                &msg.sender,
                sender_kind_to_str(msg.sender_kind),
                msg.target_persona_id.as_deref(),
                msg.root_request_id.as_deref(),
                &msg.content,
                msg.task_id.as_deref(),
                msg.run_id.as_deref(),
                &msg.created_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to append room message: {e}")))?;
        tx.execute(
            "UPDATE persona_rooms SET updated_at = ?1 WHERE id = ?2",
            params![&msg.created_at, &msg.room_id],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to touch room updated_at: {e}")))?;
        tx.commit().map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_room_messages_sync(
        &self,
        room_id: &str,
        limit: usize,
    ) -> Result<Vec<PersonaRoomMessage>, DomainError> {
        let conn = self.db.reader()?;
        let limit_i64 = i64::try_from(limit.max(1)).unwrap_or(50);
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, sender_kind, target_persona_id, root_request_id, content, task_id, run_id, created_at
                 FROM persona_room_messages
                 WHERE room_id = ?1
                 ORDER BY created_at ASC
                 LIMIT ?2",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare room messages: {e}")))?;
        let rows = stmt
            .query_map(params![room_id, limit_i64], |r| {
                let kind_raw: String = r.get(3)?;
                Ok(PersonaRoomMessage {
                    id: r.get(0)?,
                    room_id: r.get(1)?,
                    sender: r.get(2)?,
                    sender_kind: str_to_sender_kind(&kind_raw),
                    target_persona_id: r.get(4)?,
                    root_request_id: r.get(5)?,
                    content: r.get(6)?,
                    task_id: r.get(7)?,
                    run_id: r.get(8)?,
                    created_at: r.get(9)?,
                })
            })
            .map_err(|e| DomainError::Validation(format!("Failed to list room messages: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(out)
    }
}

fn decode_room_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<PersonaRoom, DomainError>> {
    let id: String = row.get(0)?;
    let title: String = row.get(1)?;
    let members_raw: String = row.get(2)?;
    let default_wb: Option<String> = row.get(3)?;
    let active_session_id: Option<String> = row.get(4)?;
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    Ok((|| {
        let member_persona_ids: Vec<String> =
            serde_json::from_str(&members_raw).unwrap_or_default();
        Ok(PersonaRoom {
            id,
            title,
            member_persona_ids,
            default_workbench: default_wb.map(WorkbenchId::new),
            active_session_id,
            created_at,
            updated_at,
        })
    })())
}

fn sender_kind_to_str(k: PersonaRoomSenderKind) -> &'static str {
    match k {
        PersonaRoomSenderKind::Owner => "owner",
        PersonaRoomSenderKind::Persona => "persona",
        PersonaRoomSenderKind::System => "system",
    }
}

fn str_to_sender_kind(s: &str) -> PersonaRoomSenderKind {
    match s {
        "persona" => PersonaRoomSenderKind::Persona,
        "system" => PersonaRoomSenderKind::System,
        _ => PersonaRoomSenderKind::Owner,
    }
}

