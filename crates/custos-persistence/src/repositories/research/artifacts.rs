use super::ResearchRepository;
use custos_domain::{
    ArtifactLineageGraph, ArtifactLineageNode, ArtifactSummary, DomainError, LineageGraphEdge,
    LineageGraphNode,
};
use rusqlite::params;

impl ResearchRepository {
    // Artifact Lineage
    pub fn record_artifact_lineage(&self, node: &ArtifactLineageNode) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let id = format!("{}_{}", node.artifact_path, node.version);
        conn.execute(
            "INSERT INTO research_artifact_lineage (
                id, artifact_path, version, content_hash, produced_by_run_id, parent_version_hash, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                content_hash = excluded.content_hash,
                produced_by_run_id = excluded.produced_by_run_id",
            params![
                id,
                node.artifact_path,
                node.version as i64,
                node.content_hash,
                node.produced_by_run_id,
                node.parent_version_hash,
                node.timestamp,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_artifact_lineage(
        &self,
        artifact_path: &str,
    ) -> Result<Vec<ArtifactLineageNode>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT artifact_path, version, content_hash, produced_by_run_id, parent_version_hash, timestamp
                 FROM research_artifact_lineage WHERE artifact_path = ?1 ORDER BY version ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![artifact_path], |row| {
                let v: i64 = row.get(1)?;
                Ok(ArtifactLineageNode {
                    artifact_path: row.get(0)?,
                    version: v as u32,
                    content_hash: row.get(2)?,
                    produced_by_run_id: row.get(3)?,
                    parent_version_hash: row.get(4)?,
                    timestamp: row.get(5)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn list_all_artifacts(&self) -> Result<Vec<ArtifactSummary>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT artifact_path, MAX(version) as max_v, count(*) as cnt
                 FROM research_artifact_lineage
                 GROUP BY artifact_path
                 ORDER BY MAX(timestamp) DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let max_v: i64 = row.get(1)?;
                let count: i64 = row.get(2)?;
                Ok((path, max_v as u32, count as u32))
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for item in rows {
            let (path, max_v, count) = item.map_err(|e| DomainError::Validation(e.to_string()))?;
            let mut detail_stmt = conn
                .prepare(
                    "SELECT content_hash, produced_by_run_id, timestamp
                     FROM research_artifact_lineage
                     WHERE artifact_path = ?1 AND version = ?2",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            let detail = detail_stmt
                .query_row(params![path, max_v as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            res.push(ArtifactSummary {
                artifact_path: path,
                latest_version: max_v,
                latest_content_hash: detail.0,
                versions_count: count,
                produced_by_run_id: detail.1,
                updated_at: detail.2,
            });
        }
        Ok(res)
    }

    pub fn get_artifact_lineage_graph(
        &self,
        filter_path: Option<&str>,
    ) -> Result<ArtifactLineageGraph, DomainError> {
        let conn = self.db.lock()?;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let query = if let Some(path) = filter_path {
            format!(
                "SELECT artifact_path, version, content_hash, produced_by_run_id, parent_version_hash, timestamp
                 FROM research_artifact_lineage WHERE artifact_path = '{}' ORDER BY version ASC",
                path.replace('\'', "''")
            )
        } else {
            "SELECT artifact_path, version, content_hash, produced_by_run_id, parent_version_hash, timestamp
             FROM research_artifact_lineage ORDER BY artifact_path ASC, version ASC".to_string()
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let lineage_rows = stmt
            .query_map([], |row| {
                let v: i64 = row.get(1)?;
                Ok(ArtifactLineageNode {
                    artifact_path: row.get(0)?,
                    version: v as u32,
                    content_hash: row.get(2)?,
                    produced_by_run_id: row.get(3)?,
                    parent_version_hash: row.get(4)?,
                    timestamp: row.get(5)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut run_ids_to_fetch = std::collections::HashSet::new();

        for r in lineage_rows {
            let node = r.map_err(|e| DomainError::Validation(e.to_string()))?;
            let node_id = format!("artifact:{}:v{}", node.artifact_path, node.version);
            let label = format!("{} (v{})", node.artifact_path, node.version);

            nodes.push(LineageGraphNode {
                id: node_id.clone(),
                label,
                node_type: if node.artifact_path.starts_with("notes/") {
                    "note".to_string()
                } else {
                    "artifact".to_string()
                },
                version: Some(node.version),
                content_hash: Some(node.content_hash.clone()),
                timestamp: node.timestamp,
                metadata: serde_json::json!({
                    "path": node.artifact_path,
                    "version": node.version,
                    "content_hash": node.content_hash,
                    "produced_by_run_id": node.produced_by_run_id,
                }),
            });

            if node.version > 1 {
                let parent_id = format!("artifact:{}:v{}", node.artifact_path, node.version - 1);
                edges.push(LineageGraphEdge {
                    from: parent_id,
                    to: node_id.clone(),
                    edge_type: "derived_from_version".to_string(),
                });
            }

            if let Some(ref run_id) = node.produced_by_run_id {
                run_ids_to_fetch.insert(run_id.clone());
                let run_node_id = format!("run:{}", run_id);
                edges.push(LineageGraphEdge {
                    from: run_node_id,
                    to: node_id,
                    edge_type: "produced_by_run".to_string(),
                });
            }
        }

        for run_id in run_ids_to_fetch {
            let mut run_stmt = conn
                .prepare(
                    "SELECT run_id, command, status, ts, reproducibility
                     FROM research_experiment_runs WHERE run_id = ?1",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            if let Ok(run_info) = run_stmt.query_row(params![run_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            }) {
                nodes.push(LineageGraphNode {
                    id: format!("run:{}", run_info.0),
                    label: format!("Run: {}", run_info.1),
                    node_type: "run".to_string(),
                    version: None,
                    content_hash: None,
                    timestamp: run_info.3,
                    metadata: serde_json::json!({
                        "run_id": run_info.0,
                        "command": run_info.1,
                        "status": run_info.2,
                        "reproducibility": run_info.4,
                    }),
                });
            }
        }

        Ok(ArtifactLineageGraph { nodes, edges })
    }
}
