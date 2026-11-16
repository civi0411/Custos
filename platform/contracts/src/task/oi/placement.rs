//! Node Placement mapping within compiled workflow revisions (RFC 004 §2C)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePlacement {
    pub node_id: String,
    pub role: String,
    pub backend_harness: String,
    pub budget_tokens_slice: u64,
    pub workspace_lease_id: Option<String>,
}

impl NodePlacement {
    pub fn new(
        node_id: impl Into<String>,
        role: impl Into<String>,
        backend_harness: impl Into<String>,
        budget_tokens_slice: u64,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            role: role.into(),
            backend_harness: backend_harness.into(),
            budget_tokens_slice,
            workspace_lease_id: None,
        }
    }
}
