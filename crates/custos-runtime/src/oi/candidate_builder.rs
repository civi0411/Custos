use custos_domain::oi::{Candidate, DecisionSnapshot, ExecutionTopology};

pub struct CandidateBuilder;

impl CandidateBuilder {
    pub fn build(_snapshot: &DecisionSnapshot) -> Vec<Candidate> {
        vec![
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
                harness_id: "goose".into(),
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
        ]
    }
}
