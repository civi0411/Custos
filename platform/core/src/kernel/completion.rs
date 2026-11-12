//! Task Completion & Terminal State Logic
//!
//! Enforces completion gate checks and evidence verification before a task succeeds.

use custos_domain::{
    DomainError, EvidenceKind, EvidenceStatus, Task, TaskStatus, VerificationClaim,
};

pub struct CompletionGate;

impl CompletionGate {
    /// Validates whether a task meets criteria to complete successfully
    pub fn can_complete(task: &Task) -> Result<(), DomainError> {
        Self::can_complete_with_evidence(task, &[])
    }

    /// Validates whether a task meets criteria to complete successfully,
    /// enforcing proof-closure against required contract evidence and freshness (INV-02).
    pub fn can_complete_with_evidence(
        task: &Task,
        claims: &[VerificationClaim],
    ) -> Result<(), DomainError> {
        if task.status != TaskStatus::Running {
            return Err(DomainError::Validation(format!(
                "Task {} must be in Running status to complete, but is in {}",
                task.id, task.status
            )));
        }

        // Expected source version from task metadata or active revision
        let expected_source_version = task
            .metadata
            .get("source_version")
            .and_then(|v| v.as_str())
            .or_else(|| {
                task.active_revision
                    .as_ref()
                    .and_then(|r| r.base_source_version.as_deref())
            });

        // Any claim passed must strictly belong to this task and have valid structure
        for claim in claims {
            if claim.task_id != task.id {
                return Err(DomainError::InvariantViolation(format!(
                    "Foreign claim violation: Claim '{}' task_id '{}' does not match task '{}'",
                    claim.id, claim.task_id, task.id
                )));
            }
            if claim.id.trim().is_empty() {
                return Err(DomainError::InvariantViolation("Claim ID cannot be empty".into()));
            }
            if claim.verifier_id.trim().is_empty() {
                return Err(DomainError::InvariantViolation(format!(
                    "Claim '{}' has empty verifier_id",
                    claim.id
                )));
            }
            if let Some(expected_ver) = expected_source_version {
                match &claim.source_version {
                    Some(claim_ver) if claim_ver == expected_ver => {}
                    Some(claim_ver) => {
                        return Err(DomainError::InvariantViolation(format!(
                            "Source version mismatch: Claim '{}' has version '{}', task requires '{}'",
                            claim.id, claim_ver, expected_ver
                        )));
                    }
                    None => {
                        return Err(DomainError::InvariantViolation(format!(
                            "Missing source version: Task requires version '{}', but claim '{}' provides none",
                            expected_ver, claim.id
                        )));
                    }
                }
            }
        }

        if let Some(ref contract) = task.contract {
            let required_evidence: Vec<_> = contract
                .evidence_requirements
                .iter()
                .filter(|req| req.required)
                .collect();

            if !required_evidence.is_empty() {
                let mut missing = Vec::new();
                for req in required_evidence {
                    let satisfied = claims.iter().any(|claim| {
                        // Strict Invariant: Claim must be bound to this exact task (F01 fix)
                        if claim.task_id != task.id {
                            return false;
                        }

                        // Strict Invariant INV-02: Must be passed and actively in Pass status (never Stale)
                        if !claim.passed || claim.status != EvidenceStatus::Pass {
                            return false;
                        }

                        // Version matching invariant
                        if let Some(expected_ver) = expected_source_version {
                            match &claim.source_version {
                                Some(claim_ver) if claim_ver == expected_ver => {}
                                _ => return false,
                            }
                        }

                        // Criterion matching invariant: if criteria are defined, claim must match a defined criterion
                        if let Some(ref rev) = task.active_contract_revision {
                            if !rev.criteria.is_empty()
                                && !rev.criteria.iter().any(|c| c.criterion_id == claim.criterion_id)
                            {
                                return false;
                            }
                        }

                        Self::is_verifier_compatible_with_kind(&req.kind, &claim.verifier_id)
                    });


                    if !satisfied {
                        missing.push(format!("{:?}", req.kind));
                    }
                }

                if !missing.is_empty() {
                    return Err(DomainError::InvariantViolation(format!(
                        "Proof-closure violation: Task {} requires verified evidence for: [{}] (Invariant I2: Evidence Binding)",
                        task.id,
                        missing.join(", ")
                    )));
                }
            }
        }

        Ok(())
    }

    /// Checks if a given verifier_id can satisfy the specified EvidenceKind requirement.
    pub fn is_verifier_compatible_with_kind(kind: &EvidenceKind, verifier_id: &str) -> bool {
        match kind {
            EvidenceKind::FileAnchor => matches!(
                verifier_id,
                "file_anchor" | "hash" | "citation" | "citation_coverage" | "citation_coverage_oracle"
            ),
            EvidenceKind::SymbolAnchor => matches!(
                verifier_id,
                "symbol_anchor" | "exact_match"
            ),
            EvidenceKind::TestResult => matches!(
                verifier_id,
                "command_exit_code"
                    | "test_result"
                    | "cargo_test_oracle"
                    | "deflated_sharpe_verifier"
                    | "lookahead_bias_checker"
                    | "pbo_overfitting_verifier"
                    | "alce"
                    | "user_acceptance_oracle"
            ),
            EvidenceKind::Diff => matches!(
                verifier_id,
                "diff" | "patch_preview"
            ),
        }
    }

