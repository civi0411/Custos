use super::ResearchRepository;
use chrono::{DateTime, Utc};
use custos_domain::{
    DomainError, EnvSnapshot, EnvironmentSpec, ExecutionArtifact, ExecutionRecord,
    ExecutionRecordStatus, Recipe, RecipeInput, ResearchExperimentRun,
};
use rusqlite::{params, types::Type, OptionalExtension};

impl ResearchRepository {
    // Runs Ledger
    pub fn save_run(&self, run: &ResearchExperimentRun) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let env_json = serde_json::to_string(&run.env_snapshot)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO research_experiment_runs (
                run_id, session_id, command, cwd, status, wall_ms, surface, reproducibility, input_merkle_root, output_merkle_root, env_snapshot_json, sade_permit_id, cas_log_uri, ts
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                run.run_id,
                run.session_id,
                run.command,
                run.cwd,
                run.status,
                run.wall_ms as i64,
                run.surface,
                run.reproducibility,
                run.input_merkle_root,
                run.output_merkle_root,
                env_json,
                run.sade_permit_id,
                run.cas_log_uri,
                run.ts,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_runs(&self) -> Result<Vec<ResearchExperimentRun>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT run_id, session_id, command, cwd, status, wall_ms, surface, reproducibility, input_merkle_root, output_merkle_root, env_snapshot_json, sade_permit_id, cas_log_uri, ts
                 FROM research_experiment_runs ORDER BY ts DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let wall: i64 = row.get(5)?;
                let env_json: String = row.get(10)?;
                let env: EnvSnapshot = serde_json::from_str(&env_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(10, Type::Text, Box::new(error))
                })?;

