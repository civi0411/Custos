//! Persona Action Queue (Triage Items), 1-Tap Ask Cards, and Multi-Persona Rooms.

use serde::{Deserialize, Serialize};

use crate::conversation::session::WorkbenchId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaItemBucket {
    NeedsYou,
    UpNext,
    Handled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaItemKind {
    PermitApproval,
    HookPermission,
    MemoryProposal,
    LookAroundFinding,
    TaskResult,
}

/// Optional pointer to an external source-of-truth table (`outbox_entries`, `memory_fact_proposals`, etc.)
/// so `persona_items` never duplicates authoritative approval state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaItemSource {
    pub source_kind: String,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaAskOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaAskQuestion {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub multi_select: bool,
    pub options: Vec<PersonaAskOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaItem {
    pub id: String,
    pub persona_id: String,
    pub bucket: PersonaItemBucket,
    pub kind: PersonaItemKind,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub ask_questions: Vec<PersonaAskQuestion>,
    #[serde(default)]
    pub source_ref: Option<PersonaItemSource>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub payload_json: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRoom {
    pub id: String,
    pub title: String,
    pub member_persona_ids: Vec<String>,
    #[serde(default)]
    pub default_workbench: Option<WorkbenchId>,
    /// Soft reference to an active streaming session; room lifetime is independent of `sessions`.
    #[serde(default)]
    pub active_session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaRoomSenderKind {
    Owner,
    Persona,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRoomMessage {
    pub id: String,
    pub room_id: String,
    pub sender: String,
    pub sender_kind: PersonaRoomSenderKind,
    #[serde(default)]
    pub target_persona_id: Option<String>,
    #[serde(default)]
    pub root_request_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub created_at: String,
}

