use crate::connection::DbConnection;
use custos_domain::{DomainError, NodePlacement, RevisionNode, WorkflowRevision};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct WorkflowRevisionRepository {
    db: DbConnection,
}

impl WorkflowRevisionRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_revision(
        &self,
        revision: &WorkflowRevision,
        placements: &[NodePlacement],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let nodes_json = serde_json::to_string(&revision.nodes)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let deps_json = serde_json::to_string(&revision.dependencies)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let obligations_json = serde_json::to_string(&revision.obligations)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "INSERT INTO workflow_revisions (
                revision_id, task_id, proposal_id, revision_number,
                nodes_json, dependencies_json, obligations_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(revision_id) DO UPDATE SET
                task_id = excluded.task_id,
                proposal_id = excluded.proposal_id,
                revision_number = excluded.revision_number,
                nodes_json = excluded.nodes_json,
                dependencies_json = excluded.dependencies_json,
                obligations_json = excluded.obligations_json,
                created_at = excluded.created_at",
            params![
                revision.revision_id,
                revision.task_id,
                revision.proposal_id,
                revision.revision_number,
                nodes_json,
                deps_json,
                obligations_json,
                revision.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        for placement in placements {
            tx.execute(
                "INSERT INTO node_placements (
                    revision_id, node_id, role, backend_harness,
                    budget_tokens_slice, workspace_lease_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(revision_id, node_id) DO UPDATE SET
                    role = excluded.role,
                    backend_harness = excluded.backend_harness,
                    budget_tokens_slice = excluded.budget_tokens_slice,
                    workspace_lease_id = excluded.workspace_lease_id",
                params![
                    revision.revision_id,
                    placement.node_id,
                    placement.role,
                    placement.backend_harness,
                    placement.budget_tokens_slice as i64,
                    placement.workspace_lease_id,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn get_revision(&self, revision_id: &str) -> Result<Option<WorkflowRevision>, DomainError> {
        let reader = self.db.reader()?;
        let mut stmt = reader
            .prepare(
                "SELECT revision_id, task_id, proposal_id, revision_number,
                        nodes_json, dependencies_json, obligations_json, created_at
                 FROM workflow_revisions WHERE revision_id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let row = stmt
            .query_row(params![revision_id], Self::map_revision)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row)
    }

    pub fn list_revisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkflowRevision>, DomainError> {
        let reader = self.db.reader()?;
        let mut stmt = reader
            .prepare(
                "SELECT revision_id, task_id, proposal_id, revision_number,
                        nodes_json, dependencies_json, obligations_json, created_at
                 FROM workflow_revisions WHERE task_id = ?1 ORDER BY revision_number ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], Self::map_revision)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_placements_for_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<NodePlacement>, DomainError> {
        let reader = self.db.reader()?;
        let mut stmt = reader
            .prepare(
                "SELECT node_id, role, backend_harness, budget_tokens_slice, workspace_lease_id
                 FROM node_placements WHERE revision_id = ?1 ORDER BY node_id ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![revision_id], |row| {
                Ok(NodePlacement {
                    node_id: row.get(0)?,
                    role: row.get(1)?,
                    backend_harness: row.get(2)?,
                    budget_tokens_slice: row.get::<_, i64>(3)? as u64,
                    workspace_lease_id: row.get(4)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    fn map_revision(row: &rusqlite::Row) -> rusqlite::Result<WorkflowRevision> {
        let nodes_str: String = row.get(4)?;
        let deps_str: String = row.get(5)?;
        let obligations_str: String = row.get(6)?;
        let created_at_str: String = row.get(7)?;

        let nodes: Vec<RevisionNode> = serde_json::from_str(&nodes_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let dependencies: Vec<(String, String)> = serde_json::from_str(&deps_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let obligations: Vec<String> = serde_json::from_str(&obligations_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(WorkflowRevision {
            revision_id: row.get(0)?,
            task_id: row.get(1)?,
            proposal_id: row.get(2)?,
            revision_number: row.get(3)?,
            nodes,
            dependencies,
            obligations,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_get_workflow_revision() {
        let db = DbConnection::open_in_memory().unwrap();
        // Insert task for foreign key
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('task_rev_1', 'Revision Task', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let repo = WorkflowRevisionRepository::new(db);

        let mut revision = WorkflowRevision::new("task_rev_1", "prop_01", 1);
        revision.nodes.push(RevisionNode {
            node_id: "node_1".into(),
            step_name: "Investigate Code".into(),
            role: "reader".into(),
            harness_id: "claude_code".into(),
            allocated_budget_tokens: 5000,
            read_set: vec!["src/main.rs".into()],
            write_set: vec![],
            required_capabilities: vec!["fs_read".into()],
        });
        revision
            .dependencies
            .push(("start".into(), "node_1".into()));
        revision.obligations.push("ensure_evidence_logged".into());

        let placements = vec![NodePlacement {
            node_id: "node_1".into(),
            role: "reader".into(),
            backend_harness: "claude_code".into(),
            budget_tokens_slice: 5000,
            workspace_lease_id: Some("lease_01".into()),
        }];

        repo.save_revision(&revision, &placements).unwrap();

        let fetched = repo.get_revision(&revision.revision_id).unwrap().unwrap();
        assert_eq!(fetched.revision_id, revision.revision_id);
        assert_eq!(fetched.task_id, "task_rev_1");
        assert_eq!(fetched.nodes.len(), 1);
        assert_eq!(fetched.nodes[0].step_name, "Investigate Code");
        assert_eq!(fetched.dependencies.len(), 1);
        assert_eq!(fetched.obligations[0], "ensure_evidence_logged");

        let fetched_placements = repo
            .get_placements_for_revision(&revision.revision_id)
            .unwrap();
        assert_eq!(fetched_placements.len(), 1);
        assert_eq!(fetched_placements[0].node_id, "node_1");
        assert_eq!(
            fetched_placements[0].workspace_lease_id,
            Some("lease_01".into())
        );

        // Test list
        let revisions = repo.list_revisions_for_task("task_rev_1").unwrap();
        assert_eq!(revisions.len(), 1);

        // Test idempotency: re-save same revision with updated placements
        let updated_placements = vec![NodePlacement {
            node_id: "node_1".into(),
            role: "reader".into(),
            backend_harness: "claude_code".into(),
            budget_tokens_slice: 8000,
            workspace_lease_id: Some("lease_02".into()),
        }];
        repo.save_revision(&revision, &updated_placements).unwrap();

        let re_fetched = repo
            .get_placements_for_revision(&revision.revision_id)
            .unwrap();
        assert_eq!(re_fetched[0].budget_tokens_slice, 8000);
        assert_eq!(re_fetched[0].workspace_lease_id, Some("lease_02".into()));
    }
}
