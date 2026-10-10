//! Agent Model Catalog Service
//!
//! Provides curated model metadata, reasoning effort levels, context window sizes,
//! and pricing for AI model selection in Custos.
//! Incorporates OrCa's model catalog TTL store (10m fresh, 30s failure TTL) and effort mappings.

use super::pricing::lookup_model_pricing;
use custos_domain::{ModelCatalogOption, ModelCatalogResult, ProbedModel};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub const MODEL_CATALOG_FRESH_DURATION: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
struct CachedCatalogEntry {
    result: ModelCatalogResult,
    cached_at: Instant,
}

#[derive(Clone)]
pub struct ModelCatalogService {
    cache: Arc<RwLock<HashMap<String, CachedCatalogEntry>>>,
}

impl Default for ModelCatalogService {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelCatalogService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns the canonical model catalog, optionally filtered by provider type.
    pub fn get_catalog(&self, provider_type: Option<&str>) -> ModelCatalogResult {
        let cache_key = provider_type.unwrap_or("all").to_string();

        // 1. Check in-memory fresh cache
        if let Ok(guard) = self.cache.read() {
            if let Some(entry) = guard.get(&cache_key) {
                if entry.cached_at.elapsed() < MODEL_CATALOG_FRESH_DURATION {
                    return entry.result.clone();
                }
            }
        }

        // 2. Build canonical catalog
        let all_models = default_canonical_models();
        let filtered: Vec<ModelCatalogOption> = match provider_type {
            Some(pt) if !pt.is_empty() && pt != "all" => all_models
                .into_iter()
                .filter(|m| m.provider_type.eq_ignore_ascii_case(pt))
                .collect(),
            _ => all_models,
        };

        let result = ModelCatalogResult {
            origin: "catalog".into(),
            models: filtered,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        };

        // 3. Store in cache
        if let Ok(mut guard) = self.cache.write() {
            guard.insert(
                cache_key,
                CachedCatalogEntry {
                    result: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        result
    }

    /// Merges live probed models into catalog entries and caches them.
    pub fn merge_probed_models(
        &self,
        provider_type: &str,
        probed: Vec<ProbedModel>,
    ) -> ModelCatalogResult {
        let mut models = Vec::new();

        for pm in probed {
            let pricing = lookup_model_pricing(&pm.id);
            let is_default = models.is_empty();
            let label = pm.name.clone().unwrap_or_else(|| pm.id.clone());

            let default_effort = if pm.id.contains("o1")
                || pm.id.contains("o3")
                || pm.id.contains("reasoner")
                || pm.id.contains("thinking")
            {
                Some("medium".into())
            } else {
                None
            };

            let efforts = if default_effort.is_some() {
                vec!["low".into(), "medium".into(), "high".into()]
            } else {
                Vec::new()
            };

            models.push(ModelCatalogOption {
                id: pm.id,
                label,
                description: pm.description,
                provider_type: provider_type.to_string(),
                is_default,
                default_effort,
                efforts,
                supports_fast_mode: true,
                context_window: pm.context_window.or(Some(128_000)),
                pricing,
            });
        }

        let result = ModelCatalogResult {
            origin: "probe".into(),
            models,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        };

        if let Ok(mut guard) = self.cache.write() {
            guard.insert(
                provider_type.to_string(),
                CachedCatalogEntry {
                    result: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        result
    }
}

/// Curated baseline models reflecting OrCa and modern AI provider catalogs.
pub fn default_canonical_models() -> Vec<ModelCatalogOption> {
    vec![
        // Anthropic Claude
        ModelCatalogOption {
            id: "claude-3-7-sonnet".into(),
            label: "Claude 3.7 Sonnet".into(),
            description: Some("Hybrid reasoning & instant code intelligence".into()),
            provider_type: "anthropic".into(),
            is_default: true,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("claude-3-7-sonnet"),
        },
        ModelCatalogOption {
            id: "claude-3-5-sonnet".into(),
            label: "Claude 3.5 Sonnet".into(),
            description: Some("High-capability reasoning and coding model".into()),
            provider_type: "anthropic".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("claude-3-5-sonnet"),
        },
        ModelCatalogOption {
            id: "claude-3-5-haiku".into(),
            label: "Claude 3.5 Haiku".into(),
            description: Some("Ultra-fast lightweight reasoning model".into()),
            provider_type: "anthropic".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("claude-3-5-haiku"),
        },
        ModelCatalogOption {
            id: "claude-3-opus".into(),
            label: "claude-3-opus".into(),
            description: Some("Anthropic legacy flagship model".into()),
            provider_type: "anthropic".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("claude-3-opus"),
        },
        ModelCatalogOption {
            id: "claude-3-haiku".into(),
            label: "claude-3-haiku".into(),
            description: Some("Anthropic fast legacy model".into()),
            provider_type: "anthropic".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("claude-3-haiku"),
        },
        // OpenAI / Codex
        ModelCatalogOption {
            id: "gpt-5.6-sol".into(),
            label: "GPT-5.6 Sol".into(),
            description: Some("Flagship Codex reasoning and code generation model".into()),
            provider_type: "openai".into(),
            is_default: true,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("gpt-5.6-sol"),
        },
        ModelCatalogOption {
            id: "gpt-5.6-terra".into(),
            label: "GPT-5.6 Terra".into(),
            description: Some("High-throughput coding model optimized for agent workflows".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("gpt-5.6-terra"),
        },
        ModelCatalogOption {
            id: "gpt-5.6-luna".into(),
            label: "GPT-5.6 Luna".into(),
            description: Some("Ultra-fast low-latency coding model for real-time iterations".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("gpt-5.6-luna"),
        },
        ModelCatalogOption {
            id: "gpt-5.5".into(),
            label: "GPT-5.5".into(),
            description: Some("Codex 5.5 general reasoning and synthesis model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("gpt-5.5"),
        },
        ModelCatalogOption {
            id: "gpt-4o".into(),
            label: "GPT-4o".into(),
            description: Some("Flagship multimodal omni model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("gpt-4o"),
        },
        ModelCatalogOption {
            id: "o3-mini".into(),
            label: "o3-mini".into(),
            description: Some("Fast, cost-efficient reasoning model with effort tiers".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("o3-mini"),
        },
        ModelCatalogOption {
            id: "gpt-4o-mini".into(),
            label: "GPT-4o Mini".into(),
            description: Some("Fast, lightweight daily driver model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("gpt-4o-mini"),
        },
        ModelCatalogOption {
            id: "o1".into(),
            label: "o1".into(),
            description: Some("OpenAI flagship reasoning model for complex STEM and coding".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(200_000),
            pricing: lookup_model_pricing("o1"),
        },
        ModelCatalogOption {
            id: "o1-mini".into(),
            label: "o1-mini".into(),
            description: Some("Fast, efficient reasoning model for coding and math".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("o1-mini"),
        },
        ModelCatalogOption {
            id: "o1-preview".into(),
            label: "o1-preview".into(),
            description: Some("OpenAI preview reasoning model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("o1-preview"),
        },
        ModelCatalogOption {
            id: "chatgpt-4o-latest".into(),
            label: "ChatGPT-4o Latest".into(),
            description: Some("Dynamic ChatGPT-4o model used in ChatGPT web".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("chatgpt-4o-latest"),
        },
        ModelCatalogOption {
            id: "gpt-4-turbo".into(),
            label: "GPT-4 Turbo".into(),
            description: Some("OpenAI GPT-4 Turbo high-intelligence model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("gpt-4-turbo"),
        },
        ModelCatalogOption {
            id: "gpt-3.5-turbo".into(),
            label: "gpt-3.5-turbo".into(),
            description: Some("OpenAI legacy fast chat model".into()),
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(16_385),
            pricing: lookup_model_pricing("gpt-3.5-turbo"),
        },
        // DeepSeek
        ModelCatalogOption {
            id: "deepseek-chat".into(),
            label: "DeepSeek V3".into(),
            description: Some("General intelligence MoE model".into()),
            provider_type: "deepseek".into(),
            is_default: true,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(64_000),
            pricing: lookup_model_pricing("deepseek-chat"),
        },
        ModelCatalogOption {
            id: "deepseek-reasoner".into(),
            label: "DeepSeek R1".into(),
            description: Some("Reinforcement learning reasoning model".into()),
            provider_type: "deepseek".into(),
            is_default: false,
            default_effort: Some("high".into()),
            efforts: vec!["medium".into(), "high".into()],
            supports_fast_mode: false,
            context_window: Some(64_000),
            pricing: lookup_model_pricing("deepseek-reasoner"),
        },
        ModelCatalogOption {
            id: "deepseek-coder".into(),
            label: "deepseek-coder".into(),
            description: Some("DeepSeek coding model".into()),
            provider_type: "deepseek".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(64_000),
            pricing: lookup_model_pricing("deepseek-coder"),
        },
        // Google Gemini
        ModelCatalogOption {
            id: "gemini-2.5-pro".into(),
            label: "Gemini 2.5 Pro".into(),
            description: Some("1M+ token context multi-modal reasoning engine".into()),
            provider_type: "gemini".into(),
            is_default: true,
            default_effort: Some("medium".into()),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-2.5-pro"),
        },
        ModelCatalogOption {
            id: "gemini-2.5-flash".into(),
            label: "Gemini 2.5 Flash".into(),
            description: Some("Ultra-low latency multimodal model".into()),
            provider_type: "gemini".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-2.5-flash"),
        },
        ModelCatalogOption {
            id: "gemini-2.5-flash-lite".into(),
            label: "gemini-2.5-flash-lite".into(),
            description: Some("Google Gemini low-latency model".into()),
            provider_type: "gemini".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-2.5-flash-lite"),
        },
        ModelCatalogOption {
            id: "gemini-2.0-flash".into(),
            label: "gemini-2.0-flash".into(),
            description: Some("Google Gemini 2.0 Flash model".into()),
            provider_type: "gemini".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-2.0-flash"),
        },
        ModelCatalogOption {
            id: "gemini-1.5-pro".into(),
            label: "gemini-1.5-pro".into(),
            description: Some("Google Gemini 1.5 Pro model".into()),
            provider_type: "gemini".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-1.5-pro"),
        },
        ModelCatalogOption {
            id: "gemini-1.5-flash".into(),
            label: "gemini-1.5-flash".into(),
            description: Some("Google Gemini 1.5 Flash model".into()),
            provider_type: "gemini".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(1_048_576),
            pricing: lookup_model_pricing("gemini-1.5-flash"),
        },
        // Local / Ollama
        ModelCatalogOption {
            id: "qwen2.5-coder:32b".into(),
            label: "Qwen 2.5 Coder 32B (Local)".into(),
            description: Some("Local high performance code generation".into()),
            provider_type: "local".into(),
            is_default: true,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(32_768),
            pricing: lookup_model_pricing("qwen2.5-coder:32b"),
        },
        ModelCatalogOption {
            id: "llama3.3:70b".into(),
            label: "Llama 3.3 70B (Local)".into(),
            description: Some("Open weights foundational model".into()),
            provider_type: "local".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: true,
            context_window: Some(128_000),
            pricing: lookup_model_pricing("llama3.3:70b"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_service_filtering() {
        let svc = ModelCatalogService::new();
        let all = svc.get_catalog(None);
        assert!(!all.models.is_empty());

        let anthropic = svc.get_catalog(Some("anthropic"));
        assert!(anthropic
            .models
            .iter()
            .all(|m| m.provider_type == "anthropic"));
        assert!(anthropic.models.iter().any(|m| m.id == "claude-3-7-sonnet"));
    }

    #[test]
    fn test_catalog_merge_probed() {
        let svc = ModelCatalogService::new();
        let probed = vec![ProbedModel {
            id: "my-custom-model:8b".into(),
            name: Some("Custom 8B".into()),
            context_window: Some(65536),
            owned_by: Some("ollama".into()),
            description: None,
        }];

        let result = svc.merge_probed_models("local", probed);
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].id, "my-custom-model:8b");
        assert_eq!(result.models[0].context_window, Some(65536));
        assert_eq!(result.origin, "probe");
    }

    #[test]
    fn test_openai_catalog_contains_real_models() {
        let svc = ModelCatalogService::new();
        let openai = svc.get_catalog(Some("openai"));
        assert!(!openai.models.is_empty());

        let ids: Vec<&str> = openai.models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"gpt-4o"));
        assert!(ids.contains(&"o1"));
        assert!(ids.contains(&"o1-mini"));
        assert!(ids.contains(&"chatgpt-4o-latest"));
        assert!(!ids.contains(&"gpt-4.1"));
        assert!(!ids.contains(&"o4-mini"));
    }
}