                Ok(ResearchExperimentRun {
                    run_id: row.get(0)?,
                    session_id: row.get(1)?,
                    command: row.get(2)?,
                    cwd: row.get(3)?,
                    status: row.get(4)?,
                    wall_ms: wall as u64,
                    surface: row.get(6)?,
                    reproducibility: row.get(7)?,
                    input_merkle_root: row.get(8)?,
                    output_merkle_root: row.get(9)?,
                    env_snapshot: env,
                    sade_permit_id: row.get(11)?,
                    cas_log_uri: row.get(12)?,
                    ts: row.get(13)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Recipes
    pub fn save_recipe(&self, recipe: &Recipe) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let env_spec_json = serde_json::to_string(&recipe.environment_spec)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let inputs_json = serde_json::to_string(&recipe.inputs)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let outputs_json = serde_json::to_string(&recipe.outputs)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO research_recipes (
                id, name, description, command, environment_spec_json, inputs_json, outputs_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                command = excluded.command,
                environment_spec_json = excluded.environment_spec_json,
                inputs_json = excluded.inputs_json,
                outputs_json = excluded.outputs_json",
            params![
                recipe.id,
                recipe.name,
                recipe.description,
                recipe.command,
                env_spec_json,
                inputs_json,
                outputs_json,
                recipe.created_at.timestamp(),
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_recipe(&self, id: &str) -> Result<Option<Recipe>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, command, environment_spec_json, inputs_json, outputs_json, created_at
                 FROM research_recipes WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let env_json: String = row.get(4)?;
                let inputs_json: String = row.get(5)?;
                let outputs_json: String = row.get(6)?;
                let ts: i64 = row.get(7)?;

                let environment_spec: EnvironmentSpec =
                    serde_json::from_str(&env_json).unwrap_or_else(|_| EnvironmentSpec::new());
                let inputs: Vec<RecipeInput> =
                    serde_json::from_str(&inputs_json).unwrap_or_default();
                let outputs: Vec<String> = serde_json::from_str(&outputs_json).unwrap_or_default();

                Ok(Recipe {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    command: row.get(3)?,
                    environment_spec,
                    inputs,
                    outputs,
                    created_at: DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now),
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn list_recipes(&self) -> Result<Vec<Recipe>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, command, environment_spec_json, inputs_json, outputs_json, created_at
                 FROM research_recipes ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let env_json: String = row.get(4)?;
                let inputs_json: String = row.get(5)?;
                let outputs_json: String = row.get(6)?;
                let ts: i64 = row.get(7)?;

                let environment_spec: EnvironmentSpec =
                    serde_json::from_str(&env_json).unwrap_or_else(|_| EnvironmentSpec::new());
                let inputs: Vec<RecipeInput> =
                    serde_json::from_str(&inputs_json).unwrap_or_default();
                let outputs: Vec<String> = serde_json::from_str(&outputs_json).unwrap_or_default();

                Ok(Recipe {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    command: row.get(3)?,
                    environment_spec,
                    inputs,
                    outputs,
                    created_at: DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now),
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Execution Records
    pub fn save_execution_record(&self, record: &ExecutionRecord) -> Result<(), DomainError> {
        record.validate_compute_state()?;
        let snapshot = serde_json::to_string(record)
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let mut conn = self.db.lock()?;
        let tx = conn.transaction().map_err(|error| DomainError::Validation(error.to_string()))?;
        let previous: Option<String> = tx.query_row(
            "SELECT record_json FROM research_execution_records WHERE id = ?1",
            params![record.id], |row| row.get(0),
        ).optional().map_err(|error| DomainError::Validation(error.to_string()))?.flatten();
        let is_new = previous.is_none();
        if let Some(previous) = previous {
            let previous: ExecutionRecord = serde_json::from_str(&previous)
                .map_err(|error| DomainError::Validation(format!("Invalid stored execution snapshot: {error}")))?;
            if previous.latest_protected_checkpoint_step.is_some_and(|step|
                record.latest_protected_checkpoint_step.is_none_or(|next| next < step))
                || previous.latest_observed_step.is_some_and(|step|
                    record.latest_observed_step.is_none_or(|next| next < step))
                || previous.latest_local_checkpoint_step.is_some_and(|step|
                    record.latest_local_checkpoint_step.is_none_or(|next| next < step))
                || (previous.backend.is_some() && previous.backend != record.backend)
                || previous.resume_from_attempt_id != record.resume_from_attempt_id
                || previous.resumed_from_checkpoint_id != record.resumed_from_checkpoint_id
                || previous.output_namespace != record.output_namespace
            {
                return Err(DomainError::Conflict("Execution lineage, output namespace and protected progress cannot be reset".into()));
            }
        }
        if let Some(parent_id) = record.resume_from_attempt_id.as_ref().filter(|_| is_new) {
            let parent: Option<String> = tx.query_row(
                "SELECT record_json FROM research_execution_records WHERE id = ?1 AND recipe_id = ?2",
                params![parent_id, record.recipe_id], |row| row.get(0),
            ).optional().map_err(|error| DomainError::Validation(error.to_string()))?.flatten();
            let parent: ExecutionRecord = serde_json::from_str(&parent.ok_or_else(||
                DomainError::Validation("Continuation parent snapshot was not found for this recipe".into()))?)
                .map_err(|error| DomainError::Validation(error.to_string()))?;
            let checkpoint = parent.checkpoint.as_ref().ok_or_else(||
                DomainError::Validation("Continuation parent has no checkpoint".into()))?;
            checkpoint.validate_for_state_resume()?;
            if record.resumed_from_checkpoint_id.as_deref() != Some(checkpoint.checkpoint_id.as_str())
                || record.output_namespace.as_ref().is_none_or(|ns| ns.trim().is_empty())
                || record.output_namespace == parent.output_namespace
            {
                return Err(DomainError::Validation("Continuation must select the parent's protected checkpoint and an isolated output namespace".into()));
            }
        }
        let status_str = match record.status {
            ExecutionRecordStatus::Pending => "pending",
            ExecutionRecordStatus::Running => "running",
            ExecutionRecordStatus::Completed => "completed",
            ExecutionRecordStatus::Failed => "failed",
            ExecutionRecordStatus::Cancelled => "cancelled",
            ExecutionRecordStatus::Preempted => "preempted",
        };
        let artifacts_json = serde_json::to_string(&record.artifacts)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let affected = tx.execute(
            "INSERT INTO research_execution_records (
                id, recipe_id, session_id, status, exit_code, stdout_cas_uri, stderr_cas_uri,
                started_at, ended_at, artifacts_json, record_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                exit_code = excluded.exit_code,
                stdout_cas_uri = excluded.stdout_cas_uri,
                stderr_cas_uri = excluded.stderr_cas_uri,
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                artifacts_json = excluded.artifacts_json,
                record_json = excluded.record_json
            WHERE research_execution_records.recipe_id = excluded.recipe_id
              AND research_execution_records.session_id IS excluded.session_id",
            params![
                record.id,
                record.recipe_id,
                record.session_id,
                status_str,
                record.exit_code,
                record.stdout_cas_uri,
                record.stderr_cas_uri,
                record.started_at.timestamp(),
                record.ended_at.map(|dt| dt.timestamp()),
                artifacts_json,
                snapshot,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        if affected != 1 {
            return Err(DomainError::Conflict("Execution recipe and session identities are immutable".into()));
        }
        tx.execute("INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![custos_domain::new_id("exec_event"), "research.execution.saved",
                serde_json::json!({"execution_id": record.id, "recipe_id": record.recipe_id,
                    "protected_step": record.latest_protected_checkpoint_step}).to_string(), Utc::now().timestamp()])
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        tx.commit().map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(())
    }

    fn decode_execution_snapshot(snapshot: &str) -> rusqlite::Result<ExecutionRecord> {
        let record: ExecutionRecord = serde_json::from_str(snapshot)
            .map_err(|error| rusqlite::Error::FromSqlConversionFailure(10, Type::Text, Box::new(error)))?;
        record.validate_compute_state()
            .map_err(|error| rusqlite::Error::FromSqlConversionFailure(10, Type::Text, Box::new(error)))?;
        Ok(record)
    }

    pub fn get_execution_record(&self, id: &str) -> Result<Option<ExecutionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, recipe_id, session_id, status, exit_code, stdout_cas_uri, stderr_cas_uri,
                        started_at, ended_at, artifacts_json, record_json
                 FROM research_execution_records WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                if let Some(snapshot) = row.get::<_, Option<String>>(10)? {
                    let record = Self::decode_execution_snapshot(&snapshot)?;
                    if record.id != row.get::<_, String>(0)? || record.recipe_id != row.get::<_, String>(1)?
                        || record.session_id != row.get::<_, Option<String>>(2)? {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                    return Ok(record);
                }
                let status_str: String = row.get(3)?;
                let status = match status_str.as_str() {
                    "running" => ExecutionRecordStatus::Running,
                    "completed" => ExecutionRecordStatus::Completed,
                    "failed" => ExecutionRecordStatus::Failed,
                    "cancelled" => ExecutionRecordStatus::Cancelled,
                    "preempted" => ExecutionRecordStatus::Preempted,
                    "pending" => ExecutionRecordStatus::Pending,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let started_ts: i64 = row.get(7)?;
                let ended_ts: Option<i64> = row.get(8)?;
                let artifacts_json: String = row.get(9)?;
                let artifacts: Vec<ExecutionArtifact> =
                    serde_json::from_str(&artifacts_json).map_err(|error|
                        rusqlite::Error::FromSqlConversionFailure(9, Type::Text, Box::new(error)))?;

                Ok(ExecutionRecord {
                    id: row.get(0)?,
                    recipe_id: row.get(1)?,
                    session_id: row.get(2)?,
                    status,
                    exit_code: row.get(4)?,
                    stdout_cas_uri: row.get(5)?,
                    stderr_cas_uri: row.get(6)?,
                    started_at: DateTime::from_timestamp(started_ts, 0).unwrap_or_else(Utc::now),
                    ended_at: ended_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    artifacts,
                    checkpoint: None,
                    resume_from_attempt_id: None,
                    resumed_from_checkpoint_id: None,
                    backend: None,
                    output_namespace: None,
                    latest_observed_step: None,
                    latest_local_checkpoint_step: None,
                    latest_protected_checkpoint_step: None,
                    last_protected_at: None,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn list_execution_records_for_recipe(
        &self,
        recipe_id: &str,
    ) -> Result<Vec<ExecutionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, recipe_id, session_id, status, exit_code, stdout_cas_uri, stderr_cas_uri,
                        started_at, ended_at, artifacts_json, record_json
                 FROM research_execution_records WHERE recipe_id = ?1 ORDER BY started_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![recipe_id], |row| {
                if let Some(snapshot) = row.get::<_, Option<String>>(10)? {
                    let record = Self::decode_execution_snapshot(&snapshot)?;
                    if record.id != row.get::<_, String>(0)? || record.recipe_id != row.get::<_, String>(1)?
                        || record.session_id != row.get::<_, Option<String>>(2)? {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                    return Ok(record);
                }
                let status_str: String = row.get(3)?;
                let status = match status_str.as_str() {
                    "running" => ExecutionRecordStatus::Running,
                    "completed" => ExecutionRecordStatus::Completed,
                    "failed" => ExecutionRecordStatus::Failed,
                    "cancelled" => ExecutionRecordStatus::Cancelled,
                    "preempted" => ExecutionRecordStatus::Preempted,
                    "pending" => ExecutionRecordStatus::Pending,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let started_ts: i64 = row.get(7)?;
                let ended_ts: Option<i64> = row.get(8)?;
                let artifacts_json: String = row.get(9)?;
                let artifacts: Vec<ExecutionArtifact> =
                    serde_json::from_str(&artifacts_json).map_err(|error|
                        rusqlite::Error::FromSqlConversionFailure(9, Type::Text, Box::new(error)))?;

                Ok(ExecutionRecord {
                    id: row.get(0)?,
                    recipe_id: row.get(1)?,
                    session_id: row.get(2)?,
                    status,
                    exit_code: row.get(4)?,
                    stdout_cas_uri: row.get(5)?,
                    stderr_cas_uri: row.get(6)?,
                    started_at: DateTime::from_timestamp(started_ts, 0).unwrap_or_else(Utc::now),
                    ended_at: ended_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    artifacts,
                    checkpoint: None,
                    resume_from_attempt_id: None,
                    resumed_from_checkpoint_id: None,
                    backend: None,
                    output_namespace: None,
                    latest_observed_step: None,
                    latest_local_checkpoint_step: None,
                    latest_protected_checkpoint_step: None,
                    last_protected_at: None,
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
