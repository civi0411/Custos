//! OI Explain Service (RFC 003 §4, RFC 004 §2)
//!
//! Provides transparent, inspectable explanations of orchestration decisions:
//! candidate evaluations, rejection rationales, cost ranges, and compiled DAG preview.

use custos_core::oi::compiler::PlanCompiler;
use custos_core::oi::hard_filters::HardFilters;
use custos_domain::oi::{DecisionSnapshot, ExecutionTopology, StrategyProposal};
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

use super::candidate_builder::CandidateBuilder;
use super::estimator::Estimator;
use super::selector::Selector;

/// Detailed explanation for a single strategy candidate considered during planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateExplanation {
    pub topology: ExecutionTopology,
    pub harness_id: String,
    pub estimated_tokens: u64,
    pub estimated_cost_usd_min: f32,
    pub estimated_cost_usd_max: f32,
    pub estimated_latency_ms: u64,
    pub admissible: bool,
    pub rejection_reasons: Option<Vec<String>>,
}

/// Comprehensive inspection report explaining the planner's decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainReport {
    pub task_id: String,
    pub chosen_topology: ExecutionTopology,
    pub chosen_harness_id: String,
    pub estimated_tokens: u64,
    pub estimated_cost_usd: f32,
    pub reasoning: String,
    pub cost_range_tokens: (u64, u64),
    pub cost_range_usd: (f32, f32),
    pub candidates: Vec<CandidateExplanation>,
    pub compiled_nodes_preview: Vec<String>,
    pub obligations: Vec<String>,
}

/// Service for generating explain reports on orchestration decisions.
pub struct ExplainService;

impl ExplainService {
    /// Evaluates snapshot and returns an inspectable ExplainReport.
    pub fn explain(snapshot: &DecisionSnapshot) -> Result<ExplainReport, DomainError> {
        let mut candidates = CandidateBuilder::build(snapshot);
        if candidates.is_empty() {
            return Err(DomainError::Validation(
                "No candidate proposals generated for snapshot".into(),
            ));
        }

        Estimator::estimate(&mut candidates);

        let mut candidate_explanations = Vec::with_capacity(candidates.len());
        let mut min_tokens = u64::MAX;
        let mut max_tokens = 0;
        let mut min_usd = f32::MAX;
        let mut max_usd = 0.0;

        for cand in &candidates {
            if cand.est_tokens < min_tokens {
                min_tokens = cand.est_tokens;
            }
            if cand.est_tokens > max_tokens {
                max_tokens = cand.est_tokens;
            }
            if cand.est_cost_usd_min < min_usd {
                min_usd = cand.est_cost_usd_min;
            }
            if cand.est_cost_usd_max > max_usd {
                max_usd = cand.est_cost_usd_max;
            }

            let temp_proposal = StrategyProposal {
                id: custos_domain::new_id("prop"),
                task_id: snapshot.task_id.clone(),
                chosen_topology: cand.topology.clone(),
                candidate_harness: cand.harness_id.clone(),
                reasoning: String::new(),
                estimated_cost_usd: cand.est_cost_usd_max,
                estimated_tokens: cand.est_tokens,
                alternatives_considered: Vec::new(),
                assumptions: Vec::new(),
                context_strategy: Some("default_windowed".into()),
            };

            match HardFilters::evaluate(&temp_proposal, snapshot) {
                Ok(()) => {
                    candidate_explanations.push(CandidateExplanation {
                        topology: cand.topology.clone(),
                        harness_id: cand.harness_id.clone(),
                        estimated_tokens: cand.est_tokens,
                        estimated_cost_usd_min: cand.est_cost_usd_min,
                        estimated_cost_usd_max: cand.est_cost_usd_max,
                        estimated_latency_ms: cand.est_latency_ms,
                        admissible: true,
                        rejection_reasons: None,
                    });
                }
                Err(reasons) => {
                    let reason_strings = reasons.iter().map(|r| r.to_string()).collect();
                    candidate_explanations.push(CandidateExplanation {
                        topology: cand.topology.clone(),
                        harness_id: cand.harness_id.clone(),
                        estimated_tokens: cand.est_tokens,
                        estimated_cost_usd_min: cand.est_cost_usd_min,
                        estimated_cost_usd_max: cand.est_cost_usd_max,
                        estimated_latency_ms: cand.est_latency_ms,
                        admissible: false,
                        rejection_reasons: Some(reason_strings),
                    });
                }
            }
        }

        let chosen = Selector::select(candidates, snapshot)?;
        let revision = PlanCompiler::compile(&chosen, 1);
        let preview_nodes = revision.nodes.into_iter().map(|n| n.step_name).collect();

        let cost_range_tokens = if min_tokens == u64::MAX {
            (0, 0)
        } else {
            (min_tokens, max_tokens)
        };

        let cost_range_usd = if min_usd == f32::MAX {
            (0.0, 0.0)
        } else {
            (min_usd, max_usd)
        };

        Ok(ExplainReport {
            task_id: snapshot.task_id.clone(),
            chosen_topology: chosen.chosen_topology,
            chosen_harness_id: chosen.candidate_harness,
            estimated_tokens: chosen.estimated_tokens,
            estimated_cost_usd: chosen.estimated_cost_usd,
            reasoning: chosen.reasoning,
            cost_range_tokens,
            cost_range_usd,
            candidates: candidate_explanations,
            compiled_nodes_preview: preview_nodes,
            obligations: revision.obligations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_service_generates_report() {
        let mut snapshot = DecisionSnapshot::new("task-explain-001");
        snapshot.remaining_budget_tokens = 50_000;

        let report = ExplainService::explain(&snapshot).expect("explain report");

        assert_eq!(report.task_id, "task-explain-001");
        assert!(!report.candidates.is_empty());
        assert!(report.cost_range_tokens.0 <= report.cost_range_tokens.1);
        assert!(!report.compiled_nodes_preview.is_empty());
        assert!(report.candidates.iter().any(|c| c.admissible));
    }
}
