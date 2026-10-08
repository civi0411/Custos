//! Remote SSH Fleet Domain Models
//!
//! Models for SSH host nodes, fleet clusters, remote command execution,
//! and verification receipts.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteHostStatus {
    Online,
    Offline,
    Degraded,
    AuthFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SshAuthMethod {
    KeyPair {
        private_key_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,
    },
    Agent,
    PasswordPrompt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteHostNode {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth_method: SshAuthMethod,
    pub status: RemoteHostStatus,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    pub last_ping_ms: Option<u64>,
    pub os_info: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl RemoteHostNode {
    pub fn new(
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        auth_method: SshAuthMethod,
    ) -> Result<Self, DomainError> {
        let name_str = name.into();
        let host_str = host.into();
        let user_str = user.into();

        if host_str.trim().is_empty() {
            return Err(DomainError::Validation("Host address cannot be empty.".to_string()));
        }
        if user_str.trim().is_empty() {
            return Err(DomainError::Validation("SSH user cannot be empty.".to_string()));
        }

        let now = chrono::Utc::now().timestamp_millis();
        Ok(Self {
            id: new_id("node"),
            name: name_str,
            host: host_str,
            port: if port == 0 { 22 } else { port },
            user: user_str,
            auth_method,
            status: RemoteHostStatus::Offline,
            labels: HashMap::new(),
            last_ping_ms: None,
            os_info: None,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterHostParams {
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_ssh_port() -> u16 {
    22
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetExecParams {
    pub host_ids: Vec<String>,
    pub command: String,
    pub cwd: Option<String>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetExecReceipt {
    pub execution_id: String,
    pub host_id: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub verified: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetExecResult {
    pub receipts: Vec<FleetExecReceipt>,
    pub all_succeeded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_host_node_creation() {
        let node = RemoteHostNode::new(
            "gpu-worker-1",
            "10.0.0.45",
            22,
            "ubuntu",
            SshAuthMethod::KeyPair {
                private_key_path: "~/.ssh/id_ed25519".to_string(),
                passphrase: None,
            },
        ).unwrap();

        assert!(node.id.starts_with("node_"));
        assert_eq!(node.port, 22);
        assert_eq!(node.status, RemoteHostStatus::Offline);
    }
}
