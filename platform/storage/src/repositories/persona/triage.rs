use super::PersonaRepository;
use chrono::Utc;
use custos_contracts::oi::{
    PersonaAskQuestion, PersonaItem, PersonaItemBucket, PersonaItemKind, PersonaItemSource,
};
use custos_contracts::DomainError;
use rusqlite::{params, OptionalExtension};

impl PersonaRepository {
    pub fn get_item_sync(&self, item_id: &str) -> Result<Option<PersonaItem>, DomainError> {
        let conn = self.db.reader()?;
        let row = conn
            .query_row(
                "SELECT id, persona_id, bucket, kind, title, summary, ask_questions_json,
                        source_kind, source_id, task_id, run_id, priority, payload_json,
                        created_at, updated_at, resolved_at
                 FROM persona_items WHERE id = ?1",
                params![item_id],
                decode_item_row,
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to get persona item: {e}")))?;
        row.transpose()
    }

    pub fn list_items_sync(
        &self,
        persona_id: Option<&str>,
        bucket: Option<PersonaItemBucket>,
    ) -> Result<Vec<PersonaItem>, DomainError> {
        let conn = self.db.reader()?;
        let bucket_str = bucket.map(bucket_to_str);
        let mut stmt = conn
            .prepare(
                "SELECT id, persona_id, bucket, kind, title, summary, ask_questions_json,
                        source_kind, source_id, task_id, run_id, priority, payload_json,
                        created_at, updated_at, resolved_at
                 FROM persona_items
                 WHERE (?1 IS NULL OR persona_id = ?1)
                   AND (?2 IS NULL OR bucket = ?2)
                 ORDER BY priority DESC, updated_at DESC",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare persona_items: {e}")))?;
        let rows = stmt
            .query_map(params![persona_id, bucket_str], decode_item_row)
            .map_err(|e| DomainError::Validation(format!("Failed to list persona_items: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| DomainError::Validation(e.to_string()))??);
        }
        Ok(out)
    }

    pub fn upsert_item_sync(&self, item: &PersonaItem) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let ask_json = serde_json::to_string(&item.ask_questions)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let payload_json = serde_json::to_string(&item.payload_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let (source_kind, source_id) = match &item.source_ref {
            Some(s) => (Some(s.source_kind.as_str()), Some(s.source_id.as_str())),
            None => (None, None),
        };
        conn.execute(
            "INSERT INTO persona_items
             (id, persona_id, bucket, kind, title, summary, ask_questions_json,
              source_kind, source_id, task_id, run_id, priority, payload_json,
              created_at, updated_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
               bucket = excluded.bucket,
               kind = excluded.kind,
               title = excluded.title,
               summary = excluded.summary,
               ask_questions_json = excluded.ask_questions_json,
               source_kind = excluded.source_kind,
               source_id = excluded.source_id,
               task_id = excluded.task_id,
               run_id = excluded.run_id,
               priority = excluded.priority,
               payload_json = excluded.payload_json,
               updated_at = excluded.updated_at,
               resolved_at = excluded.resolved_at",
            params![
                &item.id,
                &item.persona_id,
                bucket_to_str(item.bucket),
                kind_to_str(item.kind),
                &item.title,
                &item.summary,
                &ask_json,
                source_kind,
                source_id,
                item.task_id.as_deref(),
                item.run_id.as_deref(),
                item.priority,
                &payload_json,
                &item.created_at,
                &item.updated_at,
                item.resolved_at.as_deref(),
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to upsert persona item: {e}")))?;
        Ok(())
    }

    pub fn resolve_item_sync(&self, item_id: &str, resolved_at: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = Utc::now().to_rfc3339();
        let updated = conn
            .execute(
                "UPDATE persona_items SET bucket = 'handled', resolved_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![resolved_at, &now, item_id],
            )
            .map_err(|e| DomainError::Validation(format!("Failed to resolve persona item: {e}")))?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                kind: "persona_item".into(),
                id: item_id.to_string(),
            });
        }
        Ok(())
    }
}

fn decode_item_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<PersonaItem, DomainError>> {
    let id: String = row.get(0)?;
    let persona_id: String = row.get(1)?;
    let bucket_raw: String = row.get(2)?;
    let kind_raw: String = row.get(3)?;
    let title: String = row.get(4)?;
    let summary: String = row.get(5)?;
    let ask_raw: String = row.get(6)?;
    let source_kind: Option<String> = row.get(7)?;
    let source_id: Option<String> = row.get(8)?;
    let task_id: Option<String> = row.get(9)?;
    let run_id: Option<String> = row.get(10)?;
    let priority: i32 = row.get(11)?;
    let payload_raw: String = row.get(12)?;
    let created_at: String = row.get(13)?;
    let updated_at: String = row.get(14)?;
    let resolved_at: Option<String> = row.get(15)?;

    Ok((|| {
        let ask_questions: Vec<PersonaAskQuestion> =
            serde_json::from_str(&ask_raw).unwrap_or_default();
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload_raw).unwrap_or(serde_json::Value::Object(Default::default()));
        let source_ref = match (source_kind, source_id) {
            (Some(k), Some(i)) => Some(PersonaItemSource {
                source_kind: k,
                source_id: i,
            }),
            _ => None,
        };
        Ok(PersonaItem {
            id,
            persona_id,
            bucket: str_to_bucket(&bucket_raw),
            kind: str_to_kind(&kind_raw),
            title,
            summary,
            ask_questions,
            source_ref,
            task_id,
            run_id,
            priority,
            payload_json,
            created_at,
            updated_at,
            resolved_at,
        })
    })())
}

fn bucket_to_str(b: PersonaItemBucket) -> &'static str {
    match b {
        PersonaItemBucket::NeedsYou => "needs_you",
        PersonaItemBucket::UpNext => "up_next",
        PersonaItemBucket::Handled => "handled",
    }
}

fn str_to_bucket(s: &str) -> PersonaItemBucket {
    match s {
        "up_next" => PersonaItemBucket::UpNext,
        "handled" => PersonaItemBucket::Handled,
        _ => PersonaItemBucket::NeedsYou,
    }
}

fn kind_to_str(k: PersonaItemKind) -> &'static str {
    match k {
        PersonaItemKind::PermitApproval => "permit_approval",
        PersonaItemKind::HookPermission => "hook_permission",
        PersonaItemKind::MemoryProposal => "memory_proposal",
        PersonaItemKind::LookAroundFinding => "look_around_finding",
        PersonaItemKind::TaskResult => "task_result",
    }
}

fn str_to_kind(s: &str) -> PersonaItemKind {
    match s {
        "hook_permission" => PersonaItemKind::HookPermission,
        "memory_proposal" => PersonaItemKind::MemoryProposal,
        "look_around_finding" => PersonaItemKind::LookAroundFinding,
        "task_result" => PersonaItemKind::TaskResult,
        _ => PersonaItemKind::PermitApproval,
    }
}

