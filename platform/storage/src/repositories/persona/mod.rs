mod registry;
mod rooms;
mod triage;

use crate::connection::DbConnection;
use async_trait::async_trait;
use custos_contracts::oi::{
    AgentPersonaProfile, PersonaItem, PersonaItemBucket, PersonaRoom, PersonaRoomMessage,
};
use custos_contracts::{DomainError, PersonaRegistryPort, PersonaRoomPort, PersonaTriagePort};

#[derive(Clone)]
pub struct PersonaRepository {
    pub(super) db: DbConnection,
}

impl PersonaRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PersonaRegistryPort for PersonaRepository {
    async fn get_persona(&self, persona_id: &str) -> Result<Option<AgentPersonaProfile>, DomainError> {
        self.get_persona_sync(persona_id)
    }

    async fn list_personas(&self) -> Result<Vec<AgentPersonaProfile>, DomainError> {
        self.list_personas_sync()
    }

    async fn upsert_persona(&self, profile: &AgentPersonaProfile) -> Result<(), DomainError> {
        self.upsert_persona_sync(profile)
    }

    async fn set_persona_enabled(&self, persona_id: &str, enabled: bool) -> Result<(), DomainError> {
        self.set_persona_enabled_sync(persona_id, enabled)
    }
}

#[async_trait]
impl PersonaTriagePort for PersonaRepository {
    async fn get_item(&self, item_id: &str) -> Result<Option<PersonaItem>, DomainError> {
        self.get_item_sync(item_id)
    }

    async fn list_items(
        &self,
        persona_id: Option<&str>,
        bucket: Option<PersonaItemBucket>,
    ) -> Result<Vec<PersonaItem>, DomainError> {
        self.list_items_sync(persona_id, bucket)
    }

    async fn upsert_item(&self, item: &PersonaItem) -> Result<(), DomainError> {
        self.upsert_item_sync(item)
    }

    async fn resolve_item(&self, item_id: &str, resolved_at: &str) -> Result<(), DomainError> {
        self.resolve_item_sync(item_id, resolved_at)
    }
}

#[async_trait]
impl PersonaRoomPort for PersonaRepository {
    async fn create_room(&self, room: &PersonaRoom) -> Result<(), DomainError> {
        self.create_room_sync(room)
    }

    async fn get_room(&self, room_id: &str) -> Result<Option<PersonaRoom>, DomainError> {
        self.get_room_sync(room_id)
    }

    async fn list_rooms(&self) -> Result<Vec<PersonaRoom>, DomainError> {
        self.list_rooms_sync()
    }

    async fn append_room_message(&self, message: &PersonaRoomMessage) -> Result<(), DomainError> {
        self.append_room_message_sync(message)
    }

    async fn list_room_messages(
        &self,
        room_id: &str,
        limit: usize,
    ) -> Result<Vec<PersonaRoomMessage>, DomainError> {
        self.list_room_messages_sync(room_id, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SqliteTaskStore;
    use custos_contracts::oi::{
        ActsForTrustPolicy, MemoryScopeId, PersonaAskOption, PersonaAskQuestion, PersonaItemKind,
        PersonaRoomSenderKind, WhoCanAskPolicy,
    };
    use custos_contracts::WorkbenchId;

    #[tokio::test]
    async fn persona_registry_triage_and_rooms_roundtrip() {
        let store = SqliteTaskStore::new_in_memory().expect("in-memory store");
        let personas = store.personas();

        let profile = AgentPersonaProfile {
            id: "persona-quant".into(),
            name: "Quant".into(),
            role: "Quantitative Strategist".into(),
            default_workbench: WorkbenchId::new(WorkbenchId::QUANT),
            memory_scope: MemoryScopeId::new("persona:quant"),
            allowed_triggers: WhoCanAskPolicy::OwnerOnly,
            acts_for: ActsForTrustPolicy::default(),
            sovereign_instructions: "Never exceed drawdown limits.".into(),
            look_around_job_id: Some("job-1".into()),
            max_up_next_items: 5,
            enabled: true,
        };
        personas.upsert_persona(&profile).await.expect("upsert");
        let fetched = personas
            .get_persona("persona-quant")
            .await
            .expect("get")
            .expect("some");
        assert_eq!(
            fetched.default_workbench,
            WorkbenchId::new(WorkbenchId::QUANT)
        );
        assert_eq!(fetched.memory_scope.as_str(), "persona:quant");

        let item = PersonaItem {
            id: "pitem-1".into(),
            persona_id: "persona-quant".into(),
            bucket: PersonaItemBucket::NeedsYou,
            kind: PersonaItemKind::LookAroundFinding,
            title: "Sharpe drift detected".into(),
            summary: "Backtest Sharpe dropped below 1.2".into(),
            ask_questions: vec![PersonaAskQuestion {
                id: "q1".into(),
                prompt: "Pause live paper strategy?".into(),
                multi_select: false,
                options: vec![PersonaAskOption {
                    id: "pause".into(),
                    label: "Pause now".into(),
                    recommended: true,
                }],
            }],
            source_ref: None,
            task_id: None,
            run_id: None,
            priority: 10,
            payload_json: serde_json::json!({}),
            created_at: "2026-11-28T00:00:00Z".into(),
            updated_at: "2026-11-28T00:00:00Z".into(),
            resolved_at: None,
        };
        personas.upsert_item(&item).await.expect("upsert item");
        assert_eq!(
            personas
                .list_items(Some("persona-quant"), Some(PersonaItemBucket::NeedsYou))
                .await
                .expect("list needs_you")
                .len(),
            1
        );
        personas
            .resolve_item("pitem-1", "2026-11-28T01:00:00Z")
            .await
            .expect("resolve");
        assert_eq!(
            personas
                .list_items(Some("persona-quant"), Some(PersonaItemBucket::Handled))
                .await
                .expect("list handled")
                .len(),
            1
        );

        let room = PersonaRoom {
            id: "room-1".into(),
            title: "Morning Standup".into(),
            member_persona_ids: vec!["persona-quant".into()],
            default_workbench: Some(WorkbenchId::new(WorkbenchId::QUANT)),
            active_session_id: None,
            created_at: "2026-11-28T00:00:00Z".into(),
            updated_at: "2026-11-28T00:00:00Z".into(),
        };
        personas.create_room(&room).await.expect("create room");
        personas
            .append_room_message(&PersonaRoomMessage {
                id: "msg-1".into(),
                room_id: "room-1".into(),
                sender: "owner".into(),
                sender_kind: PersonaRoomSenderKind::Owner,
                target_persona_id: Some("persona-quant".into()),
                root_request_id: Some("req-1".into()),
                content: "@quant check overnight drift".into(),
                task_id: None,
                run_id: None,
                created_at: "2026-11-28T00:01:00Z".into(),
            })
            .await
            .expect("append msg");
        assert_eq!(
            personas
                .list_room_messages("room-1", 10)
                .await
                .expect("list msgs")
                .len(),
            1
        );
    }
}
