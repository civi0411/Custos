use super::ResearchRepository;
use custos_domain::{DomainError, ResearchSynthesisProposal, SynthesisProposalStatus};
use rusqlite::params;

impl ResearchRepository {
    // Research Synthesis Proposals
    pub fn save_synthesis_proposal(
        &self,
        proposal: &ResearchSynthesisProposal,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let claims_json = serde_json::to_string(&proposal.claims)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let recipes_json = serde_json::to_string(&proposal.recipes)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let artifact_paths_json = serde_json::to_string(&proposal.artifact_paths)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let caveats_json = serde_json::to_string(&proposal.caveats)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let status_str = match proposal.status {
            SynthesisProposalStatus::Draft => "draft",
            SynthesisProposalStatus::Submitted => "submitted",
            SynthesisProposalStatus::HandoffCompleted => "handoff_completed",
            SynthesisProposalStatus::Rejected => "rejected",
        };

        conn.execute(
            "INSERT INTO research_synthesis_proposals (
                id, title, summary, claims_json, recipes_json, artifact_paths_json,
                workspace_id, target_branch, caveats_json, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                summary = excluded.summary,
                claims_json = excluded.claims_json,
                recipes_json = excluded.recipes_json,
                artifact_paths_json = excluded.artifact_paths_json,
                workspace_id = excluded.workspace_id,
                target_branch = excluded.target_branch,
                caveats_json = excluded.caveats_json,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                proposal.id,
                proposal.title,
                proposal.summary,
                claims_json,
                recipes_json,
                artifact_paths_json,
                proposal.workspace_id,
                proposal.target_branch,
                caveats_json,
                status_str,
                proposal.created_at,
                proposal.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_synthesis_proposal(
        &self,
        id: &str,
    ) -> Result<Option<ResearchSynthesisProposal>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, summary, claims_json, recipes_json, artifact_paths_json,
                        workspace_id, target_branch, caveats_json, status, created_at, updated_at
                 FROM research_synthesis_proposals WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], Self::row_to_synthesis_proposal)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match rows.next() {
            Some(row) => Ok(Some(
                row.map_err(|e| DomainError::Validation(e.to_string()))?,
            )),
            None => Ok(None),
        }
    }

    pub fn list_synthesis_proposals(&self) -> Result<Vec<ResearchSynthesisProposal>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, summary, claims_json, recipes_json, artifact_paths_json,
                        workspace_id, target_branch, caveats_json, status, created_at, updated_at
                 FROM research_synthesis_proposals ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let proposals = stmt
            .query_map([], Self::row_to_synthesis_proposal)
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(proposals)
    }

    fn row_to_synthesis_proposal(
        row: &rusqlite::Row,
    ) -> rusqlite::Result<ResearchSynthesisProposal> {
        let claims_json: String = row.get(3)?;
        let claims = serde_json::from_str(&claims_json).unwrap_or_default();
        let recipes_json: String = row.get(4)?;
        let recipes = serde_json::from_str(&recipes_json).unwrap_or_default();
        let artifact_paths_json: String = row.get(5)?;
        let artifact_paths = serde_json::from_str(&artifact_paths_json).unwrap_or_default();
        let caveats_json: String = row.get(8)?;
        let caveats = serde_json::from_str(&caveats_json).unwrap_or_default();
        let status_str: String = row.get(9)?;
        let status = match status_str.as_str() {
            "submitted" => SynthesisProposalStatus::Submitted,
            "handoff_completed" => SynthesisProposalStatus::HandoffCompleted,
            "rejected" => SynthesisProposalStatus::Rejected,
            _ => SynthesisProposalStatus::Draft,
        };

        Ok(ResearchSynthesisProposal {
            id: row.get(0)?,
            title: row.get(1)?,
            summary: row.get(2)?,
            claims,
            recipes,
            artifact_paths,
            workspace_id: row.get(6)?,
            target_branch: row.get(7)?,
            caveats,
            status,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }
}