    /// Verifies if a task can be cancelled
    pub fn can_cancel(task: &Task) -> Result<(), DomainError> {
        if task.status.is_terminal() {
            return Err(DomainError::Conflict(format!(
                "Task {} is already finished with status {}",
                task.id, task.status
            )));
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{ContractEvidence, EvidenceKind, TaskContract};

    #[test]
    fn test_task_without_contract_can_complete_when_running() {
        let mut task = Task::new("task_1".into(), "No contract".into());
        task.status = TaskStatus::Running;
        assert!(CompletionGate::can_complete(&task).is_ok());
    }

    #[test]
    fn test_task_not_running_cannot_complete() {
        let task = Task::new("task_2".into(), "Draft task".into());
        assert!(CompletionGate::can_complete(&task).is_err());
    }

    #[test]
    fn test_task_with_required_evidence_fails_without_claims() {
        let mut task = Task::new("task_3".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        let err = CompletionGate::can_complete(&task).unwrap_err();
        match err {
            DomainError::InvariantViolation(msg) => {
                assert!(msg.contains("Proof-closure violation"));
                assert!(msg.contains("TestResult"));
            }
            other => panic!("Expected InvariantViolation, got: {other:?}"),
        }
    }

    #[test]
    fn test_task_with_required_evidence_fails_when_claim_not_passed() {
        let mut task = Task::new("task_4".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        let failing_claim = VerificationClaim::new(
            "task_4".into(),
            "Build failed".into(),
            "command_exit_code".into(),
            false,
            serde_json::json!({"exit_code": 1}),
        );

        let err = CompletionGate::can_complete_with_evidence(&task, &[failing_claim]).unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));
    }

    #[test]
    fn test_task_with_required_evidence_succeeds_when_claim_passed() {
        let mut task = Task::new("task_5".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        let passing_claim = VerificationClaim::new(
            "task_5".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        );

        assert!(CompletionGate::can_complete_with_evidence(&task, &[passing_claim]).is_ok());
    }

    #[test]
    fn test_task_with_stale_claim_fails_completion() {
        let mut task = Task::new("task_stale".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        let mut stale_claim = VerificationClaim::new(
            "task_stale".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        );
        stale_claim.status = EvidenceStatus::Stale;

        let err = CompletionGate::can_complete_with_evidence(&task, &[stale_claim]).unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));
    }

    #[test]
    fn test_task_with_mismatched_source_version_fails_completion() {
        let mut task = Task::new("task_ver".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.metadata = serde_json::json!({ "source_version": "commit_b" });
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        // Claim from commit_a (older version)
        let old_claim = VerificationClaim::new(
            "task_ver".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        )
        .with_source_version("commit_a");

        let err = CompletionGate::can_complete_with_evidence(&task, &[old_claim]).unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));

        // Claim from commit_b (matching version)
        let fresh_claim = VerificationClaim::new(
            "task_ver".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        )
        .with_source_version("commit_b");

        assert!(CompletionGate::can_complete_with_evidence(&task, &[fresh_claim]).is_ok());
    }

    #[test]
    fn test_task_with_missing_source_version_fails_completion() {
        let mut task = Task::new("task_req_ver".into(), "Contract task".into());
        task.status = TaskStatus::Running;
        task.metadata = serde_json::json!({ "source_version": "v1.2.3" });
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        // Claim with no source_version
        let unversioned_claim = VerificationClaim::new(
            "task_req_ver".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        );

        let err = CompletionGate::can_complete_with_evidence(&task, &[unversioned_claim]).unwrap_err();
        match err {
            DomainError::InvariantViolation(msg) => {
                assert!(msg.contains("Missing source version"));
            }
            other => panic!("Expected InvariantViolation, got: {other:?}"),
        }
    }

    #[test]
    fn test_foreign_claim_fails_completion() {
        let mut task = Task::new("task_target".into(), "Target task".into());
        task.status = TaskStatus::Running;

        let foreign_claim = VerificationClaim::new(
            "task_foreign".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        );

        let err = CompletionGate::can_complete_with_evidence(&task, &[foreign_claim]).unwrap_err();
        match err {
            DomainError::InvariantViolation(msg) => {
                assert!(msg.contains("Foreign claim violation"));
            }
            other => panic!("Expected InvariantViolation, got: {other:?}"),
        }
    }

    #[test]
    fn test_task_with_mismatched_criterion_fails_completion() {
        use chrono::Utc;
        use custos_domain::{CriterionSpec, TaskContractRevision};

        let mut task = Task::new("task_crit".into(), "Crit task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });
        task.active_contract_revision = Some(TaskContractRevision {
            revision: 1,
            pack_id: "engineering".into(),
            name: "Build".into(),
            description: "Must pass tests".into(),
            required_capabilities: vec![],
            criteria: vec![CriterionSpec {
                criterion_id: "crit_unit_tests".into(),
                pack: "engineering".into(),
                rubric: "Unit tests must pass".into(),
                required_evidence: vec![ContractEvidence {
                    kind: EvidenceKind::TestResult,
                    required: true,
                }],
            }],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
            created_at: Utc::now(),
        });

        // Claim bound to non-existent criterion
        let invalid_crit_claim = VerificationClaim::new(
            "task_crit".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        )
        .with_criterion("crit_non_existent");

        let err = CompletionGate::can_complete_with_evidence(&task, &[invalid_crit_claim]).unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));

        // Claim bound to valid criterion
        let valid_crit_claim = VerificationClaim::new(
            "task_crit".into(),
            "Build succeeded".into(),
            "command_exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        )
        .with_criterion("crit_unit_tests");

        assert!(CompletionGate::can_complete_with_evidence(&task, &[valid_crit_claim]).is_ok());
    }
}
