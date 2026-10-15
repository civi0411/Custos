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

            let default_effort = pm.default_effort;
            let efforts = pm.efforts;

            models.push(ModelCatalogOption {
                id: pm.id,
                label,
                description: pm.description,
                provider_type: provider_type.to_string(),
                is_default,
                default_effort,
                efforts,
                supports_fast_mode: false,
                context_window: pm.context_window,
                pricing,
            });
        }

        let result = ModelCatalogResult {
            origin: "probe".into(),
            models,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        };

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
            description: Some(
                "Ultra-fast low-latency coding model for real-time iterations".into(),
            ),
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
            ..Default::default()
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

/// Transient account-scoped inference admission; discovery never clears quota exhaustion.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ModelUnavailable {
    pub reason: String,
    pub kind: String,
    pub retry_at: Option<i64>,
}

#[derive(Clone, Default)]
pub struct ModelAvailability {
    entries: Arc<RwLock<HashMap<(String, String), ModelUnavailable>>>,
}

impl ModelAvailability {
    pub fn snapshot(&self, scope: &str) -> Result<HashMap<String, ModelUnavailable>, custos_domain::DomainError> {
        let now = chrono::Utc::now().timestamp_millis();
        let entries = self.entries.read().map_err(|_| custos_domain::DomainError::Validation("Model availability lock unavailable".into()))?;
        Ok(entries.iter().filter(|((key, _), value)| key == scope && value.retry_at.is_none_or(|time| time > now))
            .map(|((_, model), value)| (model.clone(), value.clone())).collect())
    }

    pub fn check(&self, scope: &str, model: &str) -> Result<(), custos_domain::DomainError> {
        let blocked = self.snapshot(scope)?;
        if let Some(reason) = blocked.get("*").or_else(|| blocked.get(model)) {
            return Err(custos_domain::DomainError::Validation(format!("Model unavailable: {}", reason.reason)));
        }
        Ok(())
    }

    pub fn refresh(&self, scope: &str) -> Result<(), custos_domain::DomainError> {
        let mut entries = self.entries.write().map_err(|_| custos_domain::DomainError::Validation("Model availability lock unavailable".into()))?;
        entries.retain(|(key, _), value| key != scope || matches!(value.kind.as_str(), "quota" | "rate"));
        Ok(())
    }

    pub fn quota_recovered(&self, scope: &str) -> Result<(), custos_domain::DomainError> {
        self.entries.write().map_err(|_| custos_domain::DomainError::Validation("Model availability lock unavailable".into()))?
            .retain(|(key, _), value| key != scope || value.kind != "quota");
        Ok(())
    }

    pub fn observe(&self, scope: &str, model: &str, status: u16, body: &str, retry_at: Option<i64>) -> Result<(), custos_domain::DomainError> {
        let lower = body.to_ascii_lowercase();
        let value = serde_json::from_str::<serde_json::Value>(body).ok();
        let retry_at = value.as_ref().and_then(|value| value.pointer("/error/resets_at").or_else(|| value.pointer("/error/reset_at"))).and_then(|reset| {
            reset.as_i64().map(|timestamp| if timestamp < 10_000_000_000 { timestamp.saturating_mul(1000) } else { timestamp })
                .or_else(|| reset.as_str().and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok()).map(|time| time.timestamp_millis()))
        }).or(retry_at);
        let code = value.as_ref().and_then(|v| v.pointer("/error/code").filter(|value| value.is_string()).or_else(|| v.pointer("/error/type")).or_else(|| v.get("code"))).and_then(|v| v.as_str()).unwrap_or("");
        let (target, kind, reason) = if status == 0 {
            ("*", "offline", "Provider connection unavailable; refresh models to reconnect")
        } else if status == 401 {
            ("*", "credentials", "Provider authentication failed; reconnect this account")
        } else if status == 429 && (matches!(code, "insufficient_quota" | "credit_balance_exhausted" | "quota_exhausted" | "usage_limit_reached") || lower.contains("quota exhausted") || lower.contains("usage limit") || lower.contains("insufficient_quota")) {
            ("*", "quota", "Provider quota exhausted; refresh quota or wait for reset")
        } else if status == 429 {
            (model, "rate", "Model temporarily rate limited")
        } else if matches!(status, 403 | 404) && (matches!(code, "model_not_found" | "model_not_supported" | "model_access_denied") || lower.contains("model not found") || lower.contains("model is not supported") || lower.contains("do not have access to model")) {
            (model, "access", "This account cannot access the model; refresh models")
        } else if status == 403 {
            ("*", "access", "Provider denied access; refresh models or reconnect")
        } else { return Ok(()); };
        let retry_at = if kind == "rate" { Some(retry_at.unwrap_or_else(|| chrono::Utc::now().timestamp_millis() + 30_000)) } else { retry_at };
        self.entries.write().map_err(|_| custos_domain::DomainError::Validation("Model availability lock unavailable".into()))?
            .insert((scope.to_string(), target.to_string()), ModelUnavailable { reason: reason.into(), kind: kind.into(), retry_at });
        Ok(())
    }
}

#[cfg(test)]
mod availability_tests {
    use super::*;
    #[test]
    fn transient_blocks_are_scoped_and_prompt_errors_do_not_hide_models() {
        let state = ModelAvailability::default();
        state.observe("A", "gpt", 400, r#"{"error":{"code":"invalid_reasoning_effort"}}"#, None).unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state.observe("A", "gpt", 404, r#"{"error":{"code":"model_not_found"}}"#, None).unwrap();
        assert!(state.check("A", "gpt").is_err());
        assert!(state.check("B", "gpt").is_ok());
        assert!(state.check("A", "another").is_ok());
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state.observe("A", "gpt", 401, "", None).unwrap();
        assert!(state.check("A", "another").is_err());
        state.refresh("A").unwrap();
        state.observe("A", "gpt", 0, "", None).unwrap();
        assert!(state.check("A", "gpt").is_err());
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
    }
    #[test]
    fn catalog_refresh_does_not_reset_quota_or_retry_window() {
        let state = ModelAvailability::default();
        state.observe("A", "gpt", 429, r#"{"error":{"code":"insufficient_quota"}}"#, None).unwrap();
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_err());
        state.quota_recovered("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state.observe("A", "gpt", 429, r#"{"error":{"code":"rate_limit_exceeded"}}"#, None).unwrap();
        state.refresh("A").unwrap();
        assert_eq!(state.snapshot("A").unwrap()["gpt"].kind, "rate");
        assert!(state.check("A", "another").is_ok());
        state.observe("A", "gpt", 429, "rate", Some(chrono::Utc::now().timestamp_millis()-1)).unwrap();
        assert!(state.check("A", "gpt").is_ok());
    }
    #[test]
    fn merge_preserves_exact_advertised_efforts_without_name_heuristics() {
        let service = ModelCatalogService::new();
        let result = service.merge_probed_models("openai", vec![
            ProbedModel { id:"gpt-6-sol".into(), efforts:vec!["xhigh".into(),"max".into(),"ultra".into()], default_effort:Some("max".into()), ..Default::default() },
            ProbedModel { id:"sonnet-pro-thinking".into(), ..Default::default() },
        ]);
        assert_eq!(result.models[0].efforts, vec!["xhigh","max","ultra"]);
        assert_eq!(result.models[0].default_effort.as_deref(), Some("max"));
        assert!(result.models[1].efforts.is_empty());
        assert!(!service.get_catalog(Some("openai")).models.iter().any(|model| model.id == "sonnet-pro-thinking"));
    }
}
