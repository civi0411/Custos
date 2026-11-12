//! Core Provider trait and registry resolution helpers.

use super::metadata::ModelInfo;
use super::stream::{collect_stream, MessageStream, PermissionRouting};
use crate::{
    catalog::canonical::{
        map_to_canonical_model, recommended_models_from_registry, CanonicalModelRegistry,
    },
    conversation::{message::Message, token_usage::ProviderUsage},
    custos_mode::CustosMode,
    errors::ProviderError,
    model::ModelConfig,
    permission::PermissionConfirmation,
    retry::RetryConfig,
    thinking::ThinkingEffortSupport,
};
use async_trait::async_trait;
use rmcp::model::Tool;
use tokio::sync::watch;

pub fn model_info_for_provider_model(provider_name: &str, model_name: &str) -> ModelInfo {
    let registry = CanonicalModelRegistry::bundled().ok();
    let canonical = registry.as_ref().and_then(|registry| {
        let canonical_id = map_to_canonical_model(provider_name, model_name, registry)?;
        let (provider, model) = canonical_id.split_once('/')?;
        registry.get(provider, model)
    });

    let reasoning = canonical
        .as_ref()
        .and_then(|model| model.reasoning)
        .unwrap_or_else(|| ModelConfig::new(model_name).is_reasoning_model());

    ModelInfo {
        name: model_name.to_string(),
        resolved_model: None,
        context_limit: canonical.as_ref().map(|model| model.limit.context),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning,
        thinking_preservation_format: None,
        request_params: None,
    }
}

pub fn known_models_from_registry(provider: &str) -> Vec<ModelInfo> {
    recommended_models_from_registry(provider)
        .into_iter()
        .map(|name| model_info_for_provider_model(provider, &name))
        .collect()
}

/// Base trait for AI providers (OpenAI, Anthropic, etc)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the name of this provider instance
    fn get_name(&self) -> &str;

    fn provider_session_id(&self) -> Option<String> {
        None
    }

    async fn resume(&self, _session_id: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Primary streaming method that all providers must implement.
    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError>;

    async fn complete(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let stream = self.stream(model_config, system, messages, tools).await?;
        collect_stream(stream).await
    }

    /// Resolve the effective context limit for a model.
    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        crate::context_limit::ContextLimitResolver::new(self.get_name())
            .resolve(model, override_limit, || async { Ok(None) })
            .await
    }

    fn retry_config(&self) -> RetryConfig {
        RetryConfig::default()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }

    async fn fetch_supported_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(self
            .fetch_supported_models()
            .await?
            .iter()
            .map(|model_name| model_info_for_provider_model(self.get_name(), model_name))
            .collect())
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        Ok(model_info_for_provider_model(self.get_name(), model_name))
    }

    fn skip_canonical_filtering(&self) -> bool {
        false
    }

    /// Fetch inventory models filtered by canonical registry and usability.
    async fn fetch_recommended_models(&self, toolshim: bool) -> Result<Vec<String>, ProviderError> {
        let all_models = self.fetch_supported_models().await?;

        if self.skip_canonical_filtering() {
            return Ok(all_models);
        }

        let registry = CanonicalModelRegistry::bundled().map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load canonical registry: {}", e))
        })?;

        let provider_name = self.get_name();

        // Get all text-capable models with their release dates
        let mut models_with_dates: Vec<(String, Option<String>)> = all_models
            .iter()
            .filter_map(|model| {
                let canonical_model = map_to_canonical_model(provider_name, model, registry)
                    .and_then(|canonical_id| {
                        let (provider, model_name) = canonical_id.split_once('/')?;
                        registry.get(provider, model_name)
                    });
                let Some(canonical_model) = canonical_model else {
                    return Some((model.clone(), None));
                };

                if !canonical_model
                    .modalities
                    .input
                    .contains(&crate::catalog::canonical::Modality::Text)
                {
                    return None;
                }

                if !canonical_model.tool_call && !toolshim {
                    return None;
                }

                let release_date = canonical_model.release_date.clone();

                Some((model.clone(), release_date))
            })
            .collect();

        // Sort by release date (most recent first), then alphabetically for models without dates
        models_with_dates.sort_by(|a, b| match (&a.1, &b.1) {
            (Some(date_a), Some(date_b)) => date_b.cmp(date_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(&b.0),
        });

        let inventory_models: Vec<String> = models_with_dates
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        if inventory_models.is_empty() {
            Ok(all_models)
        } else {
            Ok(inventory_models)
        }
    }

    async fn fetch_recommended_model_info(
        &self,
        toolshim: bool,
    ) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(self
            .fetch_recommended_models(toolshim)
            .await?
            .iter()
            .map(|model_name| model_info_for_provider_model(self.get_name(), model_name))
            .collect())
    }

    async fn map_to_canonical_model(
        &self,
        provider_model: &str,
    ) -> Result<Option<String>, ProviderError> {
        let registry = CanonicalModelRegistry::bundled().map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load canonical registry: {}", e))
        })?;

        Ok(map_to_canonical_model(
            self.get_name(),
            provider_model,
            registry,
        ))
    }

    /// Whether the provider manages its own conversation context.
    fn manages_own_context(&self) -> bool {
        false
    }

    fn uses_local_session_naming(&self) -> bool {
        self.manages_own_context()
    }

    fn supports_builtin_tools(&self) -> bool {
        !self.manages_own_context()
    }

    /// Configure OAuth authentication for this provider
    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        Err(ProviderError::ExecutionError(
            "OAuth configuration not supported by this provider".to_string(),
        ))
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        Err(ProviderError::NotImplemented(
            "credential refresh not supported by this provider".to_string(),
        ))
    }

    async fn update_mode(&self, _session_id: &str, _mode: CustosMode) -> Result<(), ProviderError> {
        Ok(())
    }

    /// How this provider participates in thinking-effort selection.
    fn thinking_effort_support(&self) -> ThinkingEffortSupport {
        ThinkingEffortSupport::Unspecified
    }

    /// Subscribe to provider-managed thinking-effort capability changes.
    fn subscribe_thinking_effort_support(&self) -> Option<watch::Receiver<ThinkingEffortSupport>> {
        None
    }

    /// Forward a thinking-effort selection to the provider.
    async fn set_thinking_effort(
        &self,
        _session_id: &str,
        _value: &str,
    ) -> Result<bool, ProviderError> {
        Ok(false)
    }

    /// Apply a session's model selection after the provider is installed.
    async fn apply_model_selection(
        &self,
        _model_config: &ModelConfig,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::Noop
    }

    async fn handle_permission_confirmation(
        &self,
        _request_id: &str,
        _confirmation: &PermissionConfirmation,
    ) -> bool {
        false
    }
}
