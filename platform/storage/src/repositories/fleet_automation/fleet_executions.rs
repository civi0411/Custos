use custos_domain::{DomainError, FleetExecReceipt};
use rusqlite::params;

use super::FleetAutomationRepository;

type FleetExecutionRow = (
    String,
    String,
    String,
    Option<i32>,
    String,
    String,
    i64,
    i64,
    i64,
);

impl FleetAutomationRepository {
    pub fn save_fleet_execution(&self, receipt: &FleetExecReceipt) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            r#"
            INSERT INTO remote_fleet_executions (
                execution_id, host_id, command, exit_code, stdout, stderr,
                duration_ms, verified, timestamp
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                receipt.execution_id,
                receipt.host_id,
                receipt.command,
                receipt.exit_code,
                receipt.stdout,
                receipt.stderr,
                receipt.duration_ms as i64,
                if receipt.verified { 1 } else { 0 },
                receipt.timestamp,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save fleet execution: {}", e)))?;
        Ok(())
    }

    pub fn list_fleet_executions(
        &self,
        host_id: Option<&str>,
    ) -> Result<Vec<FleetExecReceipt>, DomainError> {
        let conn = self.db.lock()?;
        let mut receipts = Vec::new();

        if let Some(hid) = host_id {
            let mut stmt = conn
                .prepare("SELECT execution_id, host_id, command, exit_code, stdout, stderr, duration_ms, verified, timestamp FROM remote_fleet_executions WHERE host_id = ?1 ORDER BY timestamp DESC LIMIT 50")
                .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map(params![hid], Self::extract_execution_tuple)
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let (
                    execution_id,
                    host_id,
                    command,
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms,
                    verified_num,
                    timestamp,
                ) = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
                receipts.push(FleetExecReceipt {
                    execution_id,
                    host_id,
                    command,
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms: duration_ms as u64,
                    verified: verified_num != 0,
                    timestamp,
                });
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT execution_id, host_id, command, exit_code, stdout, stderr, duration_ms, verified, timestamp FROM remote_fleet_executions ORDER BY timestamp DESC LIMIT 50")
                .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map([], Self::extract_execution_tuple)
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let (
                    execution_id,
                    host_id,
                    command,
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms,
                    verified_num,
                    timestamp,
                ) = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
                receipts.push(FleetExecReceipt {
                    execution_id,
                    host_id,
                    command,
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms: duration_ms as u64,
                    verified: verified_num != 0,
                    timestamp,
                });
            }
        }

        Ok(receipts)
    }

    fn extract_execution_tuple(row: &rusqlite::Row<'_>) -> rusqlite::Result<FleetExecutionRow> {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
        ))
    }
}

