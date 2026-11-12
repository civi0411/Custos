use crate::connection::DbConnection;
use custos_domain::{
    BrowserConsoleEntry, BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab,
    BrowserTabStatus, DomainError,
};
use rusqlite::{params, OptionalExtension};

type BrowserTabRow = (
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
);

/// SQLite-backed repository managing scoped browser sessions, tabs, and snapshots.
#[derive(Clone)]
pub struct BrowserRepository {
    pub(crate) db: DbConnection,
}

impl BrowserRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
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

    pub fn get_browser_session(
        &self,
        session_id: &str,
    ) -> Result<Option<BrowserSession>, DomainError> {
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

    pub fn list_browser_tabs(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<BrowserTab>, DomainError> {
        let conn = self.db.lock()?;
        let mut tabs = Vec::new();

        if let Some(sid) = session_id {
            let mut stmt = conn
                .prepare("SELECT id, session_id, url, title, status, active, last_snapshot_json, console_logs_json, network_requests_json, created_at, updated_at FROM browser_tabs WHERE session_id = ?1 ORDER BY created_at ASC")
                .map_err(|e| DomainError::Validation(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map(params![sid], Self::extract_tab_tuple)
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
                .query_map([], Self::extract_tab_tuple)
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

    fn extract_tab_tuple(row: &rusqlite::Row<'_>) -> rusqlite::Result<BrowserTabRow> {
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

    fn row_to_browser_tab(row: BrowserTabRow) -> Result<BrowserTab, DomainError> {
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
            Some(json) if !json.is_empty() => {
                serde_json::from_str::<Vec<BrowserConsoleEntry>>(&json)
                    .map_err(|e| DomainError::Validation(e.to_string()))?
            }
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
}

