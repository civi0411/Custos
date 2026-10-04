use custos_domain::oi::{Candidate, DecisionSnapshot, StrategyProposal};
use custos_core::oi::hard_filters::HardFilters;
use custos_domain::DomainError;

pub struct Selector;

impl Selector {
    pub fn select(candidates: Vec<Candidate>, snapshot: &DecisionSnapshot) -> Result<StrategyProposal, DomainError> {
        let mut alternatives = Vec::new();
        let mut best: Option<Candidate> = None;

        for cand in candidates {
            let temp_proposal = StrategyProposal {
                id: custos_domain::new_id("prop"),
                task_id: snapshot.task_id.clone(),
                chosen_topology: cand.topology.clone(),
                candidate_harness: cand.harness_id.clone(),
                reasoning: "".into(),
                estimated_cost_usd: cand.est_cost_usd_max,
                estimated_tokens: cand.est_tokens,
                alternatives_considered: vec![],
                assumptions: vec![],
                context_strategy: Some("default_windowed".into()),
            };

            match HardFilters::evaluate(&temp_proposal, snapshot) {
                Ok(_) => {
                    if best.is_none() {
                        best = Some(cand.clone());
                    } else {
                        alternatives.push(format!("{} (admitted, but deferred)", cand.harness_id));
                    }
                }
                Err(reasons) => {
                    let reason_str = reasons.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ");
                    alternatives.push(format!("{} (abstained: {})", cand.harness_id, reason_str));
                }
            }
        }

        if let Some(cand) = best {
            Ok(StrategyProposal {
                id: custos_domain::new_id("prop"),
                task_id: snapshot.task_id.clone(),
                chosen_topology: cand.topology,
                candidate_harness: cand.harness_id,
                reasoning: "Selected based on admissibility and internal heuristics".into(),
                estimated_cost_usd: cand.est_cost_usd_max,
                estimated_tokens: cand.est_tokens,
                alternatives_considered: alternatives,
                assumptions: vec![],
                context_strategy: Some("default_windowed".into()),
            })
        } else {
            Err(DomainError::Validation("No admissible candidates found".into()))
        }
    }
}
