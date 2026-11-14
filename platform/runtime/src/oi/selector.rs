//! Pareto Multi-Objective Selector for OI Engine (RFC 004 §3)
//!
//! Scores each admissible candidate with a utility function:
//!   U = w_q * Q  -  w_c * C  -  w_t * T
//! where Q = quality proxy (1 / latency_ms, normalised), C = cost_usd_max (normalised),
//! T = latency_ms (normalised).  Weights are configurable; defaults: quality 0.5, cost 0.3,
//! latency 0.2.  The candidate with the highest U wins; ties broken by lower cost.
//!
//! Candidates that fail HardFilters are excluded from scoring and recorded in
//! `alternatives_considered` as abstained with reasons.

use super::hard_filters::HardFilters;
use custos_domain::oi::{Candidate, DecisionSnapshot, StrategyProposal};
use custos_domain::DomainError;

/// Pareto utility weights — sum should equal 1.0 (not enforced; normalisation is relative).
struct Weights {
    quality: f64,
    cost: f64,
    latency: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self { quality: 0.5, cost: 0.3, latency: 0.2 }
    }
}

pub struct Selector;

impl Selector {
    pub fn select(
        candidates: Vec<Candidate>,
        snapshot: &DecisionSnapshot,
    ) -> Result<StrategyProposal, DomainError> {
        Self::select_with_weights(candidates, snapshot, &Weights::default())
    }

