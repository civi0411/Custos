use super::profile::EnvironmentProfile;
use crate::authority::action::Assurance;
use serde::{Deserialize, Serialize};

/// Immutable specification for provisioning an Agent Execution Environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentSpec {
    pub env_id: String,
    pub profile: EnvironmentProfile,
    pub target_assurance: Assurance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_network_denied: Option<bool>,
}

impl EnvironmentSpec {
    pub fn new(env_id: impl Into<String>, profile: EnvironmentProfile) -> Self {
        Self {
            env_id: env_id.into(),
            profile,
            target_assurance: Assurance::CustosMediated,
            owner_task_id: None,
            sandbox_network_denied: None,
        }
    }
}

