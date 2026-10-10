//! Virtual Model Combos and Capability-Aware Routing
//!
//! Synthesized from 9Router (`combo.js` and `comboPresets.js`).
//! Allows the runtime to target virtual tiers (`fast-combo`, `smart-combo`, `reasoning-combo`)
//! and dynamically re-orders candidates according to required capabilities (e.g., Vision, Tools).

use crate::catalog::capabilities::{get_model_capabilities, ModelCapabilities};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard model tier categories for cost/intelligence trade-offs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Fast,
    Balanced,
    Heavy,
    Reasoning,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Heavy => "heavy",
            Self::Reasoning => "reasoning",
        }
    }
}

/// Explicit capabilities required for a turn
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequiredCapability {
    Vision,
    Pdf,
    Tools,
    Reasoning,
    Search,
}

impl RequiredCapability {
    pub fn is_satisfied_by(&self, caps: &ModelCapabilities) -> bool {
        match self {
            Self::Vision => caps.vision,
            Self::Pdf => caps.pdf,
            Self::Tools => caps.tools,
            Self::Reasoning => caps.reasoning,
            Self::Search => caps.search,
        }
    }
}

/// A configured Virtual Model Combo Preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboPreset {
    pub name: String,
    pub tier: ModelTier,
    pub candidates: Vec<String>,
    pub fallback_enabled: bool,
}

impl ComboPreset {
    pub fn new(name: impl Into<String>, tier: ModelTier, candidates: Vec<String>) -> Self {
        Self {
            name: name.into(),
            tier,
            candidates,
            fallback_enabled: true,
        }
    }
}

/// Registry of virtual combos and capability-aware route resolvers
#[derive(Debug, Clone)]
pub struct ComboRegistry {
    presets: HashMap<String, ComboPreset>,
}

impl Default for ComboRegistry {
    fn default() -> Self {
        let mut registry = Self {
            presets: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }
}

impl ComboRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register default virtual combos
    pub fn register_defaults(&mut self) {
        self.register(ComboPreset::new(
            "fast-combo",
            ModelTier::Fast,
            vec![
                "gemini-1.5-flash".to_string(),
                "gpt-4o-mini".to_string(),
                "claude-3-5-haiku".to_string(),
            ],
        ));

        self.register(ComboPreset::new(
            "smart-combo",
            ModelTier::Heavy,
            vec![
                "claude-3-5-sonnet".to_string(),
                "gpt-4o".to_string(),
                "gemini-1.5-pro".to_string(),
            ],
        ));

        self.register(ComboPreset::new(
            "reasoning-combo",
            ModelTier::Reasoning,
            vec![
                "o1".to_string(),
                "o3-mini".to_string(),
                "deepseek-r1".to_string(),
            ],
        ));

        self.register(ComboPreset::new(
            "code-combo",
            ModelTier::Balanced,
            vec![
                "claude-3-5-sonnet".to_string(),
                "gpt-4o".to_string(),
                "deepseek-coder".to_string(),
            ],
        ));
    }

    /// Register a custom combo preset
    pub fn register(&mut self, preset: ComboPreset) {
        self.presets.insert(preset.name.clone(), preset);
    }

    /// Lookup a combo preset by name
    pub fn get_preset(&self, name: &str) -> Option<&ComboPreset> {
        self.presets.get(name)
    }

    /// Check if a model identifier refers to a virtual combo
    pub fn is_combo(&self, name: &str) -> bool {
        self.presets.contains_key(name)
    }

    /// Resolve candidates for a combo, enforcing hard capability constraints.
    ///
    /// Candidates that satisfy ALL required capabilities are retained.
    /// Candidates missing any required capability are strictly filtered out to prevent
    /// dispatching incompatible requests (e.g. vision or tools to non-supporting models).
    pub fn resolve_candidates(&self, name: &str, required: &[RequiredCapability]) -> Vec<String> {
        let Some(preset) = self.presets.get(name) else {
            return vec![name.to_string()];
        };

        if required.is_empty() {
            return preset.candidates.clone();
        }

        preset
            .candidates
            .iter()
            .filter(|candidate| {
                let caps = get_model_capabilities(candidate);
                required.iter().all(|req| req.is_satisfied_by(&caps))
            })
            .cloned()
            .collect()
    }

    /// Resolve candidates applying both hard capabilities and route constraints (pin, privacy/local-only).
    pub fn resolve_with_constraints(
        &self,
        name: &str,
        required: &[RequiredCapability],
        constraints: &RouteConstraints,
    ) -> Vec<String> {
        // 1. User Model Pin is an absolute constraint
        if let Some(ref pin) = constraints.pinned_model {
            let caps = get_model_capabilities(pin);
            if required.iter().all(|req| req.is_satisfied_by(&caps)) {
                return vec![pin.clone()];
            } else {
                return vec![]; // Pinned model fails hard required capability constraint!
            }
        }

        let candidates = self.resolve_candidates(name, required);

        // 2. Local-only / Privacy boundary constraint (disallow cloud egress)
        if constraints.local_only {
            candidates
                .into_iter()
                .filter(|c| {
                    let m = c.to_lowercase();
                    m.starts_with("llama")
                        || m.starts_with("qwen")
                        || m.starts_with("mistral")
                        || m.starts_with("ollama")
                        || m == "local"
                })
                .collect()
        } else {
            candidates
        }
    }
}

/// Hard routing constraints enforced before candidate selection
#[derive(Debug, Clone, Default)]
pub struct RouteConstraints {
    pub pinned_model: Option<String>,
    pub local_only: bool,
    pub max_cost_per_m_tokens: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_combos_registered() {
        let registry = ComboRegistry::default();
        assert!(registry.is_combo("fast-combo"));
        assert!(registry.is_combo("smart-combo"));
        assert!(registry.is_combo("reasoning-combo"));
        assert!(!registry.is_combo("gpt-4o"));
    }

    #[test]
    fn test_resolve_without_requirements() {
        let registry = ComboRegistry::default();
        let candidates = registry.resolve_candidates("fast-combo", &[]);
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], "gemini-1.5-flash");
    }

    #[test]
    fn test_resolve_with_reasoning_requirement() {
        let registry = ComboRegistry::default();
        // o1 has reasoning: true
        let resolved = registry.resolve_candidates("reasoning-combo", &[RequiredCapability::Reasoning]);
        assert_eq!(resolved[0], "o1");
    }

    #[test]
    fn test_pinned_model_constraint() {
        let registry = ComboRegistry::default();
        let constraints = RouteConstraints {
            pinned_model: Some("gpt-4o".to_string()),
            local_only: false,
            max_cost_per_m_tokens: None,
        };
        // gpt-4o has vision: true
        let resolved = registry.resolve_with_constraints(
            "fast-combo",
            &[RequiredCapability::Vision],
            &constraints,
        );
        assert_eq!(resolved, vec!["gpt-4o"]);
    }

    #[test]
    fn test_local_only_privacy_constraint() {
        let mut registry = ComboRegistry::default();
        registry.register(ComboPreset::new(
            "hybrid-combo",
            ModelTier::Fast,
            vec![
                "gemini-1.5-flash".to_string(),
                "qwen-2.5-coder".to_string(),
                "llama-3.2-3b".to_string(),
            ],
        ));
        let constraints = RouteConstraints {
            pinned_model: None,
            local_only: true,
            max_cost_per_m_tokens: None,
        };
        let resolved = registry.resolve_with_constraints("hybrid-combo", &[], &constraints);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], "qwen-2.5-coder");
        assert_eq!(resolved[1], "llama-3.2-3b");
    }
}
