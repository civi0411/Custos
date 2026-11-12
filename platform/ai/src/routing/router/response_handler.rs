use custos_domain::DomainError;
use custos_provider::ProviderRequest;

use super::provider::RouterModelProvider;

impl RouterModelProvider {
    pub(crate) async fn checked_response(
        &self,
        result: Result<reqwest::Response, reqwest::Error>,
        req: &ProviderRequest,
    ) -> Result<reqwest::Response, DomainError> {
        let account = req.provider_account_id.as_deref();
        let mut response = match result {
            Ok(response) => response,
            Err(_) => {
                if let Some(account) = account {
                    self.resolver
                        .observe_response(account, &req.model, 0, "", None)
                        .await?;
                }
                let label = if req.model.starts_with("ollama/") {
                    "Ollama"
                } else {
                    "Provider"
                };
                return Err(DomainError::Validation(format!(
                    "{label} connection failed; refresh models to reconnect"
                )));
            }
        };
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let retry_at = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|value| {
                value
                    .parse::<i64>()
                    .ok()
                    .map(|seconds| {
                        chrono::Utc::now()
                            .timestamp_millis()
                            .saturating_add(seconds.max(0).saturating_mul(1000))
                    })
                    .or_else(|| {
                        chrono::DateTime::parse_from_rfc2822(value)
                            .ok()
                            .map(|date| date.timestamp_millis())
                    })
            });
        let mut body = Vec::new();
        while body.len() < 16_384 {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let remaining = 16_384 - body.len();
                    body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                }
                _ => break,
            }
        }
        let body = String::from_utf8_lossy(&body);
        if let Some(account) = account {
            self.resolver
                .observe_response(account, &req.model, status, &body, retry_at)
                .await?;
        }
        tracing::debug!(task_id = %req.task_id, run_id = %req.request_id, status, "Provider request rejected");
        Err(DomainError::Validation(format!(
            "Provider request rejected (HTTP {status}); refresh models or inspect account limits"
        )))
    }

    pub(crate) async fn observe_adapter_error<T>(
        &self,
        result: Result<T, DomainError>,
        req: &ProviderRequest,
    ) -> Result<T, DomainError> {
        if let (Err(error), Some(account)) = (&result, req.provider_account_id.as_deref()) {
            let message = error.to_string();
            let status = message
                .split("returned ")
                .nth(1)
                .and_then(|part| part.split_whitespace().next())
                .and_then(|part| part.trim_end_matches(':').parse::<u16>().ok())
                .or_else(|| {
                    (message.contains("429") || message.contains("RESOURCE_EXHAUSTED"))
                        .then_some(429)
                })
                .or_else(|| message.contains("connection failed").then_some(0));
            if let Some(status) = status {
                self.resolver
                    .observe_response(account, &req.model, status, &message, None)
                    .await?;
            }
        }
        result
    }

    pub(crate) async fn resolve_token_with_fallback(
        &self,
        provider_id: &str,
    ) -> Result<Option<String>, DomainError> {
        if let Some(tok) = self.resolver.get_token(provider_id).await? {
            if !tok.trim().is_empty() {
                return Ok(Some(tok.trim().to_string()));
            }
        }
        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "groq" => "GROQ_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "together" => "TOGETHER_API_KEY",
            "mistral" => "MISTRAL_API_KEY",
            "cerebras" => "CEREBRAS_API_KEY",
            "perplexity" => "PERPLEXITY_API_KEY",
            "nvidia" => "NVIDIA_API_KEY",
            "moonshot" | "kimi" => "MOONSHOT_API_KEY",
            "zhipu" | "zai" => "ZHIPU_API_KEY",
            "minimax" => "MINIMAX_API_KEY",
            "alibaba" | "qwen" => "DASHSCOPE_API_KEY",
            "xai" => "XAI_API_KEY",
            "sambanova" => "SAMBANOVA_API_KEY",
            "fireworks" => "FIREWORKS_API_KEY",
            "siliconflow" => "SILICONFLOW_API_KEY",
            "nebius" => "NEBIUS_API_KEY",
            "hyperbolic" => "HYPERBOLIC_API_KEY",
            "cohere" => "COHERE_API_KEY",
            _ => return Ok(None),
        };
        Ok(std::env::var(env_var).ok().filter(|s| !s.trim().is_empty()))
    }
}
