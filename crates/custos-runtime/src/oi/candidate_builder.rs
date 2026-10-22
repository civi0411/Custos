use custos_domain::oi::{Candidate, DecisionSnapshot, ExecutionTopology};

pub struct CandidateBuilder;

impl CandidateBuilder {
    pub fn build(snapshot: &DecisionSnapshot) -> Vec<Candidate> {
        let mut candidates = vec![
            Candidate {
                topology: ExecutionTopology::NativeBaseline,
                harness_id: "claude-code".into(),
                est_cost_usd_min: 0.0,
                est_cost_usd_max: 0.0,
                est_tokens: 0,
                est_latency_ms: 0,
                risks: vec![],
            },
            Candidate {
                topology: ExecutionTopology::NativeBaseline,
                harness_id: "custos-agent".into(),
                est_cost_usd_min: 0.0,
                est_cost_usd_max: 0.0,
                est_tokens: 0,
                est_latency_ms: 0,
                risks: vec![],
            },
            Candidate {
                topology: ExecutionTopology::DirectModel,
                harness_id: "claude-3-5-sonnet".into(),
                est_cost_usd_min: 0.0,
                est_cost_usd_max: 0.0,
                est_tokens: 0,
                est_latency_ms: 0,
                risks: vec![],
            },
            Candidate {
                topology: ExecutionTopology::DirectModel,
                harness_id: "custos-local".into(),
                est_cost_usd_min: 0.0,
                est_cost_usd_max: 0.0,
                est_tokens: 0,
                est_latency_ms: 0,
                risks: vec![],
            },
        ];

        if let Some(pin) = &snapshot.pinned_model {
            if !candidates.iter().any(|c| &c.harness_id == pin) {
                candidates.push(Candidate {
                    topology: ExecutionTopology::DirectModel,
                    harness_id: pin.clone(),
                    est_cost_usd_min: 0.0,
                    est_cost_usd_max: 0.0,
                    est_tokens: 0,
                    est_latency_ms: 0,
                    risks: vec![],
                });
            }
        }

        candidates
    }
}
