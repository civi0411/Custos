//! Research Synthesis & Handoff to Coding Domain Models
//!
//! Packages verified claims (L1-L3), reproduction recipes, artifact lineage digests,
//! and verifier records into an auditable proposal before materializing invariant-governed
//! coding tasks and execution worktrees.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::new_id;
use crate::claim::ClaimGroundingLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisProposalStatus {
    Draft,
    Submitted,
    HandoffCompleted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimHandoffSummary {
    pub claim_id: String,
    pub statement: String,
    pub level: ClaimGroundingLevel,
    pub confidence_score: f32,
    pub has_fresh_review: bool,
    pub sealed_proof_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeHandoffSummary {
    pub recipe_id: String,
    pub name: String,
    pub command: String,
    pub inputs_count: usize,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchSynthesisProposal {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub claims: Vec<ClaimHandoffSummary>,
    pub recipes: Vec<RecipeHandoffSummary>,
    pub artifact_paths: Vec<String>,
    pub workspace_id: Option<String>,
    pub target_branch: Option<String>,
    pub caveats: Vec<String>,
    pub status: SynthesisProposalStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ResearchSynthesisProposal {
    pub fn new(
        title: impl Into<String>,
        summary: impl Into<String>,
        claims: Vec<ClaimHandoffSummary>,
        recipes: Vec<RecipeHandoffSummary>,
        artifact_paths: Vec<String>,
        workspace_id: Option<String>,
        target_branch: Option<String>,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        let summary = summary.into();

        if title.trim().is_empty() {
            return Err(DomainError::Validation("Proposal title cannot be empty".into()));
        }
        if summary.trim().is_empty() {
            return Err(DomainError::Validation("Proposal summary cannot be empty".into()));
        }

        let caveats = Self::generate_caveats(&claims, &recipes);
        let now = chrono::Utc::now().timestamp();

        Ok(Self {
            id: new_id("synth_prop"),
            title,
            summary,
            claims,
            recipes,
            artifact_paths,
            workspace_id,
            target_branch,
            caveats,
            status: SynthesisProposalStatus::Draft,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn generate_caveats(
        claims: &[ClaimHandoffSummary],
        recipes: &[RecipeHandoffSummary],
    ) -> Vec<String> {
        let mut caveats = Vec::new();

        for claim in claims {
            if claim.level == ClaimGroundingLevel::L0Ungrounded {
                caveats.push(format!(
                    "Claim '{}' is ungrounded (L0) and lacks formal citation or verification",
                    claim.statement
                ));
            } else if !claim.has_fresh_review && claim.level < ClaimGroundingLevel::L2Verified {
                caveats.push(format!(
                    "Claim '{}' has no active verifier record; verification remains pending in coding",
                    claim.statement
                ));
            }
        }

        if recipes.is_empty() && !claims.is_empty() {
            caveats.push("No reproduction recipes specified for empirical claim validation".into());
        }

        caveats
    }

    pub fn mark_completed(&mut self) {
        self.status = SynthesisProposalStatus::HandoffCompleted;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveSynthesisProposalParams {
    pub id: Option<String>,
    pub title: String,
    pub summary: String,
    pub claim_ids: Vec<String>,
    pub recipe_ids: Vec<String>,
    pub artifact_paths: Option<Vec<String>>,
    pub workspace_id: Option<String>,
    pub target_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffToCodingParams {
    pub proposal_id: Option<String>,
    pub title: String,
    pub claim_ids: Vec<String>,
    pub recipe_ids: Vec<String>,
    pub artifact_paths: Option<Vec<String>>,
    pub workspace_id: Option<String>,
    pub target_branch: Option<String>,
    pub enforce_verification: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffToCodingResult {
    pub handoff_id: String,
    pub proposal_id: Option<String>,
    pub task_ids: Vec<String>,
    pub workspace_id: Option<String>,
    pub target_branch: Option<String>,
    pub verified_claims_count: usize,
    pub converted_recipes_count: usize,
    pub caveats: Vec<String>,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesis_proposal_creation_and_caveat_generation() {
        let claims = vec![
            ClaimHandoffSummary {
                claim_id: "claim_1".into(),
                statement: "LLM agents require bounded execution sandbox".into(),
                level: ClaimGroundingLevel::L2Verified,
                confidence_score: 0.95,
                has_fresh_review: true,
                sealed_proof_uri: Some("cas://bafy123".into()),
            },
            ClaimHandoffSummary {
                claim_id: "claim_2".into(),
                statement: "Raw unverified claim".into(),
                level: ClaimGroundingLevel::L0Ungrounded,
                confidence_score: 0.2,
                has_fresh_review: false,
                sealed_proof_uri: None,
            },
        ];

        let recipes = vec![
            RecipeHandoffSummary {
                recipe_id: "recipe_1".into(),
                name: "Run test suite".into(),
                command: "cargo test".into(),
                inputs_count: 0,
                outputs: vec!["target/debug".into()],
            },
        ];

        let proposal = ResearchSynthesisProposal::new(
            "Sandbox Synthesis",
            "Synthesis of sandbox claims into tasks",
            claims,
            recipes,
            vec!["artifacts/sandbox.md".into()],
            Some("ws_default".into()),
            Some("feature/sandbox".into()),
        ).unwrap();

        assert_eq!(proposal.status, SynthesisProposalStatus::Draft);
        assert_eq!(proposal.caveats.len(), 1);
        assert!(proposal.caveats[0].contains("ungrounded (L0)"));
    }
}
