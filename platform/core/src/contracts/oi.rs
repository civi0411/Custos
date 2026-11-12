//! Port for Orchestration Intelligence (OI) Planner (RFC 003 / RFC 004)
//!
//! Core requests strategy proposals through this trait.
//! Implemented by `custos-runtime::oi::OiEngine`.

use async_trait::async_trait;
use custos_domain::oi::{DecisionSnapshot, StrategyProposal};
use custos_domain::DomainError;

#[async_trait]
pub trait OiPlannerPort: Send + Sync {
    /// Ingests an immutable reality snapshot and proposes an admissible strategy.
    async fn plan(&self, snapshot: &DecisionSnapshot) -> Result<StrategyProposal, DomainError>;
}
