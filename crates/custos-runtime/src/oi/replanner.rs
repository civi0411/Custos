//! Event-Driven Replanner (RFC 003 §2B, RFC 004 §2D)
//!
//! Generates delta ReplanBrief upon failed assumptions or verification triggers.
//! Enforces INV-03: nodes that produced an Uncertain outbox effect are NEVER blind-retried.

use custos_core::contracts::storage::{OutboxEntry, OutboxStatus};
use custos_domain::oi::{ReplanBrief, ReplanTrigger};
use custos_domain::workflow::{RevisionNode, WorkflowRevision};
use custos_domain::DomainError;
use std::collections::{HashMap, HashSet};

pub struct Replanner;

impl Replanner {
    /// Creates a delta ReplanBrief by inspecting failure context, reachability, and outbox state.
    /// Strictly enforces INV-03: blind retry of nodes with Uncertain side-effects is blocked.
    pub fn create_brief(
        task_id: &str,
        current_revision: &WorkflowRevision,
        trigger: ReplanTrigger,
        failed_node_id: Option<&str>,
        reason: &str,
        outbox_entries: &[OutboxEntry],
    ) -> Result<ReplanBrief, DomainError> {
        let mut final_reason = reason.to_string();

        // Enforce INV-03: Check if failed node or any task outbox entry is in Uncertain status
        let has_uncertain_effect = outbox_entries.iter().any(|entry| {
            entry.status == OutboxStatus::Uncertain
                && failed_node_id.map_or(true, |fid| entry.action_id.contains(fid))
        });

        if has_uncertain_effect {
            final_reason = format!(
                "[INV-03 Guardrail] Uncertain external effect detected for task '{}'. Blind retry forbidden; human reconciliation required. Original reason: {}",
                task_id, reason
            );
        }

        let mut brief = ReplanBrief::new(
            task_id,
            trigger,
            failed_node_id.map(String::from),
            final_reason,
        );

        if let Some(fid) = failed_node_id {
            // Build dependency reachability to isolate transitive downstream nodes
            let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
            for (from, to) in &current_revision.dependencies {
                adj.entry(from.as_str()).or_default().push(to.as_str());
            }

            let mut affected = HashSet::new();
            let mut queue = vec![fid];
            affected.insert(fid);

            while let Some(curr) = queue.pop() {
                if let Some(children) = adj.get(curr) {
                    for &child in children {
                        if affected.insert(child) {
                            queue.push(child);
                        }
                    }
                }
            }

            for node in &current_revision.nodes {
                if affected.contains(node.node_id.as_str()) {
                    brief.affected_node_ids.push(node.node_id.clone());
                } else {
                    brief.preserved_node_ids.push(node.node_id.clone());
                }
            }
        } else {
            // Global failure: all nodes affected
            brief.affected_node_ids = current_revision
                .nodes
                .iter()
                .map(|n| n.node_id.clone())
                .collect();
        }

        Ok(brief)
    }

    /// Applies a delta ReplanBrief to produce a new WorkflowRevision with preserved nodes retained.
    pub fn apply_delta(
        current_revision: &WorkflowRevision,
        brief: &ReplanBrief,
        new_proposal_id: &str,
        replacement_nodes: Vec<RevisionNode>,
        new_dependencies: Vec<(String, String)>,
    ) -> Result<WorkflowRevision, DomainError> {
        let mut new_rev = WorkflowRevision::new(
            &current_revision.task_id,
            new_proposal_id,
            current_revision.revision_number + 1,
        );

        let preserved_set: HashSet<&str> = brief
            .preserved_node_ids
            .iter()
            .map(|s| s.as_str())
            .collect();

        // 1. Retain preserved nodes from prior revision
        for node in &current_revision.nodes {
            if preserved_set.contains(node.node_id.as_str()) {
                new_rev.nodes.push(node.clone());
            }
        }

        // 2. Append replacement delta nodes
        new_rev.nodes.extend(replacement_nodes);

        // 3. Keep internal dependencies between preserved nodes
        for (from, to) in &current_revision.dependencies {
            if preserved_set.contains(from.as_str()) && preserved_set.contains(to.as_str()) {
                new_rev.dependencies.push((from.clone(), to.clone()));
            }
        }

        // 4. Attach new dependencies connecting replacement nodes
        new_rev.dependencies.extend(new_dependencies);

        // 5. Carry forward obligations
        new_rev.obligations = current_revision.obligations.clone();

        Ok(new_rev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replan_brief_delta_isolation() {
        let mut rev = WorkflowRevision::new("task_1", "prop_1", 1);
        rev.nodes.push(RevisionNode {
            node_id: "node_1".into(),
            step_name: "Fetch".into(),
            role: "reader".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 1000,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        });
        rev.nodes.push(RevisionNode {
            node_id: "node_2".into(),
            step_name: "Compute".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 2000,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        });
        rev.nodes.push(RevisionNode {
            node_id: "node_3".into(),
            step_name: "Verify".into(),
            role: "verifier".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 1000,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        });
        rev.dependencies.push(("node_1".into(), "node_2".into()));
        rev.dependencies.push(("node_2".into(), "node_3".into()));

        // node_2 fails
        let brief = Replanner::create_brief(
            "task_1",
            &rev,
            ReplanTrigger::TestFailure,
            Some("node_2"),
            "Assertion failed in compute step",
            &[],
        )
        .unwrap();

        // Node 1 is preserved, Nodes 2 and 3 are affected
        assert_eq!(brief.preserved_node_ids, vec!["node_1"]);
        assert!(brief.affected_node_ids.contains(&"node_2".into()));
        assert!(brief.affected_node_ids.contains(&"node_3".into()));

        // Apply delta
        let replacement = vec![RevisionNode {
            node_id: "node_2_repaired".into(),
            step_name: "Compute Repaired".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 2500,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        }];
        let new_deps = vec![("node_1".into(), "node_2_repaired".into())];

        let new_rev = Replanner::apply_delta(&rev, &brief, "prop_2", replacement, new_deps).unwrap();
        assert_eq!(new_rev.revision_number, 2);
        assert_eq!(new_rev.nodes.len(), 2); // node_1 + node_2_repaired
        assert_eq!(new_rev.nodes[0].node_id, "node_1");
        assert_eq!(new_rev.nodes[1].node_id, "node_2_repaired");
    }

    #[test]
    fn test_replan_inv03_uncertain_effect_guardrail() {
        let rev = WorkflowRevision::new("task_inv03", "prop_1", 1);
        let outbox = vec![OutboxEntry {
            id: "out_1".into(),
            task_id: "task_inv03".into(),
            action_id: "node_api_call".into(),
            permit_id: "permit_1".into(),
            argument_digest: "digest".into(),
            idempotency_key: None,
            status: OutboxStatus::Uncertain,
            created_at: chrono::Utc::now(),
            receipt: None,
        }];

        let brief = Replanner::create_brief(
            "task_inv03",
            &rev,
            ReplanTrigger::Timeout,
            Some("node_api_call"),
            "Socket timed out while waiting for HTTP response",
            &outbox,
        )
        .unwrap();

        assert!(brief.reason.contains("[INV-03 Guardrail]"));
        assert!(brief.reason.contains("Uncertain external effect detected"));
    }
}
