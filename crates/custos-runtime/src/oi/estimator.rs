use custos_domain::oi::{Candidate, DecisionSnapshot};

pub struct Estimator;

impl Estimator {
    /// Legacy estimation with default baseline values.
    pub fn estimate(candidates: &mut [Candidate]) {
        Self::estimate_with_snapshot(candidates, None);
    }

    /// Full estimation taking into account snapshot context, unknowns, verification, and recovery buffers.
    pub fn estimate_with_snapshot(
        candidates: &mut [Candidate],
        snapshot: Option<&DecisionSnapshot>,
    ) {
        for cand in candidates.iter_mut() {
            let is_local = cand.harness_id.contains("local")
                || cand.harness_id.contains("mlx")
                || cand.harness_id.contains("llama");

            if is_local {
                cand.est_cost_usd_min = 0.0;
                cand.est_cost_usd_max = 0.0;
                cand.est_tokens = 2000;
                cand.est_latency_ms = 4000;
            } else {
                match cand.harness_id.as_str() {
                    "claude-code" => {
                        cand.est_cost_usd_min = 0.02;
                        cand.est_cost_usd_max = 0.15;
                        cand.est_tokens = 5000;
                        cand.est_latency_ms = 8000;
                    }
                    "goose" | "custos-agent" => {
                        cand.est_cost_usd_min = 0.01;
                        cand.est_cost_usd_max = 0.10;
                        cand.est_tokens = 4000;
                        cand.est_latency_ms = 7000;
                    }
                    _ => {
                        cand.est_cost_usd_min = 0.01;
                        cand.est_cost_usd_max = 0.05;
                        cand.est_tokens = 2000;
                        cand.est_latency_ms = 3000;
                    }
                }
            }

            if let Some(snap) = snapshot {
                // 1. Explicit uncertainty representation (unknowns are never 0 tokens/latency)
                let unknown_count = snap.unknowns.len() as u64;
                if unknown_count > 0 {
                    cand.est_tokens += unknown_count * 500;
                    cand.est_latency_ms += unknown_count * 1000;
                    if !is_local {
                        cand.est_cost_usd_max += (unknown_count as f32) * 0.02;
                    }
                }

                // 2. Recovery / retry buffer from pending or uncertain side-effects
                let uncertain_effects =
                    (snap.uncertain_effects_count + snap.pending_effects_count) as u64;
                if uncertain_effects > 0 {
                    cand.est_tokens += uncertain_effects * 800;
                    cand.est_latency_ms += uncertain_effects * 2500;
                    if !is_local {
                        cand.est_cost_usd_max += (uncertain_effects as f32) * 0.03;
                    }
                }

                // 3. Criteria & verification demands
                let criteria_count = snap.criteria.len() as u64;
                if criteria_count > 0 {
                    cand.est_tokens += criteria_count * 300;
                    cand.est_latency_ms += criteria_count * 600;
                }
            }
        }
    }
}
