use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelResponse, ProviderRequest};
use std::sync::Arc;

use super::classification::classify_provider;
use super::credentials::{CredentialResolver, EnvCredentialResolver};

#[derive(Clone)]
pub struct RouterModelProvider {
    pub(crate) resolver: Arc<dyn CredentialResolver>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) codex_endpoint: String,
    pub(crate) native_local: Option<Arc<dyn custos_provider::ModelPort>>,
}

impl RouterModelProvider {
    pub fn new(resolver: Arc<dyn CredentialResolver>) -> Self {
        Self {
            resolver,
            codex_endpoint: "https://chatgpt.com/backend-api/codex/responses".into(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            native_local: None,
        }
    }

    pub fn with_native_local(mut self, native_local: Arc<dyn custos_provider::ModelPort>) -> Self {
        self.native_local = Some(native_local);
        self
    }

    pub fn with_env_fallback() -> Self {
        Self::new(Arc::new(EnvCredentialResolver))
    }

    /// Determines the provider service type from a given model identifier.
    pub fn classify_provider(model: &str) -> &'static str {
        classify_provider(model)
    }

    #[cfg(test)]
    pub(crate) fn responses_payload(req: &ProviderRequest, model: &str) -> serde_json::Value {
        super::classification::responses_payload(req, model)
    }
}

#[async_trait]
impl ModelProvider for RouterModelProvider {
    fn provider_id(&self) -> &str {
        "unified-router"
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let mut routed = req.clone();
        let service = Self::classify_provider(&routed.model);
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        if service == "native-local" {
            routed.provider_account_id = None;
        } else if routed.provider_account_id.is_none() {
            routed.provider_account_id = self
                .resolver
                .select_account(candidate_service, &routed.model)
                .await?;
        }

        let max_retries = 3;
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.dispatch_stream_attempt(&routed).await {
                Ok(rx) => return Ok(rx),
                Err(err) => {
                    let err_str = err.to_string();
                    let is_rate_or_quota = err_str.contains("429")
                        || err_str.contains("RESOURCE_EXHAUSTED")
                        || err_str.contains("quota exhausted")
                        || err_str.contains("rate limited")
                        || err_str.contains("temporary rate limit")
                        || err_str.contains("Model unavailable");

                    if is_rate_or_quota && attempts < max_retries && service != "native-local" {
                        if let Ok(Some(next_account)) = self
                            .resolver
                            .select_account(candidate_service, &routed.model)
                            .await
                        {
                            if routed.provider_account_id.as_deref() != Some(&next_account) {
                                tracing::info!(
                                    service,
                                    model = %routed.model,
                                    previous_account = ?routed.provider_account_id,
                                    failover_account = %next_account,
                                    "Rate limit/429 encountered; auto-failing over to next available account in pool"
                                );
                                routed.provider_account_id = Some(next_account);
                                continue;
                            }
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut routed = req.clone();
        let service = Self::classify_provider(&routed.model);
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        if service == "native-local" {
            routed.provider_account_id = None;
        } else if routed.provider_account_id.is_none() {
            routed.provider_account_id = self
                .resolver
                .select_account(candidate_service, &routed.model)
                .await?;
        }

        let max_retries = 3;
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.dispatch_generate_attempt(&routed).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let err_str = err.to_string();
                    let is_rate_or_quota = err_str.contains("429")
                        || err_str.contains("RESOURCE_EXHAUSTED")
                        || err_str.contains("quota exhausted")
                        || err_str.contains("rate limited")
                        || err_str.contains("temporary rate limit")
                        || err_str.contains("Model unavailable");

                    if is_rate_or_quota && attempts < max_retries && service != "native-local" {
                        if let Ok(Some(next_account)) = self
                            .resolver
                            .select_account(candidate_service, &routed.model)
                            .await
                        {
                            if routed.provider_account_id.as_deref() != Some(&next_account) {
                                tracing::info!(
                                    service,
                                    model = %routed.model,
                                    previous_account = ?routed.provider_account_id,
                                    failover_account = %next_account,
                                    "Rate limit/429 encountered; auto-failing over to next available account in pool"
                                );
                                routed.provider_account_id = Some(next_account);
                                continue;
                            }
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn execute_turn(
        &self,
        req: custos_provider::turn::ModelTurnRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::turn::ModelTurnEvent>, DomainError>
    {
        let service = Self::classify_provider(&req.requested_model);
        if service == "native-local" {
            let local = self.native_local.as_ref().ok_or_else(|| {
                DomainError::Validation(
                    "Native local inference runtime is not configured".to_string(),
                )
            })?;
            return local.execute_turn(req).await;
        }

        let flat_prompt = req
            .structured_messages
            .iter()
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|c| match c {
                        custos_provider::conversation::message::MessageContentBlock::Text(t) => {
                            Some(t.text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut provider_req = custos_provider::request::ProviderRequest::new(
            req.attempt_id.to_string(),
            req.task_id.to_string(),
            1,
            flat_prompt,
            req.requested_model.clone(),
        );
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        provider_req.provider_account_id = self
            .resolver
            .select_account(candidate_service, &provider_req.model)
            .await?;

        match self.send_stream_request(&provider_req).await {
            Ok((resp, wire, actual_provider, actual_model)) => {
                Ok(crate::routing::streaming::stream_turn_response_observed(
                    resp,
                    req.attempt_id,
                    actual_provider,
                    actual_model,
                    wire,
                    std::time::Duration::from_secs(45),
                ))
            }
            Err(err) => {
                let (tx, rx) = tokio::sync::mpsc::channel(2);
                let attempt_id = req.attempt_id;
                let provider_id = self.provider_id().to_string();
                let requested_model = req.requested_model.clone();
                tokio::spawn(async move {
                    let _ = tx
                        .send(custos_provider::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 1,
                            actual_provider: Some(provider_id),
                            actual_model: Some(requested_model),
                            actual_transport: None,
                            delta: custos_provider::turn::TurnDelta::Failed {
                                error_class: "router_error".to_string(),
                                message: err.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        })
                        .await;
                });
                Ok(rx)
            }
        }
    }
}
