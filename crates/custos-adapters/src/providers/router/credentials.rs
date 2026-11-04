//! Credential resolution contracts for RouterModelProvider

use async_trait::async_trait;
use custos_domain::DomainError;

#[async_trait]
pub trait CredentialResolver: Send + Sync {
    async fn select_account(&self, _service: &str, _model: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn service_for_account(&self, _account: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn validate_model(&self, _account: &str, _model: &str, _effort: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn observe_response(&self, _account: &str, _model: &str, _status: u16, _body: &str, _retry_at: Option<i64>) -> Result<(), DomainError> { Ok(()) }
    /// Return access token or API key for a given provider
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError>;
    async fn uses_oauth(&self, _provider_id: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn validate_account(&self, _account_id: &str, _service: &str) -> Result<(), DomainError> { Ok(()) }
    /// Return custom endpoint URL for a given provider
    async fn get_endpoint_url(&self, provider_id: &str) -> Result<Option<String>, DomainError>;
}

/// Fallback resolver that reads environment variables
pub struct EnvCredentialResolver;

#[async_trait]
impl CredentialResolver for EnvCredentialResolver {
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            _ => return Ok(None),
        };
        Ok(std::env::var(env_var).ok().filter(|s| !s.trim().is_empty()))
    }

    async fn get_endpoint_url(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}
