use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

/// A measurement never collapses missing telemetry into a zero value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "certainty", content = "value")]
pub enum UsageMeasurement {
    Known(u64),
    Estimated(u64),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageExecutorKind {
    Model,
    NativeHarness,
    Tool,
    Compute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub task_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub executor_kind: UsageExecutorKind,
    pub executor_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: UsageMeasurement,
    pub output_tokens: UsageMeasurement,
    pub cost_micros: UsageMeasurement,
    pub wall_time_ms: UsageMeasurement,
    pub recorded_at: DateTime<Utc>,
}

impl UsageRecord {
    pub fn new(
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        attempt_id: impl Into<String>,
        executor_kind: UsageExecutorKind,
        executor_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let task_id = task_id.into();
        let run_id = run_id.into();
        let attempt_id = attempt_id.into();
        let executor_id = executor_id.into();
        if [
            task_id.as_str(),
            run_id.as_str(),
            attempt_id.as_str(),
            executor_id.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(DomainError::Validation(
                "Usage record requires task, run, attempt, and executor identity".into(),
            ));
        }

        Ok(Self {
            id: new_id("usage"),
            task_id,
            run_id,
            attempt_id,
            executor_kind,
            executor_id,
            provider: None,
            model: None,
            input_tokens: UsageMeasurement::Unknown,
            output_tokens: UsageMeasurement::Unknown,
            cost_micros: UsageMeasurement::Unknown,
            wall_time_ms: UsageMeasurement::Unknown,
            recorded_at: Utc::now(),
        })
    }

    pub fn with_model_identity(
        mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let provider = provider.into();
        let model = model.into();
        if provider.trim().is_empty() || model.trim().is_empty() {
            return Err(DomainError::Validation(
                "Model usage identity cannot be empty".into(),
            ));
        }
        self.provider = Some(provider);
        self.model = Some(model);
        Ok(self)
    }
}
