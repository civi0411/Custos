use crate::connection::DbConnection;
use chrono::Utc;
use custos_contracts::{new_id, DomainError};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigRecord {
    pub id: String,
    pub prefix: String,
    pub display_name: String,
    pub transport_kind: McpTransportKind,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: HashMap<String, String>,
    pub auth_secret_ref: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertMcpServerParams {
    pub id: Option<String>,
    pub prefix: String,
    pub display_name: String,
    pub transport_kind: McpTransportKind,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub auth_secret_ref: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Clone)]
pub struct McpServerRepository {
    db: DbConnection,
}

impl McpServerRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<McpServerConfigRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, prefix, display_name, transport_kind, command, args_json, env_json,
                        cwd, url, headers_json, auth_secret_ref, enabled, created_at, updated_at
                 FROM mcp_servers
                 ORDER BY prefix ASC",
            )
            .map_err(|e| DomainError::Validation(format!("Failed to prepare mcp_servers query: {e}")))?;

        let rows = stmt
            .query_map([], decode_row)
            .map_err(|e| DomainError::Validation(format!("Failed to query mcp_servers: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| DomainError::Validation(format!("MCP server row error: {e}")))?);
        }
        Ok(out)
    }

    pub fn get_by_prefix(&self, prefix: &str) -> Result<Option<McpServerConfigRecord>, DomainError> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT id, prefix, display_name, transport_kind, command, args_json, env_json,
                    cwd, url, headers_json, auth_secret_ref, enabled, created_at, updated_at
             FROM mcp_servers WHERE prefix = ?1",
            params![prefix],
            decode_row,
        )
        .optional()
        .map_err(|e| DomainError::Validation(format!("Failed to get mcp_server '{prefix}': {e}")))
    }

    pub fn upsert(&self, params_in: UpsertMcpServerParams) -> Result<McpServerConfigRecord, DomainError> {
        validate_prefix(&params_in.prefix)?;
        validate_transport(&params_in)?;

        let conn = self.db.lock()?;
        let now = Utc::now().to_rfc3339();
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM mcp_servers WHERE prefix = ?1",
                params![&params_in.prefix],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Validation(format!("Failed to check mcp_server: {e}")))?;

        let id = params_in
            .id
            .or(existing_id)
            .unwrap_or_else(|| new_id("mcpsrv"));
        let transport_str = match params_in.transport_kind {
            McpTransportKind::Stdio => "stdio",
            McpTransportKind::Http => "http",
        };
        let args_json = serde_json::to_string(&params_in.args)
            .map_err(|e| DomainError::Validation(format!("Invalid args JSON: {e}")))?;
        let env_json = serde_json::to_string(&params_in.env)
            .map_err(|e| DomainError::Validation(format!("Invalid env JSON: {e}")))?;
        let headers_json = serde_json::to_string(&params_in.headers)
            .map_err(|e| DomainError::Validation(format!("Invalid headers JSON: {e}")))?;

        conn.execute(
            "INSERT INTO mcp_servers
             (id, prefix, display_name, transport_kind, command, args_json, env_json, cwd, url, headers_json, auth_secret_ref, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
             ON CONFLICT(prefix) DO UPDATE SET
                display_name = excluded.display_name,
                transport_kind = excluded.transport_kind,
                command = excluded.command,
                args_json = excluded.args_json,
                env_json = excluded.env_json,
                cwd = excluded.cwd,
                url = excluded.url,
                headers_json = excluded.headers_json,
                auth_secret_ref = excluded.auth_secret_ref,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
            params![
                &id,
                &params_in.prefix,
                &params_in.display_name,
                transport_str,
                &params_in.command,
                &args_json,
                &env_json,
                &params_in.cwd,
                &params_in.url,
                &headers_json,
                &params_in.auth_secret_ref,
                i32::from(params_in.enabled),
                &now,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to upsert mcp_server: {e}")))?;

        drop(conn);
        self.get_by_prefix(&params_in.prefix)?
            .ok_or_else(|| DomainError::NotFound {
                kind: "mcp_server".into(),
                id: params_in.prefix.clone(),
            })
    }

    pub fn set_enabled(&self, prefix: &str, enabled: bool) -> Result<McpServerConfigRecord, DomainError> {
        let conn = self.db.lock()?;
        let updated = conn
            .execute(
                "UPDATE mcp_servers SET enabled = ?1, updated_at = ?2 WHERE prefix = ?3",
                params![i32::from(enabled), Utc::now().to_rfc3339(), prefix],
            )
            .map_err(|e| DomainError::Validation(format!("Failed to toggle mcp_server: {e}")))?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                kind: "mcp_server".into(),
                id: prefix.to_string(),
            });
        }
        drop(conn);
        self.get_by_prefix(prefix)?
            .ok_or_else(|| DomainError::NotFound {
                kind: "mcp_server".into(),
                id: prefix.to_string(),
            })
    }

    pub fn delete(&self, prefix: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let deleted = conn
            .execute("DELETE FROM mcp_servers WHERE prefix = ?1", params![prefix])
            .map_err(|e| DomainError::Validation(format!("Failed to delete mcp_server: {e}")))?;
        if deleted == 0 {
            return Err(DomainError::NotFound {
                kind: "mcp_server".into(),
                id: prefix.to_string(),
            });
        }
        Ok(())
    }
}

