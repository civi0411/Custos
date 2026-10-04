use async_trait::async_trait;
use custos_core::contracts::OiPlannerPort;
use custos_domain::oi::{DecisionSnapshot, StrategyProposal};
use custos_domain::DomainError;

use super::candidate_builder::CandidateBuilder;
use super::estimator::Estimator;
use super::selector::Selector;

pub struct OiEngine;

impl OiEngine {
    pub fn new() -> Self {
        Self
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
        let mut candidates = CandidateBuilder::build(snapshot);
        Estimator::estimate(&mut candidates);
        
        let proposal = Selector::select(candidates, snapshot)?;
        Ok(proposal)
    }
}
