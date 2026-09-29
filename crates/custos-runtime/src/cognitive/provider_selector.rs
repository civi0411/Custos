use crate::routing::ReasoningTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderPreference {
    pub provider_id: String,
    pub model_id: String,
    pub cost_per_1k_tokens_usd: f32,
    pub priority: u8, // Higher number = higher priority
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierProviderMap {
    pub tier_mappings: HashMap<String, Vec<ProviderPreference>>,
}

impl Default for TierProviderMap {
    fn default() -> Self {
        let mut tier_mappings = HashMap::new();

        // System 0
        tier_mappings.insert(
            "tier_zero".to_string(),
            vec![ProviderPreference {
                provider_id: "deterministic".into(),
                model_id: "system-0-exact".into(),
                cost_per_1k_tokens_usd: 0.0,
                priority: 100,
                is_available: true,
            }],
        );

        // System 1 (fast, low cost)
        tier_mappings.insert(
            "system_one".to_string(),
            vec![
                ProviderPreference {
                    provider_id: "claude".into(),
                    model_id: "claude-3-5-haiku".into(),
                    cost_per_1k_tokens_usd: 0.001,
                    priority: 90,
                    is_available: true,
                },
                ProviderPreference {
                    provider_id: "codex".into(),
                    model_id: "gpt-4o-mini".into(),
                    cost_per_1k_tokens_usd: 0.0006,
                    priority: 80,
                    is_available: true,
                },
                ProviderPreference {
                    provider_id: "local".into(),
                    model_id: "qwen2.5:7b".into(),
                    cost_per_1k_tokens_usd: 0.0,
                    priority: 70,
                    is_available: true,
                },
            ],
        );

        // System 2 (deep deliberation, complex tasks)
        tier_mappings.insert(
            "system_two".to_string(),
            vec![
                ProviderPreference {
                    provider_id: "claude".into(),
                    model_id: "claude-3-5-sonnet".into(),
                    cost_per_1k_tokens_usd: 0.015,
                    priority: 95,
                    is_available: true,
                },
                ProviderPreference {
                    provider_id: "codex".into(),
                    model_id: "o3-mini".into(),
                    cost_per_1k_tokens_usd: 0.012,
                    priority: 85,
                    is_available: true,
                },
                ProviderPreference {
                    provider_id: "local".into(),
                    model_id: "deepseek-r1:14b".into(),
                    cost_per_1k_tokens_usd: 0.0,
                    priority: 70,
                    is_available: true,
                },
            ],
        );

        Self { tier_mappings }
    }
}

pub struct ProviderSelector {
    config: TierProviderMap,
}

impl Default for ProviderSelector {
    fn default() -> Self {
        Self::new(TierProviderMap::default())
    }
}

impl ProviderSelector {
    pub fn new(config: TierProviderMap) -> Self {
        Self { config }
    }

    fn tier_key(tier: ReasoningTier) -> &'static str {
        match tier {
            ReasoningTier::TierZero => "tier_zero",
            ReasoningTier::SystemOne => "system_one",
            ReasoningTier::SystemTwo => "system_two",
            ReasoningTier::Abstain => "abstain",
        }
    }

    /// Selects the highest-priority available provider for a given tier.
    pub fn select_best(&self, tier: ReasoningTier) -> Option<ProviderPreference> {
        let key = Self::tier_key(tier);
        let list = self.config.tier_mappings.get(key)?;

        let mut available: Vec<_> = list.iter().filter(|p| p.is_available).cloned().collect();
        available.sort_by_key(|a| std::cmp::Reverse(a.priority));
        available.into_iter().next()
    }

    /// Returns all available candidates for a given tier, sorted by priority.
    pub fn list_candidates(&self, tier: ReasoningTier) -> Vec<ProviderPreference> {
        let key = Self::tier_key(tier);
        let Some(list) = self.config.tier_mappings.get(key) else {
            return Vec::new();
        };

        let mut candidates: Vec<_> = list.iter().filter(|p| p.is_available).cloned().collect();
        candidates.sort_by_key(|a| std::cmp::Reverse(a.priority));
        candidates
    }

    /// Marks a provider as available or unavailable (e.g. on circuit breaker trigger).
    pub fn update_availability(&mut self, provider_id: &str, available: bool) {
        for list in self.config.tier_mappings.values_mut() {
            for p in list.iter_mut() {
                if p.provider_id == provider_id {
                    p.is_available = available;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_best_system_one() {
        let selector = ProviderSelector::default();
        let best = selector.select_best(ReasoningTier::SystemOne).unwrap();
        assert_eq!(best.provider_id, "claude");
        assert_eq!(best.model_id, "claude-3-5-haiku");
    }

    #[test]
    fn test_fallback_when_primary_unavailable() {
        let mut selector = ProviderSelector::default();
        selector.update_availability("claude", false);

        let best = selector.select_best(ReasoningTier::SystemOne).unwrap();
        // Should fall back to codex gpt-4o-mini
        assert_eq!(best.provider_id, "codex");
        assert_eq!(best.model_id, "gpt-4o-mini");
    }

    #[test]
    fn test_tier_zero_selection() {
        let selector = ProviderSelector::default();
        let best = selector.select_best(ReasoningTier::TierZero).unwrap();
        assert_eq!(best.provider_id, "deterministic");
    }
}
