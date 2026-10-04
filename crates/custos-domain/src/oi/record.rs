//! Durable Decision Record appended to the ledger (RFC 003 §2D)

use super::proposal::StrategyProposal;
use super::snapshot::DecisionSnapshot;
use crate::ids::new_id;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub task_id: String,
    pub created_at: DateTime<Utc>,
    pub snapshot: DecisionSnapshot,
    pub proposal: StrategyProposal,
    pub overhead_ms: u64,
}

impl DecisionRecord {
    pub fn new(snapshot: DecisionSnapshot, proposal: StrategyProposal, overhead_ms: u64) -> Self {
        Self {
            id: new_id("drec"),
            task_id: snapshot.task_id.clone(),
            created_at: Utc::now(),
            snapshot,
            proposal,
            overhead_ms,
        }
    }
}
