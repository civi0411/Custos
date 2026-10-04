//! Event-driven Replanning models and ReplanBrief (RFC 003 §2B / RFC 004 §2D)
//!
//! When an execution assumption fails or unexpected outcomes occur,
//! an immutable ReplanBrief is generated without blind retrying.

use crate::ids::new_id;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assumption {
    pub id: String,
    pub description: String,
    pub is_critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplanTrigger {
    /// Verification test failed
    TestFailure,
    /// Required evidence artifact missing
    EvidenceMissing,
    /// Code or workspace drift detected
    SourceDrift,
    /// Budget slice exhausted before completion
    BudgetDepleted,
    /// Worker explicitly abstained
    WorkerAbstain,
    /// Timeout reached
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanBrief {
    pub id: String,
    pub task_id: String,
    pub trigger: ReplanTrigger,
    pub failed_node_id: Option<String>,
    pub reason: String,
    pub affected_node_ids: Vec<String>,
    pub preserved_node_ids: Vec<String>,
}

impl ReplanBrief {
    pub fn new(
        task_id: impl Into<String>,
        trigger: ReplanTrigger,
        failed_node_id: Option<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: new_id("rbrf"),
            task_id: task_id.into(),
            trigger,
            failed_node_id,
            reason: reason.into(),
            affected_node_ids: Vec::new(),
            preserved_node_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanRecord {
    pub id: String,
    pub task_id: String,
    pub created_at: DateTime<Utc>,
    pub brief: ReplanBrief,
    pub new_proposal_id: String,
}

impl ReplanRecord {
    pub fn new(
        task_id: impl Into<String>,
        brief: ReplanBrief,
        new_proposal_id: impl Into<String>,
    ) -> Self {
        Self {
            id: new_id("rrec"),
            task_id: task_id.into(),
            created_at: Utc::now(),
            brief,
            new_proposal_id: new_proposal_id.into(),
        }
    }
}
