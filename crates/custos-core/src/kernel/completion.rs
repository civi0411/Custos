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
                        // Strict Invariant INV-02: Must be passed and actively in Pass status (never Stale)
                        if !claim.passed || claim.status != EvidenceStatus::Pass {
                            return false;
                        }

                        // Version matching invariant: if both claim and task specify source_version, they must match
                        if let (Some(claim_ver), Some(task_ver)) = (
                            &claim.source_version,
                            task.metadata.get("source_version").and_then(|v| v.as_str()),
                        ) {
                            if claim_ver != task_ver {
                                return false;
                            }
                        }

                        match req.kind {
                            EvidenceKind::FileAnchor => {
                                claim.verifier_id == "file_anchor"
                                    || claim.verifier_id == "hash"
                                    || claim.verifier_id == "citation"
                            }
                            EvidenceKind::SymbolAnchor => {
                                claim.verifier_id == "symbol_anchor"
                                    || claim.verifier_id == "exact_match"
                            }
                            EvidenceKind::TestResult => {
                                claim.verifier_id == "command_exit_code"
                                    || claim.verifier_id == "test_result"
                            }
                            EvidenceKind::Diff => {
                                claim.verifier_id == "diff"
                                    || claim.verifier_id == "patch_preview"
                            }
                        }
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
}
