//! Orchestration Intelligence (OI) Snapshot Extractor (RFC 003 §2A)
//!
//! Extracts an immutable DecisionSnapshot from the current state of reality:
//! Task, Budget, Outbox, and Contract criteria.

use crate::contracts::storage::{OutboxPort, OutboxStatus};
use custos_domain::{DecisionSnapshot, DomainError, Task};

pub struct DecisionSnapshotExtractor;

impl DecisionSnapshotExtractor {
    pub async fn extract_snapshot(
        task: &Task,
        outbox: Option<&dyn OutboxPort>,
        remaining_budget_tokens: u64,
        remaining_budget_usd: f32,
    ) -> Result<DecisionSnapshot, DomainError> {
        let (pending_effects, uncertain_effects) = if let Some(ob) = outbox {
            let pending = ob
                .list_by_task_and_status(&task.id, OutboxStatus::Pending)
                .await?
                .len();
            let uncertain = ob
                .list_by_task_and_status(&task.id, OutboxStatus::Uncertain)
                .await?
                .len();
            (pending, uncertain)
        } else {
            (0, 0)
        };

        let (criteria, required_capabilities) = if let Some(ref contract) = task.contract {
            let crit = contract
                .evidence_requirements
                .iter()
                .map(|e| format!("{:?}", e.kind))
                .collect();
            let caps = contract.required_capabilities.clone();
            (crit, caps)
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(DecisionSnapshot {
            task_id: task.id.clone(),
            task_version: task.epoch,
            policy_version: "custos.policy.v1".into(),
            remaining_budget_tokens,
            remaining_budget_usd,
            pending_effects_count: pending_effects,
            uncertain_effects_count: uncertain_effects,
            active_worker_runs_count: if task.status == custos_domain::TaskStatus::Running {
                1
            } else {
                0
            },
            criteria,
            pinned_model: None,
            required_capabilities,
            egress_rules: Vec::new(),
            unknowns: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{ContractEvidence, EvidenceKind, TaskContract, TaskStatus};

    #[tokio::test]
    async fn test_extract_snapshot_from_task() {
        let mut task = Task::new("task_snapshot_01".into(), "Snapshot Task".into());
        task.status = TaskStatus::Running;
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "test_contract".into(),
            description: "Test".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::FileAnchor,
                required: true,
            }],
        });

        let snapshot = DecisionSnapshotExtractor::extract_snapshot(&task, None, 50_000, 5.0)
            .await
            .unwrap();

        assert_eq!(snapshot.task_id, "task_snapshot_01");
        assert_eq!(snapshot.remaining_budget_tokens, 50_000);
        assert_eq!(snapshot.remaining_budget_usd, 5.0);
        assert_eq!(snapshot.active_worker_runs_count, 1);
        assert_eq!(snapshot.criteria, vec!["FileAnchor".to_string()]);
    }
}
