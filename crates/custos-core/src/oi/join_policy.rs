//! Pure Join Policies Evaluator (RFC 004 §2C)
//!
//! Evaluates fan-in results at synchronization join nodes.

pub use super::write_set::WriteSetConflictChecker;
use custos_domain::oi::{WorkerResult, WorkerStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinPolicy {
    /// All prerequisite nodes must achieve sufficient completion
    AllRequired,
    /// A minimum ratio of reader workers must succeed (T3 Read Fan-Out)
    CoverageThreshold { threshold: f64 },
    /// Validates branch diff integration without regression (T4 Integrator)
    IntegrationOracle,
    /// Bounded repair evaluation with maximum iteration limit (T6 Repair Loop)
    BoundedIterations {
        max_iterations: u32,
        current_iteration: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinEvaluation {
    Approved {
        merged_payload: String,
        total_tokens: u64,
        total_cost: f32,
    },
    RetryRequired {
        next_iteration: u32,
        feedback: String,
    },
    Rejected {
        reason: String,
    },
}

pub struct JoinEvaluator;

impl JoinEvaluator {
    pub fn evaluate(policy: &JoinPolicy, results: &[WorkerResult]) -> JoinEvaluation {
        if results.is_empty() {
            return JoinEvaluation::Rejected {
                reason: "No worker results provided for join evaluation".into(),
            };
        }

        let mut total_tokens = 0u64;
        let mut total_cost = 0.0f32;
        let mut payloads = Vec::new();
        let mut sufficient_count = 0usize;

        for r in results {
            total_tokens += r.tokens_used;
            total_cost += r.cost_usd;
            if r.status == WorkerStatus::Sufficient {
                sufficient_count += 1;
                payloads.push(r.payload.clone());
            }
        }

        match policy {
            JoinPolicy::AllRequired => {
                for r in results {
                    if r.status != WorkerStatus::Sufficient {
                        return JoinEvaluation::Rejected {
                            reason: format!(
                                "Node packet '{}' failed required completion with status {:?}",
                                r.packet_id, r.status
                            ),
                        };
                    }
                }
                JoinEvaluation::Approved {
                    merged_payload: payloads.join("\n---\n"),
                    total_tokens,
                    total_cost,
                }
            }

            JoinPolicy::CoverageThreshold { threshold } => {
                let ratio = sufficient_count as f64 / results.len() as f64;
                if ratio >= *threshold {
                    JoinEvaluation::Approved {
                        merged_payload: payloads.join("\n---\n"),
                        total_tokens,
                        total_cost,
                    }
                } else {
                    JoinEvaluation::Rejected {
                        reason: format!(
                            "Coverage threshold not met: {:.1}% < {:.1}% required",
                            ratio * 100.0,
                            threshold * 100.0
                        ),
                    }
                }
            }

            JoinPolicy::IntegrationOracle => {
                for r in results {
                    if r.status == WorkerStatus::Failed || r.status == WorkerStatus::Cancelled {
                        return JoinEvaluation::Rejected {
                            reason: format!(
                                "Integration oracle rejected due to failed worker packet '{}'",
                                r.packet_id
                            ),
                        };
                    }
                }
                JoinEvaluation::Approved {
                    merged_payload: payloads.join("\n---\n"),
                    total_tokens,
                    total_cost,
                }
            }

            JoinPolicy::BoundedIterations {
                max_iterations,
                current_iteration,
            } => {
                let latest = results.last().unwrap();
                if latest.status == WorkerStatus::Sufficient {
                    JoinEvaluation::Approved {
                        merged_payload: latest.payload.clone(),
                        total_tokens,
                        total_cost,
                    }
                } else if *current_iteration < *max_iterations {
                    JoinEvaluation::RetryRequired {
                        next_iteration: current_iteration + 1,
                        feedback: format!(
                            "Evaluation rejected at iteration {}: {}",
                            current_iteration, latest.payload
                        ),
                    }
                } else {
                    JoinEvaluation::Rejected {
                        reason: format!(
                            "Repair loop bounded limit reached ({} iterations exhausted)",
                            max_iterations
                        ),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_required_join() {
        let r1 = WorkerResult::success("p1", "Result 1");
        let r2 = WorkerResult::success("p2", "Result 2");
        let eval = JoinEvaluator::evaluate(&JoinPolicy::AllRequired, &[r1.clone(), r2]);
        match eval {
            JoinEvaluation::Approved { merged_payload, .. } => {
                assert!(merged_payload.contains("Result 1"));
                assert!(merged_payload.contains("Result 2"));
            }
            _ => panic!("Expected approval"),
        }

        let r3 = WorkerResult::failed("p3", "Disk full");
        let eval_fail = JoinEvaluator::evaluate(&JoinPolicy::AllRequired, &[r1, r3]);
        assert!(matches!(eval_fail, JoinEvaluation::Rejected { .. }));
    }

    #[test]
    fn test_coverage_threshold_join() {
        let policy = JoinPolicy::CoverageThreshold { threshold: 0.75 };
        let r1 = WorkerResult::success("p1", "Content 1");
        let r2 = WorkerResult::success("p2", "Content 2");
        let r3 = WorkerResult::success("p3", "Content 3");
        let r4 = WorkerResult::abstain("p4", "Timeout reading file");

        let eval = JoinEvaluator::evaluate(&policy, &[r1, r2, r3, r4]);
        assert!(matches!(eval, JoinEvaluation::Approved { .. }));
    }
}
