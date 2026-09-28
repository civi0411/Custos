//! Immutable Audit Log Engine
//!
//! Cryptographically records all security-sensitive events including
//! capability grants, approval decisions, and permit issuance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use custos_core_domain::{digest, new_id};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub event_type: String,
    pub task_id: String,
    pub actor: String,
    pub details: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub prev_digest: String,
    pub entry_digest: String,
}

#[derive(Debug, Default, Clone)]
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &self,
        event_type: &str,
        task_id: &str,
        actor: &str,
        details: serde_json::Value,
    ) -> AuditEntry {
        let mut guard = self.entries.write().expect("lock poisoned");
        let prev_digest = guard
            .last()
            .map(|e| e.entry_digest.clone())
            .unwrap_or_else(|| "0".repeat(64));

        let now = Utc::now();
        let payload = format!(
            "{}|{}|{}|{}|{}|{}",
            event_type,
            task_id,
            actor,
            details,
            now.to_rfc3339(),
            prev_digest
        );
        let entry_digest = digest(payload.as_bytes());

        let entry = AuditEntry {
            id: new_id("audit"),
            event_type: event_type.to_string(),
            task_id: task_id.to_string(),
            actor: actor.to_string(),
            details,
            timestamp: now,
            prev_digest,
            entry_digest,
        };

        guard.push(entry.clone());
        entry
    }

    pub fn len(&self) -> usize {
        let guard = self.entries.read().expect("lock poisoned");
        guard.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
