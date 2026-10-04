//! Write-Set Conflict Checker (RFC 004 §2C, Gate 5)
//!
//! Validates that concurrent nodes in a revision do not write to overlapping paths
//! unless explicitly isolated into distinct workspace leases.

use custos_domain::oi::NodePlacement;
use custos_domain::workflow::RevisionNode;
use custos_domain::DomainError;
use std::collections::{HashMap, HashSet};

pub struct WriteSetConflictChecker;

impl WriteSetConflictChecker {
    /// Validates that concurrent nodes in a revision do not write to overlapping paths
    /// unless explicitly isolated into distinct workspace leases.
    pub fn check_conflicts(
        nodes: &[RevisionNode],
        dependencies: &[(String, String)],
        placements: Option<&[NodePlacement]>,
    ) -> Result<(), DomainError> {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in nodes {
            adj.entry(&node.node_id).or_default();
        }
        for (from, to) in dependencies {
            adj.entry(from.as_str()).or_default().push(to.as_str());
        }

        // Build transitive reachability set
        let mut reachable: HashMap<&str, HashSet<&str>> = HashMap::new();
        for node in nodes {
            let mut visited = HashSet::new();
            let mut queue = vec![node.node_id.as_str()];
            while let Some(curr) = queue.pop() {
                if let Some(neighbors) = adj.get(curr) {
                    for &next in neighbors {
                        if visited.insert(next) {
                            queue.push(next);
                        }
                    }
                }
            }
            reachable.insert(&node.node_id, visited);
        }

        let lease_map: HashMap<&str, Option<&str>> = placements
            .map(|ps| {
                ps.iter()
                    .map(|p| (p.node_id.as_str(), p.workspace_lease_id.as_deref()))
                    .collect()
            })
            .unwrap_or_default();

        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let a = &nodes[i];
                let b = &nodes[j];

                let a_reaches_b = reachable.get(a.node_id.as_str()).map_or(false, |s| s.contains(b.node_id.as_str()));
                let b_reaches_a = reachable.get(b.node_id.as_str()).map_or(false, |s| s.contains(a.node_id.as_str()));

                // Concurrent if neither depends on the other
                if !a_reaches_b && !b_reaches_a {
                    if Self::paths_overlap(&a.write_set, &b.write_set) {
                        let lease_a = lease_map.get(a.node_id.as_str()).copied().flatten();
                        let lease_b = lease_map.get(b.node_id.as_str()).copied().flatten();

                        let is_isolated = match (lease_a, lease_b) {
                            (Some(la), Some(lb)) => la != lb,
                            _ => false,
                        };

                        if !is_isolated {
                            return Err(DomainError::Validation(format!(
                                "Gate 5 Write-Set Conflict: Concurrent nodes '{}' and '{}' write to overlapping paths ({:?}, {:?}) without isolated workspace leases",
                                a.node_id, b.node_id, a.write_set, b.write_set
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn paths_overlap(set_a: &[String], set_b: &[String]) -> bool {
        for a in set_a {
            for b in set_b {
                if a == b {
                    return true;
                }
                let clean_a = a.trim_end_matches("/**").trim_end_matches("/*");
                let clean_b = b.trim_end_matches("/**").trim_end_matches("/*");
                if clean_a == clean_b || clean_a.starts_with(clean_b) || clean_b.starts_with(clean_a) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_set_conflict_detected() {
        let node_a = RevisionNode {
            node_id: "node_a".into(),
            step_name: "Coder A".into(),
            role: "coder".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 1000,
            read_set: vec![],
            write_set: vec!["workspace/src/**".into()],
            required_capabilities: vec![],
        };
        let node_b = RevisionNode {
            node_id: "node_b".into(),
            step_name: "Coder B".into(),
            role: "coder".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 1000,
            read_set: vec![],
            write_set: vec!["workspace/src/lib.rs".into()],
            required_capabilities: vec![],
        };

        // Concurrent (no dependencies) without leases -> Conflict!
        let res = WriteSetConflictChecker::check_conflicts(&[node_a.clone(), node_b.clone()], &[], None);
        assert!(res.is_err(), "Must reject un-isolated concurrent write-set overlap");

        // Isolated with separate leases -> Allowed!
        let placements = vec![
            NodePlacement {
                node_id: "node_a".into(),
                role: "coder".into(),
                backend_harness: "claude".into(),
                budget_tokens_slice: 1000,
                workspace_lease_id: Some("lease_a".into()),
            },
            NodePlacement {
                node_id: "node_b".into(),
                role: "coder".into(),
                backend_harness: "claude".into(),
                budget_tokens_slice: 1000,
                workspace_lease_id: Some("lease_b".into()),
            },
        ];
        let res_isolated = WriteSetConflictChecker::check_conflicts(&[node_a, node_b], &[], Some(&placements));
        assert!(res_isolated.is_ok(), "Must allow isolated concurrent write sets");
    }
}