    fn select_with_weights(
        candidates: Vec<Candidate>,
        snapshot: &DecisionSnapshot,
        w: &Weights,
    ) -> Result<StrategyProposal, DomainError> {
        if candidates.is_empty() {
            return Err(DomainError::Validation("No candidates provided to selector".into()));
        }

        // Separate admissible from rejected, collecting rejection reasons
        let mut admissible: Vec<(Candidate, StrategyProposal)> = Vec::new();
        let mut alternatives: Vec<String> = Vec::new();

        for cand in candidates {
            let proposal = Self::candidate_to_proposal(&cand, snapshot, vec![]);
            match HardFilters::evaluate(&proposal, snapshot) {
                Ok(_) => admissible.push((cand, proposal)),
                Err(reasons) => {
                    let reason_str = reasons
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    alternatives.push(format!("{} (rejected: {reason_str})", cand.harness_id));
                }
            }
        }

        if admissible.is_empty() {
            return Err(DomainError::Validation(
                "No admissible candidates — all rejected by hard filters".into(),
            ));
        }

        // Normalise cost and latency across admissible set for fair comparison
        let max_cost = admissible
            .iter()
            .map(|(c, _)| c.est_cost_usd_max as f64)
            .fold(f64::NEG_INFINITY, f64::max)
            .max(f64::EPSILON); // avoid div-by-zero for all-local (cost = 0)

        let max_latency = admissible
            .iter()
            .map(|(c, _)| c.est_latency_ms as f64)
            .fold(f64::NEG_INFINITY, f64::max)
            .max(f64::EPSILON);

        // Utility: U = w_q * Q - w_c * C_norm - w_t * T_norm
        // Q is approximated by (1 - latency_norm), i.e., faster = higher quality proxy.
        // For agents with known quality signals (e.g. benchmark scores), extend here.
        let score = |(c, _): &(Candidate, StrategyProposal)| -> f64 {
            let cost_norm = c.est_cost_usd_max as f64 / max_cost;
            let latency_norm = c.est_latency_ms as f64 / max_latency;
            let quality_proxy = 1.0 - latency_norm;
            w.quality * quality_proxy - w.cost * cost_norm - w.latency * latency_norm
        };

        // Pick highest utility; on exact tie prefer lower cost
        let best_idx = admissible
            .iter()
            .enumerate()
            .max_by(|(ai, a), (bi, b)| {
                let sa = score(a);
                let sb = score(b);
                sa.partial_cmp(&sb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        // tie-break: lower cost wins (reverse comparison)
                        b.0.est_cost_usd_max
                            .partial_cmp(&a.0.est_cost_usd_max)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then(ai.cmp(bi).reverse()) // stable: prefer earlier index on full tie
            })
            .map(|(i, _)| i)
            .unwrap(); // safe: admissible is non-empty

        let (best_cand, _) = admissible.remove(best_idx);

        // Remaining admissible candidates go to alternatives as deferred
        for (cand, _) in admissible {
            alternatives.push(format!("{} (admitted, deferred by Pareto score)", cand.harness_id));
        }

        let reasoning = format!(
            "Pareto selection: U=w_q*{:.1}-w_c*{:.1}-w_t*{:.1}; \
             chosen harness='{}' topology={:?} cost_max=${:.4} latency={}ms",
            w.quality,
            w.cost,
            w.latency,
            best_cand.harness_id,
            best_cand.topology,
            best_cand.est_cost_usd_max,
            best_cand.est_latency_ms,
        );

        Ok(Self::candidate_to_proposal(&best_cand, snapshot, alternatives)
            .with_reasoning(reasoning))
    }

    fn candidate_to_proposal(
        cand: &Candidate,
        snapshot: &DecisionSnapshot,
        alternatives: Vec<String>,
    ) -> StrategyProposal {
        StrategyProposal {
            id: custos_domain::new_id("prop"),
            task_id: snapshot.task_id.clone(),
            chosen_topology: cand.topology,
            candidate_harness: cand.harness_id.clone(),
            reasoning: String::new(),
            estimated_cost_usd: cand.est_cost_usd_max,
            estimated_tokens: cand.est_tokens,
            alternatives_considered: alternatives,
            assumptions: vec![],
            context_strategy: Some(context_strategy_for(snapshot)),
        }
    }
}

/// Chooses a context windowing strategy based on snapshot complexity.
fn context_strategy_for(snap: &DecisionSnapshot) -> String {
    let complexity = snap.unknowns.len()
        + snap.uncertain_effects_count
        + snap.pending_effects_count;
    if complexity >= 5 {
        "cache_anchored".into()      // prefer prefix-cached context for complex tasks
    } else {
        "default_windowed".into()
    }
}

// Builder extension on StrategyProposal to set reasoning after construction
trait WithReasoning: Sized {
    fn with_reasoning(self, r: String) -> Self;
}
impl WithReasoning for StrategyProposal {
    fn with_reasoning(mut self, r: String) -> Self {
        self.reasoning = r;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::oi::{Candidate, DecisionSnapshot, ExecutionTopology};

    fn snap() -> DecisionSnapshot {
        DecisionSnapshot {
            task_id: "t1".into(),
            unknowns: vec![],
            uncertain_effects_count: 0,
            pending_effects_count: 0,
            criteria: vec![],
            ..Default::default()
        }
    }

    fn cand(id: &str, cost: f32, latency: u64) -> Candidate {
        Candidate {
            topology: ExecutionTopology::T1SingleWorker,
            harness_id: id.into(),
            est_cost_usd_min: cost * 0.5,
            est_cost_usd_max: cost,
            est_tokens: 1000,
            est_latency_ms: latency,
            risks: vec![],
        }
    }

    #[test]
    fn prefers_lower_cost_on_equal_latency() {
        let candidates = vec![cand("expensive", 0.10, 1000), cand("cheap", 0.01, 1000)];
        let proposal = Selector::select(candidates, &snap()).unwrap();
        assert_eq!(proposal.candidate_harness, "cheap");
    }

    #[test]
    fn pareto_reasoning_in_output() {
        let candidates = vec![cand("a", 0.05, 2000), cand("b", 0.02, 3000)];
        let proposal = Selector::select(candidates, &snap()).unwrap();
        assert!(proposal.reasoning.contains("Pareto"));
    }

    #[test]
    fn all_rejected_returns_error() {
        // CandidateBuilder normally enforces non-empty; here we test selector directly with no candidates
        let result = Selector::select(vec![], &snap());
        assert!(result.is_err());
    }
}
