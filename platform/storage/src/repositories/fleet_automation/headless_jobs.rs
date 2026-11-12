use custos_domain::{
    DomainError, HeadlessAutomationJob, HeadlessJobStatus, HeadlessTaskSpec, HeadlessTrigger,
};
use rusqlite::{params, OptionalExtension};

use super::FleetAutomationRepository;

type HeadlessJobRow = (
    String,
    String,
    String,
    String,
    String,
    Option<i32>,
    String,
    i64,
    Option<i64>,
    Option<i64>,
);

impl FleetAutomationRepository {
    pub fn save_headless_job(&self, job: &HeadlessAutomationJob) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let spec_json =
            serde_json::to_string(&job.spec).map_err(|e| DomainError::Validation(e.to_string()))?;
        let trigger_json = serde_json::to_string(&job.trigger)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status_str = match job.status {
            HeadlessJobStatus::Pending => "pending",
            HeadlessJobStatus::Running => "running",
            HeadlessJobStatus::Completed => "completed",
            HeadlessJobStatus::Failed => "failed",
            HeadlessJobStatus::Cancelled => "cancelled",
            HeadlessJobStatus::TimedOut => "timed_out",
        };

        conn.execute(
            r#"
            INSERT INTO headless_automation_jobs (
                id, name, spec_json, trigger_json, status, exit_code,
                output_log, created_at, started_at, completed_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                spec_json = excluded.spec_json,
                trigger_json = excluded.trigger_json,
                status = excluded.status,
                exit_code = excluded.exit_code,
                output_log = excluded.output_log,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at
            "#,
            params![
                job.id,
                job.name,
                spec_json,
                trigger_json,
                status_str,
                job.exit_code,
                job.output_log,
                job.created_at,
                job.started_at,
                job.completed_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save headless job: {}", e)))?;
        Ok(())
    }

    pub fn get_headless_job(
        &self,
        job_id: &str,
    ) -> Result<Option<HeadlessAutomationJob>, DomainError> {
        let conn = self.db.lock()?;
        let maybe_row = conn
            .query_row(
                r#"
                SELECT id, name, spec_json, trigger_json, status, exit_code,
                       output_log, created_at, started_at, completed_at
                FROM headless_automation_jobs
                WHERE id = ?1
                "#,
                params![job_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<i32>>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, Option<i64>>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to query headless job: {}", e)))?;

        match maybe_row {
            Some(row) => Ok(Some(self.row_to_headless_job(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_headless_jobs(&self) -> Result<Vec<HeadlessAutomationJob>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, spec_json, trigger_json, status, exit_code,
                       output_log, created_at, started_at, completed_at
                FROM headless_automation_jobs
                ORDER BY created_at DESC
                "#,
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i32>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                ))
            })
            .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

        let mut jobs = Vec::new();
        for r in rows {
            let row = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
            jobs.push(self.row_to_headless_job(row)?);
        }
        Ok(jobs)
    }

    fn row_to_headless_job(&self, row: HeadlessJobRow) -> Result<HeadlessAutomationJob, DomainError> {
        let (
            id,
            name,
            spec_json,
            trigger_json,
            status_str,
            exit_code,
            output_log,
            created_at,
            started_at,
            completed_at,
        ) = row;

        let spec = serde_json::from_str::<HeadlessTaskSpec>(&spec_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let trigger = serde_json::from_str::<HeadlessTrigger>(&trigger_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status = match status_str.as_str() {
            "running" => HeadlessJobStatus::Running,
            "completed" => HeadlessJobStatus::Completed,
            "failed" => HeadlessJobStatus::Failed,
            "cancelled" => HeadlessJobStatus::Cancelled,
            "timed_out" => HeadlessJobStatus::TimedOut,
            _ => HeadlessJobStatus::Pending,
        };

        Ok(HeadlessAutomationJob {
            id,
            name,
            spec,
            trigger,
            status,
            exit_code,
            output_log,
            created_at,
            started_at,
            completed_at,
        })
    }
}

