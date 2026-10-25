//! Research sources, passage anchors, claims, and lineage use cases.

use crate::api::*;
use custos_domain::{PassageAnchor, ResearchClaim, SourceRecord};

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research_sources(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_SOURCES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_sources() {
                    Ok(sources) => match serde_json::to_value(&sources) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_SOURCES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let src: SourceRecord = match serde_json::from_value(req.params) {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid source payload: {e}"))
                    }
                };
                let file_bytes = src.local_path.as_ref().and_then(|p| std::fs::read(p).ok());
                let src = match custos_core::research_ingress::source_draft_with_bytes(
                    src,
                    file_bytes.as_deref(),
                ) {
                    Ok(src) => src,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_source(&src) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": src.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let source_id = match req.params.get("source_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing source_id parameter"),
                };
                match research.list_anchors_for_source(source_id) {
                    Ok(anchors) => match serde_json::to_value(&anchors) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let anchor: PassageAnchor = match serde_json::from_value(req.params) {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid anchor payload: {e}"))
                    }
                };
                let anchor = match custos_core::research_ingress::passage_draft(anchor) {
                    Ok(anchor) => anchor,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_sources() {
                    Ok(sources) if sources.iter().any(|s| s.id == anchor.source_id) => {}
                    Ok(_) => return ApiResponse::error(req.id, "Passage source does not exist"),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_anchor(&anchor) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anchor.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_claims() {
                    Ok(claims) => match serde_json::to_value(&claims) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim: ResearchClaim = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid claim payload: {e}"))
                    }
                };
                let claim = match custos_core::research_ingress::claim_draft(claim) {
                    Ok(claim) => claim,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_claim(&claim) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": claim.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_LINEAGE_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Missing artifact_path parameter"),
                };
                match research.list_artifact_lineage(artifact_path) {
                    Ok(lineage) => match serde_json::to_value(&lineage) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown research sources method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_domain::ClaimGroundingLevel;
    use custos_persistence::SqliteTaskStore;

    #[tokio::test]
    async fn test_ingest_source_and_claim_transaction_rollback() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.research();

        // Baseline: no sources, no claims
        assert_eq!(repo.list_sources().unwrap().len(), 0);
        assert_eq!(repo.list_claims().unwrap().len(), 0);

        // 1. Success case: atomic insert of valid source and valid claim
        let source1 = SourceRecord {
            id: "src-1".to_string(),
            source_type: "paper".to_string(),
            title: "Valid Paper 1".to_string(),
            doi: Some("10.1234/paper1".to_string()),
            authors: vec!["Alice".to_string()],
            year: Some(2026),
            content_hash: "hash1".to_string(),
            local_path: None,
            verified: true,
            abstract_text: Some("Abstract 1".to_string()),
            created_at: 100,
        };
        let claim1 = ResearchClaim {
            id: "claim-1".to_string(),
            statement: "Claim 1 holds true".to_string(),
            level: ClaimGroundingLevel::L1Cited,
            confidence_score: 0.95,
            evidence_links: vec![],
            invariants: vec![],
            sealed_proof_uri: None,
            created_at: 100,
        };

        repo.ingest_source_and_claim_atomic(&source1, &claim1).unwrap();
        assert_eq!(repo.list_sources().unwrap().len(), 1);
        assert_eq!(repo.list_claims().unwrap().len(), 1);

        // 2. Rollback case: valid source2 paired with an invalid claim (empty statement)
        let source2 = SourceRecord {
            id: "src-2-should-rollback".to_string(),
            source_type: "paper".to_string(),
            title: "Uncommitted Source".to_string(),
            doi: Some("10.1234/paper2".to_string()),
            authors: vec!["Bob".to_string()],
            year: Some(2026),
            content_hash: "hash2".to_string(),
            local_path: None,
            verified: false,
            abstract_text: None,
            created_at: 200,
        };

        let broken_claim = ResearchClaim {
            id: "claim-2".to_string(),
            statement: "".to_string(), // Triggers validation error during atomic ingestion
            level: ClaimGroundingLevel::L0Ungrounded,
            confidence_score: 0.1,
            evidence_links: vec![],
            invariants: vec![],
            sealed_proof_uri: None,
            created_at: 200,
        };

        let result = repo.ingest_source_and_claim_atomic(&source2, &broken_claim);
        assert!(result.is_err(), "Transaction should fail and abort on invalid claim");

        // Verify Rollback: source2 was NOT committed to the database!
        let sources = repo.list_sources().unwrap();
        assert_eq!(sources.len(), 1, "Only source1 should exist in the repository");
        assert_eq!(sources[0].id, "src-1");
        assert!(!sources.iter().any(|s| s.id == "src-2-should-rollback"));

        let claims = repo.list_claims().unwrap();
        assert_eq!(claims.len(), 1, "Only claim1 should exist in the repository");
        assert_eq!(claims[0].id, "claim-1");
    }
}
