use super::PersonaRepository;
use chrono::Utc;
use custos_contracts::oi::{ActsForTrustPolicy, AgentPersonaProfile, MemoryScopeId, WhoCanAskPolicy};
use custos_contracts::{DomainError, WorkbenchId};
use rusqlite::{params, OptionalExtension};

impl PersonaRepository {
    pub fn get_persona_sync(
        &self,
        persona_id: &str,
    ) -> Result<Option<AgentPersonaProfile>, DomainError> {
        let conn = self.db.reader()?;
        let row = conn
            .query_row(
                "SELECT id, name, role, default_workbench, memory_scope,
                        allowed_triggers_json, acts_for_policy_json, sovereign_instructions,
                        look_around_job_id, max_up_next_items, enabled
                 FROM personas WHERE id = ?1",
                params![persona_id],
                decode_persona_row,
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to query persona: {e}")))?;
        row.transpose()
    }

    pub fn list_personas_sync(&self) -> Result<Vec<AgentPersonaProfile>, DomainError> {
        let conn = self.db.reader()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, role, default_workbench, memory_scope,
                        allowed_triggers_json, acts_for_policy_json, sovereign_instructions,
                        look_around_job_id, max_up_next_items, enabled
                 FROM personas ORDER BY name ASC",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare personas list: {e}")))?;
        let rows = stmt
            .query_map([], decode_persona_row)
            .map_err(|e| DomainError::Validation(format!("Failed to list personas: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| DomainError::Validation(e.to_string()))??);
        }
        Ok(out)
    }

    pub fn upsert_persona_sync(&self, profile: &AgentPersonaProfile) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = Utc::now().to_rfc3339();
        let triggers_json = serde_json::to_string(&profile.allowed_triggers)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let acts_for_json = serde_json::to_string(&profile.acts_for)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        conn.execute(
            "INSERT INTO personas
             (id, name, role, default_workbench, memory_scope, allowed_triggers_json,
              acts_for_policy_json, sovereign_instructions, look_around_job_id,
              max_up_next_items, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               role = excluded.role,
               default_workbench = excluded.default_workbench,
               memory_scope = excluded.memory_scope,
               allowed_triggers_json = excluded.allowed_triggers_json,
               acts_for_policy_json = excluded.acts_for_policy_json,
               sovereign_instructions = excluded.sovereign_instructions,
               look_around_job_id = excluded.look_around_job_id,
               max_up_next_items = excluded.max_up_next_items,
               enabled = excluded.enabled,
               updated_at = excluded.updated_at",
            params![
                &profile.id,
                &profile.name,
                &profile.role,
                profile.default_workbench.as_str(),
                profile.memory_scope.as_str(),
                &triggers_json,
                &acts_for_json,
                &profile.sovereign_instructions,
                profile.look_around_job_id.as_deref(),
                i64::from(profile.max_up_next_items),
                i32::from(profile.enabled),
                &now,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to upsert persona: {e}")))?;
        Ok(())
    }

    pub fn set_persona_enabled_sync(
        &self,
        persona_id: &str,
        enabled: bool,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = Utc::now().to_rfc3339();
        let updated = conn
            .execute(
                "UPDATE personas SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![i32::from(enabled), &now, persona_id],
            )
            .map_err(|e| DomainError::Validation(format!("Failed to update persona: {e}")))?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                kind: "persona".into(),
                id: persona_id.to_string(),
            });
        }
        Ok(())
    }
}

fn decode_persona_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<AgentPersonaProfile, DomainError>> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let role: String = row.get(2)?;
    let workbench: String = row.get(3)?;
    let memory_scope: String = row.get(4)?;
    let triggers_raw: String = row.get(5)?;
    let acts_for_raw: String = row.get(6)?;
    let sovereign_instructions: String = row.get(7)?;
    let look_around_job_id: Option<String> = row.get(8)?;
    let max_up_next_items: i64 = row.get(9)?;
    let enabled: i32 = row.get(10)?;

    Ok((|| {
        let allowed_triggers: WhoCanAskPolicy = serde_json::from_str(&triggers_raw)
            .map_err(|e| DomainError::Validation(format!("Invalid triggers json: {e}")))?;
        let acts_for: ActsForTrustPolicy = serde_json::from_str(&acts_for_raw)
            .map_err(|e| DomainError::Validation(format!("Invalid acts_for json: {e}")))?;
        Ok(AgentPersonaProfile {
            id,
            name,
            role,
            default_workbench: WorkbenchId::new(workbench),
            memory_scope: MemoryScopeId::new(memory_scope),
            allowed_triggers,
            acts_for,
            sovereign_instructions,
            look_around_job_id,
            max_up_next_items: u8::try_from(max_up_next_items.clamp(0, 50)).unwrap_or(5),
            enabled: enabled != 0,
        })
    })())
}

