//! Research Persistence Repository
//!
//! SQLite-backed repository managing scientific Sources, Cryptographic Passage Anchors,
//! Claims Matrix (L0-L3), Experiment Runs, and Artifact Lineage.

use crate::connection::DbConnection;
use custos_domain::{
    ArtifactLineageNode, ClaimEvidenceLink, ClaimGroundingLevel, DomainError, EnvSnapshot,
    EvidenceRelation, PassageAnchor, ResearchClaim, ResearchExperimentRun, SourceRecord,
};
use rusqlite::params;

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

    pub fn list_anchors_for_source(&self, source_id: &str) -> Result<Vec<PassageAnchor>, DomainError> {
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
            ).map_err(|e| DomainError::Validation(e.to_string()))?;
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

                Ok((id, statement, level, conf as f32, invariants, sealed, created_at))
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
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(run_id) DO UPDATE SET
                status = excluded.status,
                wall_ms = excluded.wall_ms,
                output_merkle_root = excluded.output_merkle_root,
                cas_log_uri = excluded.cas_log_uri",
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
                let env: EnvSnapshot = serde_json::from_str(&env_json).unwrap_or(EnvSnapshot {
                    python_version: "3.11".to_string(),
                    lockfile_hash: "none".to_string(),
                    package_count: None,
                    hardware: "default".to_string(),
                    platform: None,
                });

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

    pub fn list_artifact_lineage(&self, artifact_path: &str) -> Result<Vec<ArtifactLineageNode>, DomainError> {
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
        let anchors = repo.list_anchors_for_source("src_123").expect("list anchors");
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
                exact_text: Some("Multi-Head Attention consists of several attention layers".to_string()),
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
        repo.record_artifact_lineage(&lineage).expect("record lineage");
        let lineage_list = repo.list_artifact_lineage("eval_table.csv").expect("list lineage");
        assert_eq!(lineage_list.len(), 1);
        assert_eq!(lineage_list[0].version, 1);
        assert_eq!(lineage_list[0].content_hash, "hash_csv");
    }
}
