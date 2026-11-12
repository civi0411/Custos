//! Daemon Credential Resolver implementing Provider routing trait.

use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;
use custos_provider::routing::router::CredentialResolver;
use std::sync::Arc;

use super::selection::{provider_catalog_scope, reserve_available_provider_account};

pub struct DaemonCredentialResolver {
    pub availability: custos_provider::routing::availability::ModelAvailability,
    pub store: Arc<SqliteTaskStore>,
    pub credentials: Arc<crate::credentials::ProviderCredentials>,
}

#[async_trait::async_trait]
impl CredentialResolver for DaemonCredentialResolver {
    async fn select_account(
        &self,
        service: &str,
        model: &str,
    ) -> Result<Option<String>, DomainError> {
        reserve_available_provider_account(
            self.store.providers(),
            service,
            model,
            None,
            &self.availability,
        )
    }

    async fn validate_account(&self, account_id: &str, service: &str) -> Result<(), DomainError> {
        let record = self
            .store
            .providers()
            .get_provider(account_id)?
            .ok_or_else(|| {
                DomainError::Validation("Selected provider account no longer exists".into())
            })?;
        if !matches!(
            record.status.as_str(),
            "active" | "configured" | "primary" | "standby"
        ) {
            return Err(DomainError::Validation(
                "Selected provider account is inactive".into(),
            ));
        }
        let matches_service = record.service_type == service
            || (matches!(service, "antigravity" | "gemini")
                && matches!(record.service_type.as_str(), "antigravity" | "gemini"));
        if !matches_service {
            return Err(DomainError::Validation(
                "Selected account does not belong to the requested model provider".into(),
            ));
        }
        if !matches!(service, "local" | "ollama" | "lmstudio" | "custom")
            && self
                .credentials
                .get_oauth_token(account_id)
                .await?
                .is_none()
        {
            return Err(DomainError::Validation(
                "secure_storage_missing: Reconnect this provider account".into(),
            ));
        }
        Ok(())
    }

    async fn service_for_account(&self, account: &str) -> Result<Option<String>, DomainError> {
        let service = self
            .store
            .providers()
            .get_provider(account)?
            .map(|record| record.service_type);
        Ok(service)
    }

    async fn validate_model(
        &self,
        account: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<(), DomainError> {
        let scope = provider_catalog_scope(self.store.providers(), account)?;
        self.availability.check(&scope, model)?;
        let models = self.store.providers().list_catalog_models(Some(&scope))?;
        let selected = models
            .iter()
            .find(|entry| entry.id == model)
            .ok_or_else(|| {
                DomainError::Validation(
                    "Selected account no longer advertises this model; refresh models".into(),
                )
            })?;
        if !custos_provider::routing::probe::conversation_model_is_visible(
            &selected.id,
            Some(&selected.label),
        ) {
            return Err(DomainError::Validation(
                "This model is hidden from the conversation catalog; select another model".into(),
            ));
        }
        if let Some(effort) = effort {
            if !selected.efforts.iter().any(|value| value == effort) {
                return Err(DomainError::Validation(
                    "Reasoning effort is not supported by this account/model".into(),
                ));
            }
        }
        Ok(())
    }

    async fn observe_response(
        &self,
        account: &str,
        model: &str,
        status: u16,
        body: &str,
        retry_at: Option<i64>,
    ) -> Result<(), DomainError> {
        let scope = provider_catalog_scope(self.store.providers(), account)?;
        self.availability
            .observe(&scope, model, status, body, retry_at)
    }

    async fn uses_oauth(&self, provider_id: &str) -> Result<bool, DomainError> {
        Ok(self
            .credentials
            .get_oauth_token(provider_id)
            .await?
            .is_some_and(|tok| {
                tok.refresh_token.is_some()
                    || tok.scope.as_deref().is_some_and(|s| !s.trim().is_empty())
            }))
    }

    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        if let Some(tok) = self.credentials.get_oauth_token(provider_id).await? {
            if tok.is_expired(120) {
                if tok.refresh_token.is_none() {
                    return Err(DomainError::Validation(
                        "Provider credentials expired; reconnect the account".into(),
                    ));
                }
                let settings =
                    custos_adapters::connectors::oauth::oauth_settings(&tok.service_type)?;
                let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
                    &settings.0,
                    &settings.1,
                );
                let refreshed = manager.refresh_token(&settings.2, &tok).await?;
                self.credentials.save_refreshed(&tok, &refreshed).await?;
                return Ok(Some(refreshed.access_token));
            }
            return Ok(Some(tok.access_token));
        }

        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            _ => "",
        };
        if !env_var.is_empty() {
            if let Ok(val) = std::env::var(env_var) {
                if !val.trim().is_empty() {
                    return Ok(Some(val.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    async fn get_endpoint_url(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        let repo = self.store.providers();
        if let Some(prov) = repo.get_provider(provider_id)? {
            if let Some(endpoint) = prov.endpoint_url {
                if !endpoint.trim().is_empty() {
                    return Ok(Some(endpoint.trim().to_string()));
                }
            }
        }
        Ok(None)
    }
}
