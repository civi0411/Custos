use async_trait::async_trait;
use custos_contracts::contracts::OiPlannerPort;
use custos_contracts::oi::{DecisionSnapshot, StrategyProposal};
use custos_contracts::DomainError;

use super::candidate_builder::{CandidateBuilder, CandidateFactory};
use super::estimator::Estimator;
use super::selector::Selector;

pub struct OiEngine {
    factories: Vec<Box<dyn CandidateFactory>>,
}

impl OiEngine {
    pub fn new() -> Self {
        Self {
            factories: CandidateBuilder::default_factories(),
        }
    }

    pub fn with_factories(factories: Vec<Box<dyn CandidateFactory>>) -> Self {
        Self { factories }
    }
}

impl Default for OiEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OiPlannerPort for OiEngine {
    async fn plan(&self, snapshot: &DecisionSnapshot) -> Result<StrategyProposal, DomainError> {
        let mut candidates = CandidateBuilder::build_with_factories(snapshot, &self.factories);
        Estimator::estimate_with_snapshot(&mut candidates, Some(snapshot));

        let proposal = Selector::select(candidates, snapshot)?;
        Ok(proposal)
    }
}
