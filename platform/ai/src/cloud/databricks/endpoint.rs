use std::collections::HashSet;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::base::ModelInfo;
use crate::errors::ProviderError;
use crate::cloud::codecs::databricks::DATABRICKS_PROVIDER_NAME;
use crate::model::ModelConfig;

use super::types::{
    CachedDatabricksEndpointInfo, DatabricksEndpointInfo, DatabricksProvider,
    DatabricksUpstreamModel, EndpointMetadataLookup, DATABRICKS_ENDPOINT_INFO_CACHE,
    DATABRICKS_ENDPOINT_METADATA_TIMEOUT_SECS,
};

impl DatabricksProvider {
    pub(crate) fn endpoint_model_candidates(value: &Value) -> Vec<DatabricksUpstreamModel> {
        let mut candidates: Vec<DatabricksUpstreamModel> = Vec::new();

        fn get_string_at(value: &Value, path: &[&str]) -> Option<String> {
            path.iter()
                .try_fold(value, |current, key| current.get(*key))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        }

        fn push_candidate(
            name: Option<String>,
            provider: Option<String>,
            candidates: &mut Vec<DatabricksUpstreamModel>,
        ) {
            if let Some(name) = name {
                if !candidates.iter().any(|candidate| candidate.name == name) {
                    candidates.push(DatabricksUpstreamModel { name, provider });
                }
            }
        }

        for config_key in ["config", "pending_config"] {
            let Some(config) = value.get(config_key) else {
                continue;
            };

            for collection_key in ["served_entities", "served_models"] {
                let Some(entities) = config.get(collection_key).and_then(|v| v.as_array()) else {
                    continue;
                };

                for entity in entities {
                    push_candidate(
                        get_string_at(entity, &["external_model", "name"]),
                        get_string_at(entity, &["external_model", "provider"]),
                        &mut candidates,
                    );
                    push_candidate(
                        get_string_at(entity, &["foundation_model", "name"]),
                        get_string_at(entity, &["foundation_model", "provider"]),
                        &mut candidates,
                    );
                    push_candidate(
                        get_string_at(entity, &["entity_name"]),
                        None,
                        &mut candidates,
                    );
                }
            }
        }

        candidates
    }

    pub(crate) fn endpoint_info_from_value(endpoint: &Value) -> Option<DatabricksEndpointInfo> {
        let name = endpoint.get("name")?.as_str()?.to_string();
        let supports_responses_api = Self::endpoint_supports_responses_api(endpoint);
        let upstream_model = Self::endpoint_model_candidates(endpoint)
            .into_iter()
            .find(|candidate| candidate.name != name);
        let upstream_model_name = upstream_model.as_ref().map(|model| model.name.clone());
        let upstream_model_provider = upstream_model.and_then(|model| model.provider);

        let reasoning = upstream_model_name
            .as_deref()
            .map(Self::is_reasoning_capable_model_name)
            .or_else(|| Some(Self::is_reasoning_capable_model_name(&name)));

        Some(DatabricksEndpointInfo {
            name,
            upstream_model_name,
            upstream_model_provider,
            reasoning,
            supports_responses_api,
        })
    }

    pub(crate) fn endpoint_supports_responses_api(endpoint: &Value) -> bool {
        fn value_contains_responses_api(value: &Value) -> bool {
            match value {
                Value::Object(map) => {
                    map.get("api_types")
                        .and_then(|api_types| api_types.as_array())
                        .is_some_and(|api_types| {
                            api_types
                                .iter()
                                .any(|api_type| api_type.as_str() == Some("openai/v1/responses"))
                        })
                        || map.values().any(value_contains_responses_api)
                }
                Value::Array(values) => values.iter().any(value_contains_responses_api),
                _ => false,
            }
        }

        let Some(config) = endpoint.get("config") else {
            return false;
        };

        for collection_key in ["served_entities", "served_models"] {
            let Some(entities) = config.get(collection_key).and_then(|v| v.as_array()) else {
                continue;
            };

            if entities.iter().any(value_contains_responses_api) {
                return true;
            }
        }

        false
    }

