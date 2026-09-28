//! Cognitive System Configuration
//!
//! Controls routing preferences, deliberation limits, and fallback settings.

use crate::provider_selector::TierProviderMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveConfig {
    pub tier_providers: TierProviderMap,
    pub default_timeout_secs: u64,
    pub max_deliberation_cycles: usize,
    pub enable_a2a_fallback: bool,
    pub human_gate_auto_approve_tier0: bool,
}

impl Default for CognitiveConfig {
    fn default() -> Self {
        Self {
            tier_providers: TierProviderMap::default(),
            default_timeout_secs: 60,
            max_deliberation_cycles: 3,
            enable_a2a_fallback: true,
            human_gate_auto_approve_tier0: true,
        }
    }
}

impl CognitiveConfig {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_config_roundtrip() {
        let config = CognitiveConfig::default();
        let json = config.to_json().unwrap();
        let deserialized = CognitiveConfig::from_json(&json).unwrap();

        assert_eq!(
            deserialized.default_timeout_secs,
            config.default_timeout_secs
        );
        assert_eq!(
            deserialized.max_deliberation_cycles,
            config.max_deliberation_cycles
        );
        assert!(deserialized.enable_a2a_fallback);
    }
}
