//! Parameter & Response DTOs for `v1.personas.*` Local API methods.

use custos_contracts::authority::action::RiskLevel;
use custos_contracts::oi::{PersonaItemBucket, PersonaRoomSenderKind};
use custos_contracts::WorkbenchId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskPersonaParams {
    pub persona_id: String,
    pub prompt: String,
    #[serde(default)]
    pub sender_id: Option<String>,
    #[serde(default)]
    pub caller_scope: Option<String>,
    #[serde(default)]
    pub is_paired_owner: Option<bool>,
    #[serde(default)]
    pub workbench: Option<WorkbenchId>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub requested_risk: Option<RiskLevel>,
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPersonaItemsParams {
    #[serde(default)]
    pub persona_id: Option<String>,
    #[serde(default)]
    pub bucket: Option<PersonaItemBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvePersonaItemParams {
    pub item_id: String,
    /// `"accept"` | `"dismiss"` | `"start"`
    pub decision: String,
    #[serde(default)]
    pub selected_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePersonaRoomParams {
    pub title: String,
    pub member_persona_ids: Vec<String>,
    #[serde(default)]
    pub default_workbench: Option<WorkbenchId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostPersonaRoomMessageParams {
    pub room_id: String,
    pub content: String,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub sender_kind: Option<PersonaRoomSenderKind>,
    #[serde(default)]
    pub root_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPersonaRoomHistoryParams {
    pub room_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

