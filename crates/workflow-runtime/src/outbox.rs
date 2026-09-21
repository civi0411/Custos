//! Transactional Outbox Pattern

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutboxMessage {
    pub id: String,
    pub topic: String,
    pub payload: serde_json::Value,
    pub sent: bool,
}
