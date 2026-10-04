use custos_domain::oi::Candidate;

pub struct Estimator;

impl Estimator {
    pub fn estimate(candidates: &mut [Candidate]) {
        for cand in candidates.iter_mut() {
            match cand.harness_id.as_str() {
                "claude-code" => {
                    cand.est_cost_usd_min = 0.02;
                    cand.est_cost_usd_max = 0.15;
                    cand.est_tokens = 5000;
                    cand.est_latency_ms = 8000;
                }
                "goose" => {
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
    }
}
