//! Span Lifecycle & Execution Units
//!
//! A Span represents an atomic execution step within a Task.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanState {
    Started,
    Completed,
    Failed,
}

impl std::fmt::Display for SpanState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanState::Started => write!(f, "started"),
            SpanState::Completed => write!(f, "completed"),
            SpanState::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub id: String,
    pub task_id: String,
    pub span_num: u32,
    pub state: SpanState,
    pub provider: String,
    pub model: String,
    pub input_digest: String,
    pub output_digest: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl Span {
    pub fn new(
        id: String,
        task_id: String,
        span_num: u32,
        provider: String,
        model: String,
        input_digest: String,
    ) -> Self {
        Self {
            id,
            task_id,
            span_num,
            state: SpanState::Started,
            provider,
            model,
            input_digest,
            output_digest: None,
            started_at: Utc::now(),
            ended_at: None,
        }
    }

    pub fn complete(&mut self, output_digest: String) -> Result<(), DomainError> {
        if self.state != SpanState::Started {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.to_string(),
                to: "completed".to_string(),
            });
        }
        self.state = SpanState::Completed;
        self.output_digest = Some(output_digest);
        self.ended_at = Some(Utc::now());
        Ok(())
    }

    pub fn fail(&mut self, error_digest: String) -> Result<(), DomainError> {
        if self.state != SpanState::Started {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.to_string(),
                to: "failed".to_string(),
            });
        }
        self.state = SpanState::Failed;
        self.output_digest = Some(error_digest);
        self.ended_at = Some(Utc::now());
        Ok(())
    }
}
