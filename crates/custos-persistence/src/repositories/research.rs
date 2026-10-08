//! Research Persistence Repository
//!
//! SQLite-backed repository managing scientific Sources, Cryptographic Passage Anchors,
//! Claims Matrix (L0-L3), Experiment Runs, and Artifact Lineage.

use crate::connection::DbConnection;
use chrono::{DateTime, Utc};
use custos_domain::{
    AnnotationRecord, AnnotationStatus, AnnotationTarget, ArtifactLineageGraph,
    ArtifactLineageNode, ArtifactSummary, CellExecutionStatus, ClaimEvidenceLink,
    ClaimGroundingLevel, DomainError, EnvSnapshot, EnvironmentSpec, EvidenceRelation,
    ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus, KernelStatus,
    LineageGraphEdge, LineageGraphNode, NoteRecord, NoteVersionRecord, NotebookCell,
    NotebookCellType, NotebookKernelState, PassageAnchor, Recipe, RecipeInput, ResearchClaim,
    ResearchExperimentRun, ReviewFinding, ReviewMethod, ReviewStatus, ReviewTargetType,
    ReviewerRecord, SourceRecord,
};
use rusqlite::{params, types::Type};

#[derive(Clone)]
pub struct ResearchRepository {
    db: DbConnection,
}

impl ResearchRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    // Sources
    pub fn save_source(&self, src: &SourceRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let authors_json = serde_json::to_string(&src.authors)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO research_sources (
                id, source_type, title, doi, authors_json, year, content_hash, local_path, verified, abstract_text, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                doi = excluded.doi,
                authors_json = excluded.authors_json,
                year = excluded.year,
                content_hash = excluded.content_hash,
                local_path = excluded.local_path,
                verified = excluded.verified,
                abstract_text = excluded.abstract_text",
            params![
                src.id,
                src.source_type,
                src.title,
                src.doi,
                authors_json,
                src.year,
                src.content_hash,
                src.local_path,
                if src.verified { 1 } else { 0 },
                src.abstract_text,
                src.created_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_sources(&self) -> Result<Vec<SourceRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source_type, title, doi, authors_json, year, content_hash, local_path, verified, abstract_text, created_at
                 FROM research_sources ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let authors_json: String = row.get(4)?;
                let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
                let verified_int: i32 = row.get(8)?;

                Ok(SourceRecord {
                    id: row.get(0)?,
                    source_type: row.get(1)?,
                    title: row.get(2)?,
                    doi: row.get(3)?,
                    authors,
                    year: row.get(5)?,
                    content_hash: row.get(6)?,
                    local_path: row.get(7)?,
                    verified: verified_int == 1,
                    abstract_text: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Passage Anchors
    pub fn save_anchor(&self, anchor: &PassageAnchor) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO research_passage_anchors (
                id, source_id, source_title, section_title, page_number, start_offset, end_offset, exact_text, passage_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                exact_text = excluded.exact_text,
                passage_hash = excluded.passage_hash",
            params![
                anchor.id,
                anchor.source_id,
                anchor.source_title,
                anchor.section_title,
                anchor.page_number,
                anchor.start_offset as i64,
                anchor.end_offset as i64,
                anchor.exact_text,
                anchor.passage_hash,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_anchors_for_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<PassageAnchor>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, section_title, page_number, start_offset, end_offset, exact_text, passage_hash
                 FROM research_passage_anchors WHERE source_id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![source_id], |row| {
                let start: i64 = row.get(5)?;
                let end: i64 = row.get(6)?;
                Ok(PassageAnchor {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    source_title: row.get(2)?,
                    section_title: row.get(3)?,
                    page_number: row.get(4)?,
                    start_offset: start as usize,
                    end_offset: end as usize,
                    exact_text: row.get(7)?,
                    passage_hash: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Claims Matrix
    pub fn save_claim(&self, claim: &ResearchClaim) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let invariants_json = serde_json::to_string(&claim.invariants)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let level_str = match claim.level {
            ClaimGroundingLevel::L0Ungrounded => "l0_ungrounded",
            ClaimGroundingLevel::L1Cited => "l1_cited",
            ClaimGroundingLevel::L2Verified => "l2_verified",
            ClaimGroundingLevel::L3Sealed => "l3_sealed",
        };

        conn.execute(
            "INSERT INTO research_claims (
                id, statement, level, confidence_score, invariants_json, sealed_proof_uri, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                statement = excluded.statement,
                level = excluded.level,
                confidence_score = excluded.confidence_score,
                invariants_json = excluded.invariants_json,
                sealed_proof_uri = excluded.sealed_proof_uri",
            params![
                claim.id,
                claim.statement,
                level_str,
                claim.confidence_score as f64,
                invariants_json,
                claim.sealed_proof_uri,
                claim.created_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        // Save evidence links
        for link in &claim.evidence_links {
            let rel_str = match link.relation {
                EvidenceRelation::Supports => "supports",
                EvidenceRelation::Refutes => "refutes",
                EvidenceRelation::Qualifies => "qualifies",
            };
            let link_id = format!("{}_{}", claim.id, link.passage_anchor_id);
            conn.execute(
                "INSERT INTO research_claim_evidence_links (
                    id, claim_id, passage_anchor_id, relation, rationale, verified_by
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(id) DO UPDATE SET
                    relation = excluded.relation,
                    rationale = excluded.rationale,
                    verified_by = excluded.verified_by",
                params![
                    link_id,
                    claim.id,
                    link.passage_anchor_id,
                    rel_str,
                    link.rationale,
                    link.verified_by,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }

        Ok(())
    }

    pub fn list_claims(&self) -> Result<Vec<ResearchClaim>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, statement, level, confidence_score, invariants_json, sealed_proof_uri, created_at
                 FROM research_claims ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let claim_rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let statement: String = row.get(1)?;
                let level_str: String = row.get(2)?;
                let conf: f64 = row.get(3)?;
                let inv_json: String = row.get(4)?;
                let sealed: Option<String> = row.get(5)?;
                let created_at: i64 = row.get(6)?;

                let level = match level_str.as_str() {
                    "l1_cited" => ClaimGroundingLevel::L1Cited,
                    "l2_verified" => ClaimGroundingLevel::L2Verified,
                    "l3_sealed" => ClaimGroundingLevel::L3Sealed,
                    _ => ClaimGroundingLevel::L0Ungrounded,
                };
                let invariants: Vec<String> = serde_json::from_str(&inv_json).unwrap_or_default();

                Ok((
                    id,
                    statement,
                    level,
                    conf as f32,
                    invariants,
                    sealed,
                    created_at,
                ))
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut claims = Vec::new();
        for cr in claim_rows {
            let (id, statement, level, confidence_score, invariants, sealed_proof_uri, created_at) =
                cr.map_err(|e| DomainError::Validation(e.to_string()))?;

            // Fetch evidence links for this claim
            let mut link_stmt = conn
                .prepare(
                    "SELECT l.passage_anchor_id, a.source_title, a.exact_text, l.relation, l.rationale, l.verified_by
                     FROM research_claim_evidence_links l
                     LEFT JOIN research_passage_anchors a ON l.passage_anchor_id = a.id
                     WHERE l.claim_id = ?1",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let links = link_stmt
                .query_map(params![id], |row| {
                    let rel_str: String = row.get(3)?;
                    let relation = match rel_str.as_str() {
                        "refutes" => EvidenceRelation::Refutes,
                        "qualifies" => EvidenceRelation::Qualifies,
                        _ => EvidenceRelation::Supports,
                    };
                    Ok(ClaimEvidenceLink {
                        passage_anchor_id: row.get(0)?,
                        source_title: row.get(1)?,
                        exact_text: row.get(2)?,
                        relation,
                        rationale: row.get(4)?,
                        verified_by: row.get(5)?,
                    })
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            claims.push(ResearchClaim {
                id,
                statement,
                level,
                confidence_score,
                invariants,
                evidence_links: links,
                created_at,
                sealed_proof_uri,
            });
        }

        Ok(claims)
    }

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

    // Notes
    pub fn save_note(&self, note: &NoteRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let tags_json = serde_json::to_string(&note.tags)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO notes (
                id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                version = excluded.version,
                content_hash = excluded.content_hash,
                tags_json = excluded.tags_json,
                updated_at = excluded.updated_at",
            params![
                note.id,
                note.title,
                note.content,
                note.version as i64,
                note.content_hash,
                note.session_id,
                note.task_id,
                tags_json,
                note.created_at,
                note.updated_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn save_note_version(&self, ver: &NoteVersionRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO note_versions (
                id, note_id, version, content, content_hash, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO NOTHING",
            params![
                ver.id,
                ver.note_id,
                ver.version as i64,
                ver.content,
                ver.content_hash,
                ver.created_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_note(&self, id: &str) -> Result<Option<NoteRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
                 FROM notes WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let v: i64 = row.get(3)?;
                let tags_raw: String = row.get(7)?;
                let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                Ok(NoteRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    version: v as u32,
                    content_hash: row.get(4)?,
                    session_id: row.get(5)?,
                    task_id: row.get(6)?,
                    tags,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?)),
            None => Ok(None),
        }
    }

    pub fn list_notes(&self, session_id: Option<&str>) -> Result<Vec<NoteRecord>, DomainError> {
        let conn = self.db.lock()?;
        let query = if session_id.is_some() {
            "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
             FROM notes WHERE session_id = ?1 ORDER BY updated_at DESC"
        } else {
            "SELECT id, title, content, version, content_hash, session_id, task_id, tags_json, created_at, updated_at
             FROM notes ORDER BY updated_at DESC"
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let param_vec: Vec<&dyn rusqlite::ToSql> = if let Some(ref sid) = session_id {
            vec![sid]
        } else {
            vec![]
        };

        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_vec), |row| {
                let v: i64 = row.get(3)?;
                let tags_raw: String = row.get(7)?;
                let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                Ok(NoteRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    version: v as u32,
                    content_hash: row.get(4)?,
                    session_id: row.get(5)?,
                    task_id: row.get(6)?,
                    tags,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn list_note_versions(&self, note_id: &str) -> Result<Vec<NoteVersionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, note_id, version, content, content_hash, created_at
                 FROM note_versions WHERE note_id = ?1 ORDER BY version DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![note_id], |row| {
                let v: i64 = row.get(2)?;
                Ok(NoteVersionRecord {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    version: v as u32,
                    content: row.get(3)?,
                    content_hash: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Notebook Cells & Kernel Sessions
    pub fn list_notebook_cells(&self, session_id: &str) -> Result<Vec<NotebookCell>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, cell_type, source, cell_index, execution_count, status, stdout, stderr, output_image, wall_ms, epoch, updated_at
                 FROM notebook_cells WHERE session_id = ?1 ORDER BY cell_index ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                let cell_type_str: String = row.get(2)?;
                let cell_type = match cell_type_str.as_str() {
                    "markdown" => NotebookCellType::Markdown,
                    _ => NotebookCellType::Code,
                };
                let status_str: String = row.get(6)?;
                let status = match status_str.as_str() {
                    "running" => CellExecutionStatus::Running,
                    "success" => CellExecutionStatus::Success,
                    "error" => CellExecutionStatus::Error,
                    _ => CellExecutionStatus::Idle,
                };
                let cell_idx: i64 = row.get(4)?;
                let exec_cnt: Option<i64> = row.get(5)?;
                let wall: Option<i64> = row.get(10)?;
                let ep: i64 = row.get(11)?;

                Ok(NotebookCell {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    cell_type,
                    source: row.get(3)?,
                    cell_index: cell_idx as u32,
                    execution_count: exec_cnt.map(|c| c as u32),
                    status,
                    stdout: row.get(7)?,
                    stderr: row.get(8)?,
                    output_image: row.get(9)?,
                    wall_ms: wall.map(|w| w as u64),
                    epoch: ep as u32,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let cell_type_str = match cell.cell_type {
            NotebookCellType::Code => "code",
            NotebookCellType::Markdown => "markdown",
        };
        let status_str = match cell.status {
            CellExecutionStatus::Idle => "idle",
            CellExecutionStatus::Running => "running",
            CellExecutionStatus::Success => "success",
            CellExecutionStatus::Error => "error",
        };

        conn.execute(
            "INSERT INTO notebook_cells (
                id, session_id, cell_type, source, cell_index, execution_count, status, stdout, stderr, output_image, wall_ms, epoch, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                cell_type = excluded.cell_type,
                source = excluded.source,
                cell_index = excluded.cell_index,
                execution_count = excluded.execution_count,
                status = excluded.status,
                stdout = excluded.stdout,
                stderr = excluded.stderr,
                output_image = excluded.output_image,
                wall_ms = excluded.wall_ms,
                epoch = excluded.epoch,
                updated_at = excluded.updated_at",
            params![
                cell.id,
                cell.session_id,
                cell_type_str,
                cell.source,
                cell.cell_index as i64,
                cell.execution_count.map(|c| c as i64),
                status_str,
                cell.stdout,
                cell.stderr,
                cell.output_image,
                cell.wall_ms.map(|w| w as i64),
                cell.epoch as i64,
                cell.updated_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn save_notebook_cells(&self, cells: &[NotebookCell]) -> Result<(), DomainError> {
        for cell in cells {
            self.save_notebook_cell(cell)?;
        }
        Ok(())
    }

    pub fn delete_notebook_cell(&self, cell_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM notebook_cells WHERE id = ?1", params![cell_id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn get_notebook_kernel_state(&self, session_id: &str) -> Result<NotebookKernelState, DomainError> {
        let existing = {
            let conn = self.db.lock()?;
            let mut stmt = conn
                .prepare(
                    "SELECT session_id, epoch, status, python_version, execution_counter, created_at, updated_at
                     FROM notebook_kernel_sessions WHERE session_id = ?1",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let mut rows = stmt
                .query_map(params![session_id], |row| {
                    let status_str: String = row.get(2)?;
                    let status = match status_str.as_str() {
                        "busy" => KernelStatus::Busy,
                        "interrupted" => KernelStatus::Interrupted,
                        _ => KernelStatus::Idle,
                    };
                    let ep: i64 = row.get(1)?;
                    let counter: i64 = row.get(4)?;
                    Ok(NotebookKernelState {
                        session_id: row.get(0)?,
                        epoch: ep as u32,
                        status,
                        python_version: row.get(3)?,
                        execution_counter: counter as u32,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            if let Some(r) = rows.next() {
                Some(r.map_err(|e| DomainError::Validation(e.to_string()))?)
            } else {
                None
            }
        };

        if let Some(state) = existing {
            Ok(state)
        } else {
            let now = chrono::Utc::now().timestamp();
            let default_state = NotebookKernelState {
                session_id: session_id.to_string(),
                epoch: 1,
                status: KernelStatus::Idle,
                python_version: "Python 3.14".to_string(),
                execution_counter: 0,
                created_at: now,
                updated_at: now,
            };
            self.save_notebook_kernel_state(&default_state)?;
            Ok(default_state)
        }
    }

    pub fn save_notebook_kernel_state(&self, state: &NotebookKernelState) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let status_str = match state.status {
            KernelStatus::Idle => "idle",
            KernelStatus::Busy => "busy",
            KernelStatus::Interrupted => "interrupted",
        };
        conn.execute(
            "INSERT INTO notebook_kernel_sessions (
                session_id, epoch, status, python_version, execution_counter, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(session_id) DO UPDATE SET
                epoch = excluded.epoch,
                status = excluded.status,
                python_version = excluded.python_version,
                execution_counter = excluded.execution_counter,
                updated_at = excluded.updated_at",
            params![
                state.session_id,
                state.epoch as i64,
                status_str,
                state.python_version,
                state.execution_counter as i64,
                state.created_at,
                state.updated_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    // ==========================================
    // Reviewer Records & Evidence Criteria (Step 8)
    // ==========================================

    pub fn save_review(&self, review: &ReviewerRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let target_type_str = match review.target_type {
            ReviewTargetType::Task => "task",
            ReviewTargetType::Artifact => "artifact",
            ReviewTargetType::Diff => "diff",
            ReviewTargetType::Run => "run",
            ReviewTargetType::Note => "note",
            ReviewTargetType::Workspace => "workspace",
        };
        let method_str = match review.method {
            ReviewMethod::AutomatedVerifier => "automated_verifier",
            ReviewMethod::PeerReview => "peer_review",
            ReviewMethod::ModelEvaluation => "model_evaluation",
            ReviewMethod::ContractProof => "contract_proof",
            ReviewMethod::RuntimeInspection => "runtime_inspection",
        };
        let status_str = match review.status {
            ReviewStatus::Approved => "approved",
            ReviewStatus::Rejected => "rejected",
            ReviewStatus::Degraded => "degraded",
            ReviewStatus::Pending => "pending",
        };
        let findings_json = serde_json::to_string(&review.findings)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let is_fresh_int: i64 = if review.is_fresh { 1 } else { 0 };

        conn.execute(
            "INSERT INTO reviewer_records (
                id, target_type, target_id, reviewer, method, status,
                evidence_summary, evidence_digest, findings_json, is_fresh,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                target_type = excluded.target_type,
                target_id = excluded.target_id,
                reviewer = excluded.reviewer,
                method = excluded.method,
                status = excluded.status,
                evidence_summary = excluded.evidence_summary,
                evidence_digest = excluded.evidence_digest,
                findings_json = excluded.findings_json,
                is_fresh = excluded.is_fresh,
                updated_at = excluded.updated_at",
            params![
                review.id,
                target_type_str,
                review.target_id,
                review.reviewer,
                method_str,
                status_str,
                review.evidence_summary,
                review.evidence_digest,
                findings_json,
                is_fresh_int,
                review.created_at,
                review.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_review(&self, id: &str) -> Result<Option<ReviewerRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, target_type, target_id, reviewer, method, status,
                        evidence_summary, evidence_digest, findings_json, is_fresh,
                        created_at, updated_at
                 FROM reviewer_records WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| Self::row_to_review(row))
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn list_reviews(
        &self,
        target_type: Option<ReviewTargetType>,
        target_id: Option<&str>,
    ) -> Result<Vec<ReviewerRecord>, DomainError> {
        let conn = self.db.lock()?;
        let target_type_str = target_type.map(|t| match t {
            ReviewTargetType::Task => "task",
            ReviewTargetType::Artifact => "artifact",
            ReviewTargetType::Diff => "diff",
            ReviewTargetType::Run => "run",
            ReviewTargetType::Note => "note",
            ReviewTargetType::Workspace => "workspace",
        });

        let mut sql = "SELECT id, target_type, target_id, reviewer, method, status,
                              evidence_summary, evidence_digest, findings_json, is_fresh,
                              created_at, updated_at
                       FROM reviewer_records WHERE 1=1".to_string();

        let mut params_vec: Vec<String> = Vec::new();
        if let Some(tt) = target_type_str {
            sql.push_str(" AND target_type = ?");
            params_vec.push(tt.to_string());
        }
        if let Some(ti) = target_id {
            sql.push_str(" AND target_id = ?");
            params_vec.push(ti.to_string());
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mapper = Self::row_to_review as fn(&rusqlite::Row) -> rusqlite::Result<ReviewerRecord>;
        let rows = if params_vec.is_empty() {
            stmt.query_map([], mapper)
        } else if params_vec.len() == 1 {
            stmt.query_map(params![params_vec[0]], mapper)
        } else {
            stmt.query_map(params![params_vec[0], params_vec[1]], mapper)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn mark_review_stale(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE reviewer_records SET is_fresh = 0, updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    fn row_to_review(row: &rusqlite::Row) -> rusqlite::Result<ReviewerRecord> {
        let target_type_str: String = row.get(1)?;
        let target_type = match target_type_str.as_str() {
            "task" => ReviewTargetType::Task,
            "artifact" => ReviewTargetType::Artifact,
            "diff" => ReviewTargetType::Diff,
            "run" => ReviewTargetType::Run,
            "note" => ReviewTargetType::Note,
            _ => ReviewTargetType::Workspace,
        };

        let method_str: String = row.get(4)?;
        let method = match method_str.as_str() {
            "automated_verifier" => ReviewMethod::AutomatedVerifier,
            "peer_review" => ReviewMethod::PeerReview,
            "model_evaluation" => ReviewMethod::ModelEvaluation,
            "contract_proof" => ReviewMethod::ContractProof,
            _ => ReviewMethod::RuntimeInspection,
        };

        let status_str: String = row.get(5)?;
        let status = match status_str.as_str() {
            "approved" => ReviewStatus::Approved,
            "rejected" => ReviewStatus::Rejected,
            "degraded" => ReviewStatus::Degraded,
            _ => ReviewStatus::Pending,
        };

        let findings_json: String = row.get(8)?;
        let findings: Vec<ReviewFinding> = serde_json::from_str(&findings_json).unwrap_or_default();
        let is_fresh_int: i64 = row.get(9)?;

        Ok(ReviewerRecord {
            id: row.get(0)?,
            target_type,
            target_id: row.get(2)?,
            reviewer: row.get(3)?,
            method,
            status,
            evidence_summary: row.get(6)?,
            evidence_digest: row.get(7)?,
            findings,
            is_fresh: is_fresh_int == 1,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
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
        let conn = self.db.lock()?;
        let status_str = match record.status {
            ExecutionRecordStatus::Pending => "pending",
            ExecutionRecordStatus::Running => "running",
            ExecutionRecordStatus::Completed => "completed",
            ExecutionRecordStatus::Failed => "failed",
            ExecutionRecordStatus::Cancelled => "cancelled",
        };
        let artifacts_json = serde_json::to_string(&record.artifacts)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO research_execution_records (
                id, recipe_id, session_id, status, exit_code, stdout_cas_uri, stderr_cas_uri,
                started_at, ended_at, artifacts_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                exit_code = excluded.exit_code,
                stdout_cas_uri = excluded.stdout_cas_uri,
                stderr_cas_uri = excluded.stderr_cas_uri,
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                artifacts_json = excluded.artifacts_json",
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
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_execution_record(&self, id: &str) -> Result<Option<ExecutionRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, recipe_id, session_id, status, exit_code, stdout_cas_uri, stderr_cas_uri,
                        started_at, ended_at, artifacts_json
                 FROM research_execution_records WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let status_str: String = row.get(3)?;
                let status = match status_str.as_str() {
                    "running" => ExecutionRecordStatus::Running,
                    "completed" => ExecutionRecordStatus::Completed,
                    "failed" => ExecutionRecordStatus::Failed,
                    "cancelled" => ExecutionRecordStatus::Cancelled,
                    _ => ExecutionRecordStatus::Pending,
                };
                let started_ts: i64 = row.get(7)?;
                let ended_ts: Option<i64> = row.get(8)?;
                let artifacts_json: String = row.get(9)?;
                let artifacts: Vec<ExecutionArtifact> =
                    serde_json::from_str(&artifacts_json).unwrap_or_default();

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
                        started_at, ended_at, artifacts_json
                 FROM research_execution_records WHERE recipe_id = ?1 ORDER BY started_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![recipe_id], |row| {
                let status_str: String = row.get(3)?;
                let status = match status_str.as_str() {
                    "running" => ExecutionRecordStatus::Running,
                    "completed" => ExecutionRecordStatus::Completed,
                    "failed" => ExecutionRecordStatus::Failed,
                    "cancelled" => ExecutionRecordStatus::Cancelled,
                    _ => ExecutionRecordStatus::Pending,
                };
                let started_ts: i64 = row.get(7)?;
                let ended_ts: Option<i64> = row.get(8)?;
                let artifacts_json: String = row.get(9)?;
                let artifacts: Vec<ExecutionArtifact> =
                    serde_json::from_str(&artifacts_json).unwrap_or_default();

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
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Annotations
    pub fn save_annotation(&self, ann: &AnnotationRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let target_json = serde_json::to_string(&ann.target)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let status_str = match ann.status {
            AnnotationStatus::Pending => "pending",
            AnnotationStatus::Submitted => "submitted",
            AnnotationStatus::Addressed => "addressed",
            AnnotationStatus::Dismissed => "dismissed",
        };

        conn.execute(
            "INSERT INTO research_annotations (
                id, artifact_path, artifact_version, target_json, note, actor,
                status, side_chat_session_id, submitted_turn_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                target_json = excluded.target_json,
                note = excluded.note,
                status = excluded.status,
                side_chat_session_id = excluded.side_chat_session_id,
                submitted_turn_id = excluded.submitted_turn_id",
            params![
                ann.id,
                ann.artifact_path,
                ann.artifact_version,
                target_json,
                ann.note,
                ann.actor,
                status_str,
                ann.side_chat_session_id,
                ann.submitted_turn_id,
                ann.created_at.timestamp(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_annotations_for_artifact(
        &self,
        path: &str,
        version: Option<u32>,
    ) -> Result<Vec<AnnotationRecord>, DomainError> {
        let conn = self.db.lock()?;
        let (query, has_version) = if version.is_some() {
            (
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE artifact_path = ?1 AND artifact_version = ?2 ORDER BY created_at ASC",
                true,
            )
        } else {
            (
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE artifact_path = ?1 ORDER BY created_at ASC",
                false,
            )
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = if has_version {
            stmt.query_map(params![path, version.unwrap()], Self::map_annotation_row)
        } else {
            stmt.query_map(params![path], Self::map_annotation_row)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn list_pending_annotations(&self) -> Result<Vec<AnnotationRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, artifact_path, artifact_version, target_json, note, actor,
                        status, side_chat_session_id, submitted_turn_id, created_at
                 FROM research_annotations WHERE status = 'pending' ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_annotation_row)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn update_annotation_status(
        &self,
        id: &str,
        status: AnnotationStatus,
        turn_id: Option<&str>,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let status_str = match status {
            AnnotationStatus::Pending => "pending",
            AnnotationStatus::Submitted => "submitted",
            AnnotationStatus::Addressed => "addressed",
            AnnotationStatus::Dismissed => "dismissed",
        };

        conn.execute(
            "UPDATE research_annotations SET status = ?1, submitted_turn_id = COALESCE(?2, submitted_turn_id) WHERE id = ?3",
            params![status_str, turn_id, id],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    fn map_annotation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnnotationRecord> {
        let target_json: String = row.get(3)?;
        let target: AnnotationTarget =
            serde_json::from_str(&target_json).unwrap_or(AnnotationTarget::Text {
                start_offset: 0,
                end_offset: 0,
                exact_text: String::new(),
            });
        let status_str: String = row.get(6)?;
        let status = match status_str.as_str() {
            "submitted" => AnnotationStatus::Submitted,
            "addressed" => AnnotationStatus::Addressed,
            "dismissed" => AnnotationStatus::Dismissed,
            _ => AnnotationStatus::Pending,
        };
        let ts: i64 = row.get(9)?;

        Ok(AnnotationRecord {
            id: row.get(0)?,
            artifact_path: row.get(1)?,
            artifact_version: row.get(2)?,
            target,
            note: row.get(4)?,
            actor: row.get(5)?,
            status,
            side_chat_session_id: row.get(7)?,
            submitted_turn_id: row.get(8)?,
            created_at: DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DbConnection;

    #[test]
    fn test_research_repository_full_lifecycle() {
        let db = DbConnection::open_in_memory().expect("open db");
        let repo = ResearchRepository::new(db);

        // 1. Source
        let src = SourceRecord {
            id: "src_123".to_string(),
            source_type: "paper".to_string(),
            title: "Attention Is All You Need".to_string(),
            doi: Some("10.1145/test".to_string()),
            authors: vec!["Vaswani et al.".to_string()],
            year: Some(2017),
            content_hash: "hash_vaswani".to_string(),
            local_path: Some("/papers/transformer.pdf".to_string()),
            verified: true,
            abstract_text: Some("We propose Transformer...".to_string()),
            created_at: 1700000000,
        };
        repo.save_source(&src).expect("save source");
        let sources = repo.list_sources().expect("list sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Attention Is All You Need");
        assert!(sources[0].verified);

        // 2. Anchor
        let anchor = PassageAnchor {
            id: "anc_01".to_string(),
            source_id: "src_123".to_string(),
            source_title: Some("Attention Is All You Need".to_string()),
            section_title: Some("3.2.2 Multi-Head Attention".to_string()),
            page_number: Some(3),
            start_offset: 120,
            end_offset: 250,
            exact_text: "Multi-Head Attention consists of several attention layers".to_string(),
            passage_hash: "hash_anchor_01".to_string(),
        };
        repo.save_anchor(&anchor).expect("save anchor");
        let anchors = repo
            .list_anchors_for_source("src_123")
            .expect("list anchors");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].exact_text, anchor.exact_text);

        // 3. Claim
        let claim = ResearchClaim {
            id: "claim_01".to_string(),
            statement: "Multi-Head Attention runs 8 parallel heads in base model".to_string(),
            level: ClaimGroundingLevel::L1Cited,
            confidence_score: 0.95,
            invariants: vec!["head_count == 8".to_string()],
            evidence_links: vec![ClaimEvidenceLink {
                passage_anchor_id: "anc_01".to_string(),
                source_title: Some("Attention Is All You Need".to_string()),
                exact_text: Some(
                    "Multi-Head Attention consists of several attention layers".to_string(),
                ),
                relation: EvidenceRelation::Supports,
                rationale: "Confirmed in section 3.2.2".to_string(),
                verified_by: "expert_review".to_string(),
            }],
            created_at: 1700000200,
            sealed_proof_uri: None,
        };
        repo.save_claim(&claim).expect("save claim");
        let claims = repo.list_claims().expect("list claims");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].statement, claim.statement);
        assert_eq!(claims[0].level, ClaimGroundingLevel::L1Cited);
        assert_eq!(claims[0].evidence_links.len(), 1);

        // 4. Run
        let run = ResearchExperimentRun {
            run_id: "run_exp_99".to_string(),
            session_id: "sess_01".to_string(),
            command: "python train_transformer.py".to_string(),
            cwd: "/workspace".to_string(),
            status: "ok".to_string(),
            wall_ms: 45000,
            surface: Some("sade_container".to_string()),
            reproducibility: "deterministic".to_string(),
            input_merkle_root: "merkle_in_root".to_string(),
            output_merkle_root: "merkle_out_root".to_string(),
            env_snapshot: EnvSnapshot {
                python_version: "3.11.4".to_string(),
                lockfile_hash: "lock_abc".to_string(),
                package_count: Some(42),
                hardware: "Apple M3 Max".to_string(),
                platform: Some("macos-arm64".to_string()),
            },
            sade_permit_id: Some("permit_01".to_string()),
            cas_log_uri: Some("cas://bafy_log".to_string()),
            ts: 1700000300,
        };
        repo.save_run(&run).expect("save run");
        let runs = repo.list_runs().expect("list runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "run_exp_99");
        assert_eq!(runs[0].reproducibility, "deterministic");

        // 5. Lineage
        let lineage = ArtifactLineageNode {
            artifact_path: "eval_table.csv".to_string(),
            version: 1,
            content_hash: "hash_csv".to_string(),
            produced_by_run_id: Some("run_exp_99".to_string()),
            parent_version_hash: None,
            timestamp: 1700000400,
        };
        repo.record_artifact_lineage(&lineage)
            .expect("record lineage");
        let lineage_list = repo
            .list_artifact_lineage("eval_table.csv")
            .expect("list lineage");
        assert_eq!(lineage_list.len(), 1);
        assert_eq!(lineage_list[0].version, 1);
        assert_eq!(lineage_list[0].content_hash, "hash_csv");

        // 6. Recipe
        let mut env_spec = EnvironmentSpec::new();
        env_spec.python_version = Some("3.11.4".to_string());
        env_spec.requirements = vec!["torch>=2.0.0".to_string(), "pandas".to_string()];
        let recipe = Recipe::new(
            "Train Ablation Model",
            "python train.py --lr 1e-4",
            env_spec,
        )
        .with_description("Ablation study on learning rate")
        .with_input(RecipeInput::new("train_data", "cas://bafy_data", true))
        .with_output("loss_curve.png");
        let recipe_id = recipe.id.clone();
        repo.save_recipe(&recipe).expect("save recipe");

        let fetched_recipe = repo
            .get_recipe(&recipe_id)
            .expect("get recipe")
            .expect("recipe exists");
        assert_eq!(fetched_recipe.name, "Train Ablation Model");
        assert_eq!(fetched_recipe.inputs.len(), 1);
        assert_eq!(fetched_recipe.outputs, vec!["loss_curve.png"]);

        let recipes = repo.list_recipes().expect("list recipes");
        assert_eq!(recipes.len(), 1);

        // 7. Execution Record
        let mut exec_record = ExecutionRecord::new(&recipe_id).with_session_id("session_res_01");
        exec_record.stdout_cas_uri = Some("cas://bafy_exec_stdout".to_string());
        exec_record.artifacts.push(ExecutionArtifact::new(
            "loss_curve.png",
            "hash_loss_curve",
            10240,
        ));
        let exec_id = exec_record.id.clone();
        repo.save_execution_record(&exec_record)
            .expect("save exec record");

        let fetched_exec = repo
            .get_execution_record(&exec_id)
            .expect("get exec")
            .expect("exec exists");
        assert_eq!(fetched_exec.status, ExecutionRecordStatus::Pending);
        assert_eq!(fetched_exec.artifacts.len(), 1);

        exec_record.transition(ExecutionRecordStatus::Completed);
        exec_record.exit_code = Some(0);
        repo.save_execution_record(&exec_record)
            .expect("update exec record");

        let updated_exec = repo
            .get_execution_record(&exec_id)
            .expect("get exec updated")
            .expect("exec exists");
        assert_eq!(updated_exec.status, ExecutionRecordStatus::Completed);
        assert_eq!(updated_exec.exit_code, Some(0));

        let execs_for_recipe = repo
            .list_execution_records_for_recipe(&recipe_id)
            .expect("list execs for recipe");
        assert_eq!(execs_for_recipe.len(), 1);

        // 8. Annotations
        let ann_img = AnnotationRecord::new_image_pin(
            "figures/loss_curve.png",
            1,
            0.35,
            0.75,
            1,
            "Add error bars at epoch 50",
            "alice",
        )
        .with_side_chat("session_side_01");
        let ann_img_id = ann_img.id.clone();
        repo.save_annotation(&ann_img).expect("save annotation");

        let pending = repo.list_pending_annotations().expect("list pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].actor, "alice");

        let artifact_anns = repo
            .list_annotations_for_artifact("figures/loss_curve.png", Some(1))
            .expect("list anns");
        assert_eq!(artifact_anns.len(), 1);
        assert_eq!(
            artifact_anns[0].side_chat_session_id.as_deref(),
            Some("session_side_01")
        );

        repo.update_annotation_status(&ann_img_id, AnnotationStatus::Submitted, Some("turn_101"))
            .expect("update status");
        let updated_ann = repo
            .list_annotations_for_artifact("figures/loss_curve.png", Some(1))
            .expect("list anns updated");
        assert_eq!(updated_ann[0].status, AnnotationStatus::Submitted);
        assert_eq!(
            updated_ann[0].submitted_turn_id.as_deref(),
            Some("turn_101")
        );

        let pending_after = repo.list_pending_annotations().expect("list pending after");
        assert_eq!(pending_after.len(), 0);

        // 9. Artifact List & Lineage DAG
        let all_artifacts = repo.list_all_artifacts().expect("list all artifacts");
        assert_eq!(all_artifacts.len(), 1);
        assert_eq!(all_artifacts[0].artifact_path, "eval_table.csv");
        assert_eq!(all_artifacts[0].latest_version, 1);
        assert_eq!(all_artifacts[0].latest_content_hash, "hash_csv");

        let graph = repo
            .get_artifact_lineage_graph(None)
            .expect("get lineage graph");
        assert!(!graph.nodes.is_empty());
        let csv_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "artifact:eval_table.csv:v1");
        assert!(csv_node.is_some());

        // 10. Notes & Version History
        let mut note = NoteRecord::new(
            "Convergence Architecture Notes",
            "# Custos SADE Convergence\nStep 6 artifact identity verified.",
            Some("sess_res_01".into()),
            None,
            vec!["sade".into(), "architecture".into()],
        )
        .expect("create note");
        let note_id = note.id.clone();
        repo.save_note(&note).expect("save note");

        let fetched_note = repo
            .get_note(&note_id)
            .expect("get note")
            .expect("note exists");
        assert_eq!(fetched_note.title, "Convergence Architecture Notes");
        assert_eq!(fetched_note.version, 1);
        assert_eq!(fetched_note.tags, vec!["sade", "architecture"]);

        let prev_ver = note
            .update(
                Some("Convergence Architecture Notes (Updated)".into()),
                "# Custos SADE Convergence\nStep 6 updated with lineage DAG graph.".into(),
                None,
            )
            .expect("update note");
        repo.save_note(&note).expect("save updated note");
        repo.save_note_version(&prev_ver).expect("save note version");

        let note_versions = repo.list_note_versions(&note_id).expect("list versions");
        assert_eq!(note_versions.len(), 1);
        assert_eq!(note_versions[0].version, 1);

        let notes_for_sess = repo.list_notes(Some("sess_res_01")).expect("list notes");
        assert_eq!(notes_for_sess.len(), 1);
        assert_eq!(notes_for_sess[0].version, 2);

        // 8. Reviewer Records (Step 8)
        let review = ReviewerRecord::new(
            ReviewTargetType::Diff,
            "patch_hash_987",
            "verifier:security-linter",
            ReviewMethod::AutomatedVerifier,
            ReviewStatus::Approved,
            "No path traversal or command injection vulnerabilities found.",
            Some("sha256:fedcba".into()),
            vec![ReviewFinding {
                severity: custos_domain::FindingSeverity::Info,
                criterion: "no_command_injection".to_string(),
                message: "Passed fail-closed validation".into(),
                file_path: Some("crates/custos-runtime/src/lib.rs".into()),
                line_number: Some(42),
            }],
        ).expect("create review");

        repo.save_review(&review).expect("save review");
        let fetched_rev = repo.get_review(&review.id).expect("get review").expect("review exists");
        assert_eq!(fetched_rev.target_id, "patch_hash_987");
        assert_eq!(fetched_rev.status, ReviewStatus::Approved);
        assert!(fetched_rev.is_fresh);
        assert_eq!(fetched_rev.findings.len(), 1);

        let filtered_revs = repo.list_reviews(Some(ReviewTargetType::Diff), Some("patch_hash_987")).expect("list reviews");
        assert_eq!(filtered_revs.len(), 1);

        repo.mark_review_stale(&review.id).expect("mark stale");
        let stale_rev = repo.get_review(&review.id).expect("get review").expect("review exists");
        assert!(!stale_rev.is_fresh);
    }
}
