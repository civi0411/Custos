//! ExecutionWorkspace Persistence Repository
//!
//! SQLite-backed repository implementing CRUD and lifecycle tracking
//! for ExecutionWorkspaces across the 3 workbenches.

use crate::connection::DbConnection;
use async_trait::async_trait;
use custos_core::contracts::workspace::WorkspaceRepository as WorkspaceRepoTrait;
use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage, WorkspaceStatus,
};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct WorkspaceRepository {
    db: DbConnection,
}

impl WorkspaceRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_workspace(&self, ws: &ExecutionWorkspace) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let (kind_type, kind_json) = match &ws.kind {
            WorkspaceKind::Git { .. } => {
                ("git", serde_json::to_string(&ws.kind).map_err(|e| DomainError::Validation(e.to_string()))?)
            }
            WorkspaceKind::Folder { .. } => {
                ("folder", serde_json::to_string(&ws.kind).map_err(|e| DomainError::Validation(e.to_string()))?)
            }
            WorkspaceKind::RemoteSsh { .. } => {
                ("remote_ssh", serde_json::to_string(&ws.kind).map_err(|e| DomainError::Validation(e.to_string()))?)
            }
        };

        let (status_str, status_reason) = match &ws.status {
            WorkspaceStatus::Initializing => ("initializing", None),
            WorkspaceStatus::Ready => ("ready", None),
            WorkspaceStatus::SetupFailed { reason } => ("setup_failed", Some(reason.clone())),
            WorkspaceStatus::Archived => ("archived", None),
        };

        let lineage_json = serde_json::to_string(&ws.lineage)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let metadata_json = serde_json::to_string(&ws.metadata)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO execution_workspaces (
                id, name, kind_type, kind_json, path, status, status_reason, lineage_json, metadata_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                kind_type = excluded.kind_type,
                kind_json = excluded.kind_json,
                path = excluded.path,
                status = excluded.status,
                status_reason = excluded.status_reason,
                lineage_json = excluded.lineage_json,
                metadata_json = excluded.metadata_json,
                updated_at = excluded.updated_at",
            params![
                ws.id.as_str(),
                ws.name,
                kind_type,
                kind_json,
                ws.path,
                status_str,
                status_reason,
                lineage_json,
                metadata_json,
                ws.created_at,
                ws.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_workspace(&self, id: &WorkspaceId) -> Result<Option<ExecutionWorkspace>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, kind_type, kind_json, path, status, status_reason, lineage_json, metadata_json, created_at, updated_at
                 FROM execution_workspaces WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let ws = stmt
            .query_row(params![id.as_str()], Self::map_workspace)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(ws)
    }

    pub fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, kind_type, kind_json, path, status, status_reason, lineage_json, metadata_json, created_at, updated_at
                 FROM execution_workspaces ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_workspace)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let (status_str, reason) = match &status {
            WorkspaceStatus::Initializing => ("initializing", None),
            WorkspaceStatus::Ready => ("ready", None),
            WorkspaceStatus::SetupFailed { reason } => ("setup_failed", Some(reason.clone())),
            WorkspaceStatus::Archived => ("archived", None),
        };
        let updated_at = chrono::Utc::now().to_rfc3339();

        let affected = conn
            .execute(
                "UPDATE execution_workspaces SET status = ?1, status_reason = ?2, updated_at = ?3 WHERE id = ?4",
                params![status_str, reason, updated_at, id.as_str()],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if affected == 0 {
            return Err(DomainError::NotFound {
                kind: "ExecutionWorkspace".into(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    pub fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let affected = conn
            .execute(
                "DELETE FROM execution_workspaces WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if affected == 0 {
            return Err(DomainError::NotFound {
                kind: "ExecutionWorkspace".into(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    fn map_workspace(row: &rusqlite::Row) -> rusqlite::Result<ExecutionWorkspace> {
        let id_str: String = row.get(0)?;
        let name: String = row.get(1)?;
        let _kind_type: String = row.get(2)?;
        let kind_json: String = row.get(3)?;
        let path: String = row.get(4)?;
        let status_str: String = row.get(5)?;
        let status_reason: Option<String> = row.get(6)?;
        let lineage_json: String = row.get(7)?;
        let metadata_json: String = row.get(8)?;
        let created_at: String = row.get(9)?;
        let updated_at: String = row.get(10)?;

        let kind: WorkspaceKind = serde_json::from_str(&kind_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let status = match status_str.as_str() {
            "initializing" => WorkspaceStatus::Initializing,
            "ready" => WorkspaceStatus::Ready,
            "setup_failed" => WorkspaceStatus::SetupFailed {
                reason: status_reason.unwrap_or_default(),
            },
            "archived" => WorkspaceStatus::Archived,
            other => {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    format!("Unknown workspace status: {}", other).into(),
                ))
            }
        };

        let lineage: WorkspaceLineage = serde_json::from_str(&lineage_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let metadata: serde_json::Value = serde_json::from_str(&metadata_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(ExecutionWorkspace {
            id: WorkspaceId::new(id_str),
            name,
            kind,
            path,
            status,
            lineage,
            metadata,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl WorkspaceRepoTrait for WorkspaceRepository {
    async fn save_workspace(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        self.save_workspace(workspace)
    }

    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        self.get_workspace(id)
    }

    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        self.list_workspaces()
    }

    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        self.update_workspace_status(id, status)
    }

    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError> {
        self.delete_workspace(id)
    }
}
