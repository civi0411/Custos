//! LocalModelProvider Adapter
//!
//! Provides local LLM inference capabilities (llama.cpp / MLX / local OpenAI-compatible runners)
//! integrated into Custos via ModelProvider trait.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::port::ModelProvider;
use custos_provider::request::{ModelResponse, ProviderRequest};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct LocalModelProvider {
    provider_id: String,
    endpoint_url: String,
    http_client: Client,
}

impl LocalModelProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "local".to_string(),
            endpoint_url: "http://127.0.0.1:11434/v1".to_string(), // Default Ollama / llama-server
            http_client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            provider_id: "local".to_string(),
            endpoint_url: endpoint.trim_end_matches('/').to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_model(model_name: &str) -> Self {
        Self {
            provider_id: format!("local:{}", model_name),
            endpoint_url: "http://127.0.0.1:11434/v1".to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for LocalModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct ChatCompletionMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatCompletionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: Option<ChatCompletionMessageContent>,
}

#[derive(Deserialize)]
struct ChatCompletionMessageContent {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
    usage: Option<ChatCompletionUsage>,
}

#[derive(Deserialize)]
struct ChatCompletionUsage {
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
}

#[async_trait]
impl ModelProvider for LocalModelProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let endpoint = format!("{}/chat/completions", self.endpoint_url.trim_end_matches('/'));
        let model_name = if req.model.is_empty() || req.model == "default" {
            "default".to_string()
        } else {
            req.model.clone()
        };

        let payload = ChatCompletionRequest {
            model: model_name.clone(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }],
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: false,
        };

        info!(
            model = %model_name,
            endpoint = %endpoint,
            "Executing local model inference request"
        );

        let resp = self
            .http_client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                DomainError::Validation(format!(
                    "Local runner transport error connecting to {}: {}",
                    endpoint, e
                ))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Validation(format!(
                "Local runner returned error (status {}): {}",
                status,
                body
            )));
        }

        let parsed: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to parse local runner JSON: {}", e)))?;

        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        let input_tokens = parsed
            .usage
            .as_ref()
            .and_then(|u| u.prompt_tokens)
            .unwrap_or(0);
        let output_tokens = parsed
            .usage
            .as_ref()
            .and_then(|u| u.completion_tokens)
            .unwrap_or(0);

        Ok(ModelResponse {
            content,
            tokens_used: input_tokens + output_tokens,
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            total_tokens: Some(input_tokens + output_tokens),
            model_id: model_name,
            tool_calls: Vec::new(),
        })
    }
}
