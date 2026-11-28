//! Persistent Agent Persona Profile ("Digital Colleague") sitting above `OiEngine`.

use serde::{Deserialize, Serialize};

use crate::authority::action::RiskLevel;
use crate::conversation::session::WorkbenchId;

/// Strongly-typed memory isolation namespace for a persona.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryScopeId(pub String);

impl MemoryScopeId {
    pub fn new(scope: impl Into<String>) -> Self {
        Self(scope.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verified caller identity passed into `WhoCanAskPolicy` and `ActsForTrustPolicy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerIdentity {
    pub sender_id: String,
    pub scope: String,
    pub is_paired_owner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "senders", rename_all = "snake_case")]
pub enum WhoCanAskPolicy {
    OwnerOnly,
    AllowList(Vec<String>),
    AnyChannel,
}

impl WhoCanAskPolicy {
    pub fn allows_caller(&self, caller: &CallerIdentity) -> bool {
        match self {
            WhoCanAskPolicy::OwnerOnly => caller.is_paired_owner,
            WhoCanAskPolicy::AllowList(allowed) => {
                caller.is_paired_owner || allowed.iter().any(|s| s == &caller.sender_id)
            }
            WhoCanAskPolicy::AnyChannel => true,
        }
    }
}

/// Outbound delegation policy governing what a persona may do on the user's behalf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActsForTrustPolicy {
    pub allowed_workbenches: Vec<WorkbenchId>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
    pub max_autonomous_risk: RiskLevel,
    #[serde(default = "default_true")]
    pub require_biometric_for_mutations: bool,
    #[serde(default = "default_true")]
    pub allow_mobile_dispatch: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ActsForTrustPolicy {
    fn default() -> Self {
        Self {
            allowed_workbenches: vec![WorkbenchId::new(WorkbenchId::ASSISTANT)],
            allowed_capabilities: vec!["fs_read".to_string()],
            max_autonomous_risk: RiskLevel::Low,
            require_biometric_for_mutations: true,
            allow_mobile_dispatch: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonaProfile {
    pub id: String,
    pub name: String,
    pub role: String,
    pub default_workbench: WorkbenchId,
    pub memory_scope: MemoryScopeId,
    pub allowed_triggers: WhoCanAskPolicy,
    #[serde(default)]
    pub acts_for: ActsForTrustPolicy,
    #[serde(default)]
    pub sovereign_instructions: String,
    pub look_around_job_id: Option<String>,
    #[serde(default = "default_max_up_next")]
    pub max_up_next_items: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_max_up_next() -> u8 {
    5
}
