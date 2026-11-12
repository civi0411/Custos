use std::collections::HashMap;

use custos_domain::{DomainError, RemoteHostNode, RemoteHostStatus, SshAuthMethod};
use rusqlite::{params, OptionalExtension};

use super::FleetAutomationRepository;

type RemoteHostRow = (
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
);

impl FleetAutomationRepository {
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

    fn row_to_remote_host(&self, row: RemoteHostRow) -> Result<RemoteHostNode, DomainError> {
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

        let labels = serde_json::from_str::<HashMap<String, String>>(&labels_json)
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
}

