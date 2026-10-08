//! Fleet & Automation Persistence Repository
//!
//! SQLite-backed repository managing:
//! - Scoped browser sessions, tabs, and page snapshots
//! - Remote SSH fleet nodes and command execution receipts
//! - Headless automation jobs and execution logs

use crate::connection::DbConnection;
use custos_domain::{
    BrowserConsoleEntry, BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab,
    BrowserTabStatus, DomainError, FleetExecReceipt, HeadlessAutomationJob, HeadlessJobStatus,
    HeadlessTaskSpec, HeadlessTrigger, RemoteHostNode, RemoteHostStatus, SshAuthMethod,
};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct FleetAutomationRepository {
    db: DbConnection,
}

impl FleetAutomationRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    // =========================================================================
    // Browser Sessions & Tabs
    // =========================================================================

    pub fn save_browser_session(&self, session: &BrowserSession) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            r#"
            INSERT INTO browser_sessions (id, name, workspace_id, active_tab_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                workspace_id = excluded.workspace_id,
                active_tab_id = excluded.active_tab_id,
                updated_at = excluded.updated_at
            "#,
            params![
                session.id,
                session.name,
                session.workspace_id,
                session.active_tab_id,
                session.created_at,
                session.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save browser session: {}", e)))?;
        Ok(())
    }

    pub fn get_browser_session(&self, session_id: &str) -> Result<Option<BrowserSession>, DomainError> {
        let maybe_row = {
            let conn = self.db.lock()?;
            conn.query_row(
                "SELECT id, name, workspace_id, active_tab_id, created_at, updated_at FROM browser_sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to fetch browser session: {}", e)))?
        };

        match maybe_row {
            Some((id, name, workspace_id, active_tab_id, created_at, updated_at)) => {
                let tabs = self.list_browser_tabs(Some(&id))?;
                Ok(Some(BrowserSession {
                    id,
                    name,
                    workspace_id,
                    tabs,
                    active_tab_id,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn list_browser_sessions(&self) -> Result<Vec<BrowserSession>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, name, workspace_id, active_tab_id, created_at, updated_at FROM browser_sessions ORDER BY updated_at DESC")
            .map_err(|e| DomainError::Validation(format!("Failed to prepare browser sessions query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

        let mut sessions = Vec::new();
        for res in rows {
            let (id, name, workspace_id, active_tab_id, created_at, updated_at) =
                res.map_err(|e| DomainError::Validation(format!("Row read error: {}", e)))?;
            sessions.push(BrowserSession {
                id,
                name,
                workspace_id,
                tabs: Vec::new(),
                active_tab_id,
                created_at,
                updated_at,
            });
        }
        Ok(sessions)
    }

    pub fn save_browser_tab(&self, tab: &BrowserTab) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let status_str = match tab.status {
            BrowserTabStatus::Idle => "idle",
            BrowserTabStatus::Loading => "loading",
            BrowserTabStatus::Ready => "ready",
            BrowserTabStatus::Error => "error",
            BrowserTabStatus::Closed => "closed",
        };

        let last_snapshot_json = tab
            .last_snapshot
            .as_ref()
            .map(|s| serde_json::to_string(s).map_err(|e| DomainError::Validation(e.to_string())))
            .transpose()?;

        let console_logs_json = serde_json::to_string(&tab.console_logs)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let network_requests_json = serde_json::to_string(&tab.network_requests)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            r#"
            INSERT INTO browser_tabs (
                id, session_id, url, title, status, active,
                last_snapshot_json, console_logs_json, network_requests_json,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                session_id = excluded.session_id,
                url = excluded.url,
                title = excluded.title,
                status = excluded.status,
                active = excluded.active,
                last_snapshot_json = excluded.last_snapshot_json,
                console_logs_json = excluded.console_logs_json,
                network_requests_json = excluded.network_requests_json,
                updated_at = excluded.updated_at
            "#,
            params![
                tab.id,
                tab.session_id,
                tab.url,
                tab.title,
                status_str,
                if tab.active { 1 } else { 0 },
                last_snapshot_json,
                console_logs_json,
                network_requests_json,
                tab.created_at,
                tab.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save browser tab: {}", e)))?;
        Ok(())
    }

    pub fn get_browser_tab(&self, tab_id: &str) -> Result<Option<BrowserTab>, DomainError> {
        let conn = self.db.lock()?;
        let maybe_row = conn
            .query_row(
                r#"
                SELECT id, session_id, url, title, status, active,
                       last_snapshot_json, console_logs_json, network_requests_json,
                       created_at, updated_at
                FROM browser_tabs
                WHERE id = ?1
                "#,
                params![tab_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, i64>(10)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to query browser tab: {}", e)))?;

        match maybe_row {
            Some(row) => Ok(Some(Self::row_to_browser_tab(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_browser_tabs(&self, session_id: Option<&str>) -> Result<Vec<BrowserTab>, DomainError> {
        let conn = self.db.lock()?;
        let mut tabs = Vec::new();

        if let Some(sid) = session_id {
            let mut stmt = conn
                .prepare("SELECT id, session_id, url, title, status, active, last_snapshot_json, console_logs_json, network_requests_json, created_at, updated_at FROM browser_tabs WHERE session_id = ?1 ORDER BY created_at ASC")
                .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map(params![sid], |row| Self::extract_tab_tuple(row))
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let tuple = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
                tabs.push(Self::row_to_browser_tab(tuple)?);
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT id, session_id, url, title, status, active, last_snapshot_json, console_logs_json, network_requests_json, created_at, updated_at FROM browser_tabs ORDER BY created_at ASC")
                .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map([], |row| Self::extract_tab_tuple(row))
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let tuple = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
                tabs.push(Self::row_to_browser_tab(tuple)?);
            }
        }

        Ok(tabs)
    }

    pub fn delete_browser_tab(&self, tab_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM browser_tabs WHERE id = ?1", params![tab_id])
            .map_err(|e| DomainError::Validation(format!("Failed to delete tab: {}", e)))?;
        Ok(())
    }

    fn extract_tab_tuple(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<(
        String,
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
    )> {
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
            row.get(9)?,
            row.get(10)?,
        ))
    }

    fn row_to_browser_tab(
        row: (
            String,
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            i64,
        ),
    ) -> Result<BrowserTab, DomainError> {
        let (
            id,
            session_id,
            url,
            title,
            status_str,
            active_num,
            last_snapshot_json,
            console_logs_json,
            network_requests_json,
            created_at,
            updated_at,
        ) = row;

        let status = match status_str.as_str() {
            "loading" => BrowserTabStatus::Loading,
            "ready" => BrowserTabStatus::Ready,
            "error" => BrowserTabStatus::Error,
            "closed" => BrowserTabStatus::Closed,
            _ => BrowserTabStatus::Idle,
        };

        let last_snapshot = match last_snapshot_json {
            Some(json) if !json.is_empty() => Some(
                serde_json::from_str::<BrowserPageSnapshot>(&json)
                    .map_err(|e| DomainError::Validation(e.to_string()))?,
            ),
            _ => None,
        };

        let console_logs = match console_logs_json {
            Some(json) if !json.is_empty() => serde_json::from_str::<Vec<BrowserConsoleEntry>>(&json)
                .map_err(|e| DomainError::Validation(e.to_string()))?,
            _ => Vec::new(),
        };

        let network_requests = match network_requests_json {
            Some(json) if !json.is_empty() => {
                serde_json::from_str::<Vec<BrowserNetworkRequest>>(&json)
                    .map_err(|e| DomainError::Validation(e.to_string()))?
            }
            _ => Vec::new(),
        };

        Ok(BrowserTab {
            id,
            session_id,
            url,
            title,
            status,
            active: active_num != 0,
            last_snapshot,
            console_logs,
            network_requests,
            created_at,
            updated_at,
        })
    }

    // =========================================================================
    // Remote SSH Fleet Nodes & Executions
    // =========================================================================

    pub fn save_remote_host(&self, host: &RemoteHostNode) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let auth_json = serde_json::to_string(&host.auth_method)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let labels_json = serde_json::to_string(&host.labels)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status_str = match host.status {
            RemoteHostStatus::Online => "online",
            RemoteHostStatus::Offline => "offline",
            RemoteHostStatus::Degraded => "degraded",
            RemoteHostStatus::AuthFailed => "auth_failed",
        };

        conn.execute(
            r#"
            INSERT INTO remote_fleet_nodes (
                id, name, host, port, user, auth_method_json, status,
                labels_json, last_ping_ms, os_info, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                host = excluded.host,
                port = excluded.port,
                user = excluded.user,
                auth_method_json = excluded.auth_method_json,
                status = excluded.status,
                labels_json = excluded.labels_json,
                last_ping_ms = excluded.last_ping_ms,
                os_info = excluded.os_info,
                updated_at = excluded.updated_at
            "#,
            params![
                host.id,
                host.name,
                host.host,
                host.port,
                host.user,
                auth_json,
                status_str,
                labels_json,
                host.last_ping_ms.map(|v| v as i64),
                host.os_info,
                host.created_at,
                host.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save remote host: {}", e)))?;
        Ok(())
    }

    pub fn get_remote_host(&self, host_id: &str) -> Result<Option<RemoteHostNode>, DomainError> {
        let conn = self.db.lock()?;
        let maybe_row = conn
            .query_row(
                r#"
                SELECT id, name, host, port, user, auth_method_json, status,
                       labels_json, last_ping_ms, os_info, created_at, updated_at
                FROM remote_fleet_nodes
                WHERE id = ?1
                "#,
                params![host_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, u16>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Option<i64>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, i64>(10)?,
                        row.get::<_, i64>(11)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to query remote host: {}", e)))?;

        match maybe_row {
            Some(row) => Ok(Some(self.row_to_remote_host(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_remote_hosts(&self) -> Result<Vec<RemoteHostNode>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, host, port, user, auth_method_json, status,
                       labels_json, last_ping_ms, os_info, created_at, updated_at
                FROM remote_fleet_nodes
                ORDER BY name ASC
                "#,
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u16>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            })
            .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

        let mut hosts = Vec::new();
        for r in rows {
            let row = r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
            hosts.push(self.row_to_remote_host(row)?);
        }
        Ok(hosts)
    }

    fn row_to_remote_host(
        &self,
        row: (
            String,
            String,
            String,
            u16,
            String,
            String,
            String,
            String,
            Option<i64>,
            Option<String>,
            i64,
            i64,
        ),
    ) -> Result<RemoteHostNode, DomainError> {
        let (
            id,
            name,
            host,
            port,
            user,
            auth_json,
            status_str,
            labels_json,
            last_ping_ms,
            os_info,
            created_at,
            updated_at,
        ) = row;

        let auth_method = serde_json::from_str::<SshAuthMethod>(&auth_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let labels = serde_json::from_str::<std::collections::HashMap<String, String>>(&labels_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status = match status_str.as_str() {
            "online" => RemoteHostStatus::Online,
            "degraded" => RemoteHostStatus::Degraded,
            "auth_failed" => RemoteHostStatus::AuthFailed,
            _ => RemoteHostStatus::Offline,
        };

        Ok(RemoteHostNode {
            id,
            name,
            host,
            port,
            user,
            auth_method,
            status,
            labels,
            last_ping_ms: last_ping_ms.map(|v| v as u64),
            os_info,
            created_at,
            updated_at,
        })
    }

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
                .query_map(params![hid], |row| Self::extract_execution_tuple(row))
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let (execution_id, host_id, command, exit_code, stdout, stderr, duration_ms, verified_num, timestamp) =
                    r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
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
                .query_map([], |row| Self::extract_execution_tuple(row))
                .map_err(|e| DomainError::Validation(format!("Query failed: {}", e)))?;

            for r in rows {
                let (execution_id, host_id, command, exit_code, stdout, stderr, duration_ms, verified_num, timestamp) =
                    r.map_err(|e| DomainError::Validation(format!("Row error: {}", e)))?;
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

    fn extract_execution_tuple(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<(
        String,
        String,
        String,
        Option<i32>,
        String,
        String,
        i64,
        i64,
        i64,
    )> {
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

    // =========================================================================
    // Headless Automation Jobs
    // =========================================================================

    pub fn save_headless_job(&self, job: &HeadlessAutomationJob) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let spec_json = serde_json::to_string(&job.spec)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
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

    pub fn get_headless_job(&self, job_id: &str) -> Result<Option<HeadlessAutomationJob>, DomainError> {
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

    fn row_to_headless_job(
        &self,
        row: (
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
        ),
    ) -> Result<HeadlessAutomationJob, DomainError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{BrowserTabStatus, SshAuthMethod};

    fn setup_test_repo() -> FleetAutomationRepository {
        let db = DbConnection::open_in_memory().unwrap();
        FleetAutomationRepository::new(db)
    }

    #[test]
    fn test_browser_sessions_and_tabs_persistence() {
        let repo = setup_test_repo();

        let session = BrowserSession::new("research-browsing", Some("ws_default".to_string()));
        repo.save_browser_session(&session).unwrap();

        let mut tab = BrowserTab::new(&session.id, "https://arxiv.org/abs/2301.00001").unwrap();
        tab.status = BrowserTabStatus::Ready;
        tab.console_logs.push(BrowserConsoleEntry {
            level: "info".to_string(),
            message: "Navigation complete".to_string(),
            timestamp: 1234567,
        });
        repo.save_browser_tab(&tab).unwrap();

        let fetched_tab = repo.get_browser_tab(&tab.id).unwrap().unwrap();
        assert_eq!(fetched_tab.url, "https://arxiv.org/abs/2301.00001");
        assert_eq!(fetched_tab.console_logs.len(), 1);

        let session_with_tabs = repo.get_browser_session(&session.id).unwrap().unwrap();
        assert_eq!(session_with_tabs.tabs.len(), 1);
        assert_eq!(session_with_tabs.tabs[0].id, tab.id);
    }

    #[test]
    fn test_remote_host_and_fleet_execution_persistence() {
        let repo = setup_test_repo();

        let host = RemoteHostNode::new(
            "gpu-node-01",
            "192.168.1.100",
            22,
            "ubuntu",
            SshAuthMethod::KeyPair {
                private_key_path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
        ).unwrap();
        repo.save_remote_host(&host).unwrap();

        let fetched_host = repo.get_remote_host(&host.id).unwrap().unwrap();
        assert_eq!(fetched_host.name, "gpu-node-01");
        assert_eq!(fetched_host.port, 22);

        let receipt = FleetExecReceipt {
            execution_id: "exec_1".to_string(),
            host_id: host.id.clone(),
            command: "nvidia-smi".to_string(),
            exit_code: Some(0),
            stdout: "NVIDIA-SMI 535.129.03 Driver Version: 535.129.03".to_string(),
            stderr: String::new(),
            duration_ms: 45,
            verified: true,
            timestamp: 1234567,
        };
        repo.save_fleet_execution(&receipt).unwrap();

        let receipts = repo.list_fleet_executions(Some(&host.id)).unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].command, "nvidia-smi");
        assert_eq!(receipts[0].exit_code, Some(0));
    }

    #[test]
    fn test_headless_automation_jobs_persistence() {
        let repo = setup_test_repo();

        let spec = HeadlessTaskSpec {
            workspace_id: Some("ws_test".to_string()),
            target_type: "local_process".to_string(),
            command_or_script: "cargo test --workspace".to_string(),
            timeout_secs: 180,
            env: std::collections::HashMap::new(),
            required_evidence: vec!["TestResult".to_string()],
        };

        let mut job = HeadlessAutomationJob::new(
            "continuous-integration",
            spec,
            HeadlessTrigger::Manual,
        ).unwrap();
        job.status = HeadlessJobStatus::Completed;
        job.exit_code = Some(0);
        job.output_log = "All tests passed successfully.".to_string();

        repo.save_headless_job(&job).unwrap();

        let fetched = repo.get_headless_job(&job.id).unwrap().unwrap();
        assert_eq!(fetched.name, "continuous-integration");
        assert_eq!(fetched.status, HeadlessJobStatus::Completed);
        assert_eq!(fetched.exit_code, Some(0));
    }
}
