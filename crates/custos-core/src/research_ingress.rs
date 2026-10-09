//! Trust boundary for research drafts received through the Local API.
//! These functions do not verify source bytes or scientific claims.

use custos_domain::{
    ClaimGroundingLevel, PassageAnchor, ResearchClaim, ResearchExperimentRun, SourceRecord,
};
use sha2::{Digest, Sha256};

pub fn source_draft(mut source: SourceRecord) -> Result<SourceRecord, String> {
    if source.id.trim().is_empty() || source.title.trim().is_empty() {
        return Err("Source id and title are required".into());
    }
    source.verified = false;
    Ok(source)
}

pub fn passage_draft(mut anchor: PassageAnchor) -> Result<PassageAnchor, String> {
    if anchor.id.trim().is_empty()
        || anchor.source_id.trim().is_empty()
        || anchor.exact_text.trim().is_empty()
        || anchor.start_offset >= anchor.end_offset
    {
        return Err("Passage requires id, source, text, and valid offsets".into());
    }
    let digest = Sha256::digest(anchor.exact_text.as_bytes());
    anchor.passage_hash = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    Ok(anchor)
}

pub fn claim_draft(mut claim: ResearchClaim) -> Result<ResearchClaim, String> {
    if claim.id.trim().is_empty() || claim.statement.trim().is_empty() {
        return Err("Claim id and statement are required".into());
    }
    claim.level = ClaimGroundingLevel::L0Ungrounded;
    claim.sealed_proof_uri = None;
    claim.confidence_score = 0.0;
    for link in &mut claim.evidence_links {
        link.verified_by = "unreviewed_client_proposal".into();
    }
    Ok(claim)
}

pub fn experiment_draft(mut run: ResearchExperimentRun) -> Result<ResearchExperimentRun, String> {
    if run.run_id.trim().is_empty() || run.session_id.trim().is_empty() {
        return Err("Experiment id and session id are required".into());
    }
    if run.status != "pending" {
        return Err("Only pending experiment drafts may be submitted by a client".into());
    }
    run.reproducibility = "unverified".into();
    run.wall_ms = 0;
    run.output_merkle_root.clear();
    run.sade_permit_id = None;
    run.cas_log_uri = None;
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_cannot_mint_claim_verification() {
        let claim = ResearchClaim {
            id: "c1".into(),
            statement: "A".into(),
            level: ClaimGroundingLevel::L3Sealed,
            confidence_score: 1.0,
            invariants: vec![],
            evidence_links: vec![],
            created_at: 0,
            sealed_proof_uri: Some("cas://forged".into()),
        };
        let draft = claim_draft(claim).unwrap();
        assert_eq!(draft.level, ClaimGroundingLevel::L0Ungrounded);
        assert_eq!(draft.confidence_score, 0.0);
        assert!(draft.sealed_proof_uri.is_none());
    }
}
