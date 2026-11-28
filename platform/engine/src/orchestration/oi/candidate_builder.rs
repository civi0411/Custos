use custos_contracts::oi::{Candidate, DecisionSnapshot, ExecutionTopology};

/// Open/Closed factory interface for contributing OI strategy candidates.
pub trait CandidateFactory: Send + Sync {
    fn generate(&self, snapshot: &DecisionSnapshot) -> Vec<Candidate>;
}

/// Default baseline factory for `NativeBaseline` and `DirectModel`.
pub struct BaselineCandidateFactory;

impl CandidateFactory for BaselineCandidateFactory {
    fn generate(&self, snapshot: &DecisionSnapshot) -> Vec<Candidate> {
        let mut candidates = vec![
            candidate(ExecutionTopology::NativeBaseline, "claude-code"),
            candidate(ExecutionTopology::NativeBaseline, "custos-agent"),
            candidate(ExecutionTopology::DirectModel, "claude-3-5-sonnet"),
            candidate(ExecutionTopology::DirectModel, "custos-local"),
        ];
        if let Some(pin) = &snapshot.pinned_model {
            if !candidates.iter().any(|c| &c.harness_id == pin) {
                candidates.push(candidate(ExecutionTopology::DirectModel, pin));
            }
        }
        candidates
    }
}

/// Topology factory contributing `T0Direct`, `T3ReadFanOut`, `T4Worktree`, and `T5LeadWorker`.
pub struct TopologyCandidateFactory;

impl CandidateFactory for TopologyCandidateFactory {
    fn generate(&self, snapshot: &DecisionSnapshot) -> Vec<Candidate> {
        let primary = snapshot
            .available_executors
            .first()
            .cloned()
            .unwrap_or_else(|| "custos-agent".to_string());
        let mut out = vec![candidate(ExecutionTopology::T0Direct, &primary)];
        let is_read_only = snapshot
            .required_capabilities
            .iter()
            .all(|c| c == "fs_read" || c == "read_only");
        if is_read_only {
            out.push(candidate(ExecutionTopology::T3ReadFanOut, &primary));
        } else {
            out.push(candidate(ExecutionTopology::T4Worktree, &primary));
            out.push(candidate(ExecutionTopology::T5LeadWorker, &primary));
        }
        out
    }
}

pub struct CandidateBuilder;

impl CandidateBuilder {
    pub fn default_factories() -> Vec<Box<dyn CandidateFactory>> {
        vec![
            Box::new(BaselineCandidateFactory),
            Box::new(TopologyCandidateFactory),
        ]
    }

    pub fn build(snapshot: &DecisionSnapshot) -> Vec<Candidate> {
        let factories = Self::default_factories();
        Self::build_with_factories(snapshot, &factories)
    }

    pub fn build_with_factories(
        snapshot: &DecisionSnapshot,
        factories: &[Box<dyn CandidateFactory>],
    ) -> Vec<Candidate> {
        let mut candidates: Vec<Candidate> = factories
            .iter()
            .flat_map(|f| f.generate(snapshot))
            .collect();

        if !snapshot.available_executors.is_empty() {
            candidates.retain(|c| snapshot.available_executors.contains(&c.harness_id));
        }

        candidates
    }
}

fn candidate(topology: ExecutionTopology, harness_id: impl Into<String>) -> Candidate {
    Candidate {
        topology,
        harness_id: harness_id.into(),
        est_cost_usd_min: 0.0,
        est_cost_usd_max: 0.0,
        est_tokens: 0,
        est_latency_ms: 0,
        risks: vec![],
    }
}
