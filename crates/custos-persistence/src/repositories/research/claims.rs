use super::ResearchRepository;
use custos_domain::{
    ClaimEvidenceLink, ClaimGroundingLevel, DomainError, EvidenceRelation, ResearchClaim,
};
use rusqlite::params;

impl ResearchRepository {
    // Claims Matrix
    pub(crate) fn save_claim_conn(
        conn: &rusqlite::Connection,
        claim: &ResearchClaim,
    ) -> Result<(), DomainError> {
        if claim.id.trim().is_empty() || claim.statement.trim().is_empty() {
            return Err(DomainError::Validation("Claim id and statement cannot be empty".to_string()));
        }
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

    pub fn save_claim(&self, claim: &ResearchClaim) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        Self::save_claim_conn(&conn, claim)
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
}
