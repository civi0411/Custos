//! Custos Local API Protocol Envelopes
//!
//! Canonical wrappers for mutating commands and reconnectable events
//! across the Custos Local API boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported schema versions for the Custos Local API
pub const PROTOCOL_SCHEMA_VERSION_V1: u32 = 1;

/// Standard envelope for all state-mutating commands sent to Custos Local API.
/// Enforces idempotency via `command_id`, traceability via `correlation_id`,
/// optimistic concurrency control via `expected_revision`, and caller authorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandEnvelope<T> {
    pub schema_version: u32,
    pub command_id: Uuid,
    pub correlation_id: Uuid,
    pub actor: AuthenticatedActor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    pub payload: T,
}

impl<T> CommandEnvelope<T> {
    pub fn new(
        command_id: Uuid,
        correlation_id: Uuid,
        actor: AuthenticatedActor,
        payload: T,
    ) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION_V1,
            command_id,
            correlation_id,
            actor,
            task_id: None,
            expected_revision: None,
            deadline: None,
            payload,
        }
    }

    pub fn with_task_id(mut self, task_id: Uuid) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn with_expected_revision(mut self, rev: u64) -> Self {
        self.expected_revision = Some(rev);
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

/// Verified caller identity for incoming Local API commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticatedActor {
    pub actor_id: String,
    pub actor_type: String, // "desktop_tauri", "cli_session", "ide_extension", "system"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token_digest: Option<String>,
}

impl AuthenticatedActor {
    pub fn desktop(actor_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            actor_type: "desktop_tauri".to_string(),
            process_id: Some(std::process::id()),
            auth_token_digest: None,
        }
    }

    pub fn cli(actor_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            actor_type: "cli_session".to_string(),
            process_id: Some(std::process::id()),
            auth_token_digest: None,
        }
    }

    pub fn system(actor_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            actor_type: "system".to_string(),
            process_id: Some(std::process::id()),
            auth_token_digest: None,
        }
    }
}

/// Standard envelope for all events emitted from the Custos Local API.
/// Includes monotonically increasing `cursor` for ordered event replay and reconnection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventEnvelope<T> {
    pub cursor: u64,
    pub event_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub payload: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        cursor: u64,
        event_id: Uuid,
        event_type: impl Into<String>,
        payload: T,
    ) -> Self {
        Self {
            cursor,
            event_id,
            task_id: None,
            run_id: None,
            timestamp: Utc::now(),
            event_type: event_type.into(),
            payload,
        }
    }

    pub fn with_task_id(mut self, task_id: Uuid) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn with_run_id(mut self, run_id: Uuid) -> Self {
        self.run_id = Some(run_id);
        self
    }
}
