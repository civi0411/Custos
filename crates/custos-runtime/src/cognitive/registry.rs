//! Provider Registry & Health Tracker
//!
//! Maintains active ModelProvider instances, tracks health/latency statistics,
//! and resolves the best available provider for a given ReasoningTier.

use crate::provider_selector::{ProviderPreference, TierProviderMap};
use crate::routing::ReasoningTier;
use chrono::{DateTime, Utc};
use custos_core_domain::DomainError;
use custos_provider_sdk::port::ModelProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded {
        consecutive_failures: u32,
        last_error: String,
    },
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub status: HealthStatus,
    pub latency_ema_ms: f32,
    pub total_requests: u64,
    pub total_failures: u64,
    pub last_attempt_at: Option<DateTime<Utc>>,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            status: HealthStatus::Healthy,
            latency_ema_ms: 0.0,
            total_requests: 0,
            total_failures: 0,
            last_attempt_at: None,
        }
    }
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn ModelProvider>>,
    health: HashMap<String, ProviderHealth>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            health: HashMap::new(),
        }
    }

    /// Register a provider implementation
    pub fn register(&mut self, provider: Arc<dyn ModelProvider>) {
        let id = provider.provider_id().to_string();
        self.health.entry(id.clone()).or_default();
        self.providers.insert(id, provider);
    }

    /// Get registered provider by ID
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ModelProvider>> {
        self.providers.get(provider_id).cloned()
    }

    /// Check if provider is currently healthy or degraded (still operable)
    pub fn is_operable(&self, provider_id: &str) -> bool {
        match self.health.get(provider_id) {
            Some(h) => !matches!(h.status, HealthStatus::Unavailable),
            None => false,
        }
    }

    /// Record a successful provider invocation
    pub fn record_success(&mut self, provider_id: &str, latency_ms: u64) {
        let entry = self.health.entry(provider_id.to_string()).or_default();
        entry.total_requests += 1;
        entry.last_attempt_at = Some(Utc::now());

        // Exponential moving average (alpha = 0.2)
        if entry.latency_ema_ms == 0.0 {
            entry.latency_ema_ms = latency_ms as f32;
        } else {
            entry.latency_ema_ms = entry.latency_ema_ms * 0.8 + (latency_ms as f32) * 0.2;
        }

        entry.status = HealthStatus::Healthy;
    }

    /// Record a provider failure (updates circuit breaker status)
    pub fn record_failure(&mut self, provider_id: &str, error: &str) {
        let entry = self.health.entry(provider_id.to_string()).or_default();
        entry.total_requests += 1;
        entry.total_failures += 1;
        entry.last_attempt_at = Some(Utc::now());

        let consecutive = match &entry.status {
            HealthStatus::Degraded {
                consecutive_failures,
                ..
            } => consecutive_failures + 1,
            _ => 1,
        };

        if consecutive >= 3 {
            entry.status = HealthStatus::Unavailable;
        } else {
            entry.status = HealthStatus::Degraded {
                consecutive_failures: consecutive,
                last_error: error.to_string(),
            };
        }
    }

    /// Select highest priority operable provider for a tier matching tier preferences
    pub fn select_for_tier(
        &self,
        tier: ReasoningTier,
        prefs: &TierProviderMap,
    ) -> Option<(Arc<dyn ModelProvider>, ProviderPreference)> {
        let candidates = prefs.get_candidates(tier);
        for pref in candidates {
            if !pref.is_available {
                continue;
            }
            if let Some(provider) = self.providers.get(&pref.provider_id) {
                if self.is_operable(&pref.provider_id) {
                    return Some((Arc::clone(provider), pref));
                }
            }
        }
        None
    }

    /// Select provider for tier with fallback or return DomainError
    pub fn select_with_fallback(
        &self,
        tier: ReasoningTier,
        prefs: &TierProviderMap,
    ) -> Result<(Arc<dyn ModelProvider>, ProviderPreference), DomainError> {
        if let Some(match_result) = self.select_for_tier(tier, prefs) {
            return Ok(match_result);
        }

        // Fallback: If SystemTwo requested and unavailable, attempt SystemOne
        if tier == ReasoningTier::SystemTwo {
            if let Some(fallback_match) = self.select_for_tier(ReasoningTier::SystemOne, prefs) {
                return Ok(fallback_match);
            }
        }

        Err(DomainError::NotFound {
            kind: "ModelProvider".to_string(),
            id: format!("No operable provider available for tier {:?}", tier),
        })
    }
}

// Add convenience helper on TierProviderMap for candidate lookup
impl TierProviderMap {
    pub fn get_candidates(&self, tier: ReasoningTier) -> Vec<ProviderPreference> {
        let key = match tier {
            ReasoningTier::TierZero => "tier_zero",
            ReasoningTier::SystemOne => "system_one",
            ReasoningTier::SystemTwo => "system_two",
            ReasoningTier::Abstain => "abstain",
        };

        if let Some(list) = self.tier_mappings.get(key) {
            let mut sorted = list.clone();
            sorted.sort_by_key(|a| std::cmp::Reverse(a.priority));
            sorted
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_provider_sdk::request::{ModelResponse, ProviderRequest};

    struct TestProvider {
        id: String,
    }

    #[async_trait]
    impl ModelProvider for TestProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            Ok(ModelResponse::text("test response", "test-model", 10))
        }
    }

    #[test]
    fn test_provider_registration_and_health_tracking() {
        let mut registry = ProviderRegistry::new();
        let provider = Arc::new(TestProvider {
            id: "claude".into(),
        });
        registry.register(provider);

        assert!(registry.is_operable("claude"));
        registry.record_success("claude", 120);

        let health = registry.health.get("claude").unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.total_requests, 1);
        assert_eq!(health.latency_ema_ms, 120.0);

        // Record 3 consecutive failures to trigger Unavailable circuit breaker
        registry.record_failure("claude", "rate limit");
        assert!(registry.is_operable("claude")); // degraded
        registry.record_failure("claude", "timeout");
        registry.record_failure("claude", "network error");
        assert!(!registry.is_operable("claude")); // unavailable
    }

    #[test]
    fn test_tier_selection_and_fallback() {
        let mut registry = ProviderRegistry::new();
        let claude = Arc::new(TestProvider {
            id: "claude".into(),
        });
        let codex = Arc::new(TestProvider { id: "codex".into() });

        registry.register(claude);
        registry.register(codex);

        let prefs = TierProviderMap::default();

        // System 1 should select claude first
        let (selected, pref) = registry
            .select_for_tier(ReasoningTier::SystemOne, &prefs)
            .unwrap();
        assert_eq!(selected.provider_id(), "claude");
        assert_eq!(pref.model_id, "claude-3-5-haiku");

        // Mark claude unavailable -> should pick codex
        registry.record_failure("claude", "err1");
        registry.record_failure("claude", "err2");
        registry.record_failure("claude", "err3");

        let (selected2, pref2) = registry
            .select_for_tier(ReasoningTier::SystemOne, &prefs)
            .unwrap();
        assert_eq!(selected2.provider_id(), "codex");
        assert_eq!(pref2.model_id, "gpt-4o-mini");
    }
}
