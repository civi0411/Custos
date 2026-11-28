//! Persistent Agent Persona Profile ("Digital Colleague") sitting above `OiEngine`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaWorkbench {
    Coding,
    Research,
    Assistant,
    Quant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "senders", rename_all = "snake_case")]
pub enum WhoCanAskPolicy {
    OwnerOnly,
    AllowList(Vec<String>),
    AnyChannel,
}

impl WhoCanAskPolicy {
    pub fn allows(&self, sender: &str, is_owner: bool) -> bool {
        match self {
            WhoCanAskPolicy::OwnerOnly => is_owner,
            WhoCanAskPolicy::AllowList(allowed) => {
                is_owner || allowed.iter().any(|s| s == sender)
            }
            WhoCanAskPolicy::AnyChannel => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonaProfile {
    pub id: String,
    pub name: String,
    pub role: String,
    pub default_workbench: PersonaWorkbench,
    pub memory_scope: String,
    pub allowed_triggers: WhoCanAskPolicy,
    pub look_around_job_id: Option<String>,
}