fn validate_prefix(prefix: &str) -> Result<(), DomainError> {
    if prefix.is_empty()
        || prefix.contains("__")
        || !prefix.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(DomainError::Validation(format!(
            "Invalid MCP prefix '{prefix}': use non-empty alphanumeric, '-', or '_' without '__'"
        )));
    }
    Ok(())
}

fn validate_transport(p: &UpsertMcpServerParams) -> Result<(), DomainError> {
    match p.transport_kind {
        McpTransportKind::Stdio => {
            if p.command.as_deref().map(str::trim).unwrap_or("").is_empty() {
                return Err(DomainError::Validation(
                    "Stdio MCP server requires a non-empty command".into(),
                ));
            }
        }
        McpTransportKind::Http => {
            let url = p.url.as_deref().map(str::trim).unwrap_or("");
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(DomainError::Validation(
                    "HTTP MCP server requires an http:// or https:// URL".into(),
                ));
            }
        }
    }
    Ok(())
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerConfigRecord> {
    let transport_raw: String = row.get(3)?;
    let args_json: String = row.get(5)?;
    let env_json: String = row.get(6)?;
    let headers_json: String = row.get(9)?;
    let enabled_int: i32 = row.get(11)?;
    Ok(McpServerConfigRecord {
        id: row.get(0)?,
        prefix: row.get(1)?,
        display_name: row.get(2)?,
        transport_kind: if transport_raw == "http" {
            McpTransportKind::Http
        } else {
            McpTransportKind::Stdio
        },
        command: row.get(4)?,
        args: serde_json::from_str(&args_json).unwrap_or_default(),
        env: serde_json::from_str(&env_json).unwrap_or_default(),
        cwd: row.get(7)?,
        url: row.get(8)?,
        headers: serde_json::from_str(&headers_json).unwrap_or_default(),
        auth_secret_ref: row.get(10)?,
        enabled: enabled_int != 0,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SqliteTaskStore;

    #[test]
    fn mcp_server_repository_crud_and_validation() {
        let store = SqliteTaskStore::new_in_memory().expect("store");
        let repo = store.mcp_servers();

        let created = repo
            .upsert(UpsertMcpServerParams {
                id: None,
                prefix: "github".into(),
                display_name: "GitHub MCP".into(),
                transport_kind: McpTransportKind::Stdio,
                command: Some("npx".into()),
                args: vec!["-y".into(), "@modelcontextprotocol/server-github".into()],
                env: HashMap::new(),
                cwd: None,
                url: None,
                headers: HashMap::new(),
                auth_secret_ref: Some("GITHUB_TOKEN".into()),
                enabled: true,
            })
            .expect("upsert");
        assert_eq!(created.prefix, "github");
        assert!(created.enabled);

        let toggled = repo.set_enabled("github", false).expect("disable");
        assert!(!toggled.enabled);

        assert_eq!(repo.list().expect("list").len(), 1);
        repo.delete("github").expect("delete");
        assert!(repo.list().expect("empty").is_empty());
    }
}
