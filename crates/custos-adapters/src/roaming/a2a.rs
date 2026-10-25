//! System One A2A (Agent-to-Agent) Fast Delegation Router
//!
//! Discovers eligible peers via Roaming Directory and verifies trust via TrustBook
//! before dispatching delegation requests to peer agents.

use crate::roaming::directory::{Directory, PeerEntry};
use crate::roaming::error::RoamingError;
use crate::roaming::frame::{read_frame, write_frame};
use crate::roaming::node::RoamingNode;
use crate::roaming::trust::TrustBook;
use iroh::{EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ACapability {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2ADelegationRequest {
    pub delegation_id: String,
    pub task: String,
    pub required_capabilities: Vec<String>,
    pub budget_limit_usd: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum A2ADelegationStatus {
    Delegated,
    Completed,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2ADelegationResult {
    pub delegation_id: String,
    pub peer_endpoint_id: String,
    pub status: A2ADelegationStatus,
    pub response: Option<String>,
}

pub struct SystemOneA2ARouter {
    directory: Directory,
    trust_book: Arc<RwLock<TrustBook>>,
    node: Option<Arc<RoamingNode>>,
}

impl SystemOneA2ARouter {
    pub fn new(directory: Directory, trust_book: Arc<RwLock<TrustBook>>) -> Self {
        Self {
            directory,
            trust_book,
            node: None,
        }
    }

    /// Wire an active Iroh RoamingNode to enable live QUIC peer delegation.
    pub fn with_node(mut self, node: Arc<RoamingNode>) -> Self {
        self.node = Some(node);
        self
    }

    /// List active, trusted peer candidates
    pub async fn list_trusted_peers(&self) -> Result<Vec<PeerEntry>, RoamingError> {
        let entries = self.directory.list().await;
        let trust = self.trust_book.read().await;

        let mut trusted = Vec::new();
        for entry in entries {
            if let Ok(endpoint_id) = entry.endpoint_id.parse() {
                if trust.is_allowed(&endpoint_id) {
                    trusted.push(entry);
                }
            }
        }
        Ok(trusted)
    }

    /// Delegate a task to an available trusted peer
    pub async fn delegate(
        &self,
        req: A2ADelegationRequest,
    ) -> Result<A2ADelegationResult, RoamingError> {
        let eligible = self.list_trusted_peers().await?;

        // Find connected peer first
        let chosen_peer = eligible
            .iter()
            .find(|p| p.connected)
            .or_else(|| eligible.first());

        let peer = match chosen_peer {
            Some(p) => p,
            None => {
                return Err(RoamingError::Identity(
                    "No trusted roaming peer available for delegation".into(),
                ))
            }
        };

        // If a live Iroh node is wired, establish a QUIC bi-stream to the peer
        if let Some(ref node) = self.node {
            if let Ok(endpoint_id) = peer.endpoint_id.parse::<EndpointId>() {
                let addr = EndpointAddr::from(endpoint_id);
                let mut stream = node.connect_with_addr(addr, Some(req.task.clone())).await?;

                let payload = serde_json::to_vec(&req)
                    .map_err(|e| RoamingError::Transport(format!("serialize delegation request: {e}")))?;
                write_frame(&mut stream.send, &payload).await?;

                let resp_bytes = read_frame(&mut stream.recv).await?;
                let remote_res: A2ADelegationResult = serde_json::from_slice(&resp_bytes)
                    .unwrap_or_else(|_| A2ADelegationResult {
                        delegation_id: req.delegation_id.clone(),
                        peer_endpoint_id: peer.endpoint_id.clone(),
                        status: A2ADelegationStatus::Completed,
                        response: Some(String::from_utf8_lossy(&resp_bytes).to_string()),
                    });
                return Ok(remote_res);
            }
        }

        // Local or offline test dispatch
        Ok(A2ADelegationResult {
            delegation_id: req.delegation_id,
            peer_endpoint_id: peer.endpoint_id.clone(),
            status: A2ADelegationStatus::Delegated,
            response: Some(format!(
                "Delegated task '{}' to peer {}",
                req.task, peer.endpoint_id
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roaming::directory::Direction;
    use iroh::SecretKey;

    #[tokio::test]
    async fn test_system_one_a2a_delegation_flow() {
        let directory = Directory::new();
        let trust_book = Arc::new(RwLock::new(TrustBook::new()));

        // Create two identities
        let key_a = SecretKey::generate();
        let key_b = SecretKey::generate();

        let endpoint_a = key_a.public();
        let endpoint_b = key_b.public();

        // Record both in directory
        directory
            .record_connect(
                endpoint_a,
                Some("agent_a".into()),
                Direction::Outbound,
                None,
                1000,
            )
            .await;
        directory
            .record_connect(
                endpoint_b,
                Some("agent_b".into()),
                Direction::Inbound,
                None,
                1000,
            )
            .await;

        // Only accept endpoint_b in trust book
        trust_book.write().await.accept(&endpoint_b);

        let router = SystemOneA2ARouter::new(directory, trust_book);
        let trusted = router.list_trusted_peers().await.unwrap();

        assert_eq!(trusted.len(), 1);
        assert_eq!(trusted[0].endpoint_id, endpoint_b.to_string());

        let req = A2ADelegationRequest {
            delegation_id: "del_123".into(),
            task: "Verify cryptographic signature".into(),
            required_capabilities: vec!["crypto".into()],
            budget_limit_usd: Some(0.05),
        };

        let result = router.delegate(req).await.unwrap();
        assert_eq!(result.delegation_id, "del_123");
        assert_eq!(result.peer_endpoint_id, endpoint_b.to_string());
        assert_eq!(result.status, A2ADelegationStatus::Delegated);
    }
}
