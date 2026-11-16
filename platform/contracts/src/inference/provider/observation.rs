//! Provider & Model Health Observation Domain Types
//!
//! Pure domain representation of health, latency, failure statistics,
//! and admission availability across inference providers and models.
//! Zero-I/O pure data contracts for OI, routing, runtime, and UI telemetry.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderHealthStatus {
    #[default]
    Healthy,
    Degraded {
        consecutive_failures: u32,
        last_error: String,
    },
    Unavailable {
        kind: String,
        reason: String,
        retry_at: Option<i64>,
    },
}

/// Unified atomic snapshot of a provider/model endpoint's operational state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderObservationSnapshot {
    pub provider_id: String,
    pub model_id: Option<String>,
    pub status: ProviderHealthStatus,
    pub latency_ema_ms: f32,
    pub total_requests: u64,
    pub total_failures: u64,
    pub last_attempt_at: Option<i64>,
}

impl ProviderObservationSnapshot {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: None,
            status: ProviderHealthStatus::Healthy,
            latency_ema_ms: 0.0,
            total_requests: 0,
            total_failures: 0,
            last_attempt_at: None,
        }
    }

    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_status(mut self, status: ProviderHealthStatus) -> Self {
        self.status = status;
        self
    }

    pub fn is_operable(&self) -> bool {
        !matches!(self.status, ProviderHealthStatus::Unavailable { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_observation_snapshot_lifecycle() {
        let mut obs = ProviderObservationSnapshot::new("anthropic")
            .with_model("claude-3-5-sonnet")
            .with_status(ProviderHealthStatus::Healthy);

        assert!(obs.is_operable());
        assert_eq!(obs.provider_id, "anthropic");
        assert_eq!(obs.model_id.as_deref(), Some("claude-3-5-sonnet"));

        obs.status = ProviderHealthStatus::Unavailable {
            kind: "quota".into(),
            reason: "Monthly quota exhausted".into(),
            retry_at: Some(1730000000),
        };
        assert!(!obs.is_operable());
    }
}

