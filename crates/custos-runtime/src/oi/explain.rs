//! OI Explain Service (RFC 003 §4, RFC 004 §2)
//!
//! Provides transparent, inspectable explanations of orchestration decisions:
//! candidate evaluations, rejection rationales, cost ranges, and compiled DAG preview.

use custos_core::oi::compiler::PlanCompiler;
use custos_core::oi::hard_filters::HardFilters;
pub use custos_domain::oi::{CandidateExplanation, ExplainReport};
use custos_domain::oi::{DecisionSnapshot, StrategyProposal};
use custos_domain::DomainError;

use super::candidate_builder::CandidateBuilder;
use super::estimator::Estimator;
use super::selector::Selector;

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
                chosen_topology: cand.topology,
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
                        topology: cand.topology,
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
                        topology: cand.topology,
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

        let unknown_tokens = if snapshot.unknowns.is_empty() {
            0
        } else {
            (snapshot.unknowns.len() as u64) * 500
        };

        Ok(ExplainReport {
            task_id: snapshot.task_id.clone(),
            chosen_topology: chosen.chosen_topology,
            chosen_harness_id: chosen.candidate_harness,
            estimated_tokens: chosen.estimated_tokens,
            billed_tokens: 0,
            unknown_tokens,
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
        snapshot.unknowns = vec!["Unverified remote checkpoint".into()];

        let report = ExplainService::explain(&snapshot).expect("explain report");

        assert_eq!(report.task_id, "task-explain-001");
        assert!(!report.candidates.is_empty());
        assert!(report.cost_range_tokens.0 <= report.cost_range_tokens.1);
        assert!(!report.compiled_nodes_preview.is_empty());
        assert!(report.candidates.iter().any(|c| c.admissible));
        assert_eq!(report.billed_tokens, 0);
        assert_eq!(report.unknown_tokens, 500);
    }

    #[test]
    fn test_explain_service_local_only_and_truthful_spend() {
        let mut snapshot = DecisionSnapshot::new("task-explain-local");
        snapshot.required_capabilities = vec!["LocalOnly".into()];

        let report = ExplainService::explain(&snapshot).expect("explain report");
        assert_eq!(report.chosen_harness_id, "custos-local");
        assert_eq!(report.estimated_cost_usd, 0.0);
        let cloud_candidate = report.candidates.iter().find(|c| c.harness_id == "claude-code").unwrap();
        assert!(!cloud_candidate.admissible);
        assert!(cloud_candidate.rejection_reasons.as_ref().unwrap().iter().any(|r| r.contains("Local-only")));
    }
}