    pub(crate) async fn fetch_endpoint_info(
        &self,
        endpoint_name: &str,
    ) -> Result<DatabricksEndpointInfo, ProviderError> {
        let response = self
            .api_client
            .request(&format!(
                "api/2.0/serving-endpoints/{}",
                urlencoding::encode(endpoint_name)
            ))
            .response_get()
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to fetch Databricks endpoint metadata: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Failed to fetch Databricks endpoint metadata: {} {}",
                status, detail
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse Databricks endpoint metadata: {}",
                e
            ))
        })?;

        Self::endpoint_info_from_value(&json).ok_or_else(|| {
            ProviderError::RequestFailed(
                "Unexpected response format from Databricks endpoint metadata".to_string(),
            )
        })
    }

    pub(crate) async fn resolve_endpoint_info(
        &self,
        endpoint_name: &str,
    ) -> Result<DatabricksEndpointInfo, ProviderError> {
        const MAX_MODEL_SERVING_HOPS: usize = 4;

        let original_endpoint_name = endpoint_name.to_string();
        let mut current_endpoint_name = endpoint_name.to_string();
        let mut visited = HashSet::new();
        let mut last_info: Option<DatabricksEndpointInfo> = None;
        let mut first_hop_supports_responses_api: Option<bool> = None;

        for _ in 0..MAX_MODEL_SERVING_HOPS {
            if !visited.insert(current_endpoint_name.clone()) {
                break;
            }

            let info = self.fetch_endpoint_info(&current_endpoint_name).await?;
            let supports_responses_api =
                *first_hop_supports_responses_api.get_or_insert(info.supports_responses_api);
            let next_endpoint_name = match (
                info.upstream_model_provider.as_deref(),
                info.upstream_model_name.as_deref(),
            ) {
                (Some("databricks-model-serving"), Some(next_endpoint_name))
                    if !visited.contains(next_endpoint_name) =>
                {
                    Some(next_endpoint_name.to_string())
                }
                _ => None,
            };

            if let Some(next_endpoint_name) = next_endpoint_name {
                last_info = Some(info);
                current_endpoint_name = next_endpoint_name;
                continue;
            }

            let mut resolved_info = if info.name == original_endpoint_name {
                info
            } else {
                let upstream_model_name = info
                    .upstream_model_name
                    .clone()
                    .or_else(|| Some(info.name.clone()));
                DatabricksEndpointInfo {
                    name: original_endpoint_name,
                    upstream_model_name,
                    upstream_model_provider: info.upstream_model_provider.clone(),
                    reasoning: info.reasoning,
                    supports_responses_api,
                }
            };
            resolved_info.supports_responses_api = supports_responses_api;
            return Ok(resolved_info);
        }

        last_info
            .map(|info| DatabricksEndpointInfo {
                name: original_endpoint_name,
                upstream_model_name: info.upstream_model_name,
                upstream_model_provider: info.upstream_model_provider,
                reasoning: info.reasoning,
                supports_responses_api: first_hop_supports_responses_api.unwrap_or(false),
            })
            .ok_or_else(|| {
                ProviderError::RequestFailed(
                    "Failed to resolve Databricks endpoint metadata".to_string(),
                )
            })
    }

    pub(crate) async fn resolve_endpoint_info_cached(
        &self,
        endpoint_name: &str,
        lookup: EndpointMetadataLookup,
    ) -> Result<DatabricksEndpointInfo, ProviderError> {
        let cache_key = format!("{}:{}", self.host, endpoint_name);
        let cached = DATABRICKS_ENDPOINT_INFO_CACHE
            .lock()
            .unwrap()
            .get(&cache_key)
            .cloned();

        if let Some(cached) = cached {
            if cached.applies_to(lookup) {
                return cached.info.ok_or_else(|| {
                    ProviderError::RequestFailed(
                        "Databricks endpoint metadata is unavailable".to_string(),
                    )
                });
            }
        }

        let info = tokio::time::timeout(
            Duration::from_secs(DATABRICKS_ENDPOINT_METADATA_TIMEOUT_SECS),
            self.resolve_endpoint_info(endpoint_name),
        )
        .await
        .ok()
        .and_then(Result::ok);
        DATABRICKS_ENDPOINT_INFO_CACHE.lock().unwrap().insert(
            cache_key,
            CachedDatabricksEndpointInfo {
                info: info.clone(),
                fetched_at: Instant::now(),
            },
        );
        info.ok_or_else(|| {
            ProviderError::RequestFailed("Databricks endpoint metadata is unavailable".to_string())
        })
    }

    pub(crate) fn model_info_from_endpoint(info: DatabricksEndpointInfo) -> ModelInfo {
        let context_model = info.upstream_model_name.as_deref().unwrap_or(&info.name);
        let context_limit =
            crate::catalog::canonical::maybe_get_canonical_model(DATABRICKS_PROVIDER_NAME, context_model)
                .map(|model| model.limit.context);
        let reasoning = info
            .reasoning
            .unwrap_or_else(|| ModelConfig::new(context_model).is_reasoning_model());

        ModelInfo {
            name: info.name,
            resolved_model: info.upstream_model_name,
            context_limit,
            input_token_cost: None,
            output_token_cost: None,
            currency: None,
            supports_cache_control: None,
            reasoning,
            thinking_preservation_format: None,
            request_params: None,
        }
    }
}

