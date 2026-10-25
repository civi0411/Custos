use super::ResearchRepository;
use custos_domain::{
    DomainError, ReviewFinding, ReviewMethod, ReviewStatus, ReviewTargetType, ReviewerRecord,
};
use rusqlite::params;

impl ResearchRepository {
    // Reviewer Records & Evidence Criteria
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
                       FROM reviewer_records WHERE 1=1"
            .to_string();

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

    pub(crate) fn row_to_review(row: &rusqlite::Row) -> rusqlite::Result<ReviewerRecord> {
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
}
