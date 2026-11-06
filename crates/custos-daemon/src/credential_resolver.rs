//! Provider Credential Resolver & Account Selection
//!
//! Provides account reservation, OAuth credential extraction, model availability observation,
//! and routing policy management for provider accounts.

use custos_adapters::providers::CredentialResolver;
use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

#[cfg(test)]
pub(crate) fn reserve_provider_account(
    repo: &custos_persistence::ProviderRepository,
    service: &str,
    model: &str,
) -> Result<Option<String>, DomainError> {
    reserve_available_provider_account(repo, service, model, None, &Default::default())
}

pub(crate) fn reserve_available_provider_account(
    repo: &custos_persistence::ProviderRepository,
    service: &str,
    model: &str,
    effort: Option<&str>,
    availability: &custos_adapters::providers::catalog::ModelAvailability,
) -> Result<Option<String>, DomainError> {
    let providers = repo.list_providers()?;
    let mut eligible = Vec::new();
    let mut stored = false;
    for account in providers.iter().filter(|p| {
        p.service_type == service
            || (matches!(service, "antigravity" | "gemini")
                && matches!(p.service_type.as_str(), "antigravity" | "gemini"))
    }) {
        stored = true;
        if !matches!(
            account.status.as_str(),
            "active" | "configured" | "primary" | "standby"
        ) {
            continue;
        }
        let metadata = repo.credential_metadata(&account.id)?;
        let effective_service = account.service_type.as_str();
        if effective_service != "local"
            && !metadata.as_ref().is_some_and(|m| {
                m.state == "active"
                    && (m.has_refresh_token || m.expires_at > chrono::Utc::now().timestamp_millis())
            })
        {
            continue;
        }
        let oauth = metadata.as_ref().is_some_and(|m| {
            m.has_refresh_token || m.scope.as_deref().is_some_and(|s| !s.is_empty())
        });
        let endpoint = account
            .endpoint_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(match effective_service {
                "openai" => "https://api.openai.com/v1",
                "anthropic" => "https://api.anthropic.com/v1",
                "gemini" => "https://cloudcode-pa.googleapis.com",
                "copilot" => "https://api.githubcopilot.com",
                "antigravity" => "https://cloudcode-pa.googleapis.com",
                "deepseek" => "https://api.deepseek.com/v1",
                "local" => "http://localhost:11434",
                _ => "",
            });
        let key = custos_adapters::providers::probe::verified_catalog_key(
            &account.id,
            endpoint,
            oauth,
            metadata.as_ref().map(|m| m.created_at).unwrap_or(0),
            effective_service,
        );
        if availability.check(&key, model).is_err() {
            continue;
        }
        if repo.list_catalog_models(Some(&key))?.iter().any(|m| {
            m.id == model
                && custos_adapters::providers::probe::conversation_model_is_visible(
                    &m.id,
                    Some(&m.label),
                )
                && effort.is_none_or(|value| m.efforts.iter().any(|supported| supported == value))
        }) {
            eligible.push(account.id.clone());
        }
    }
    if stored && eligible.is_empty() {
        return Err(DomainError::Validation(
            "No active account advertises this model. Refresh models or reconnect the provider."
                .into(),
        ));
    }
    let selected = repo.select_account(service, &eligible)?;
    tracing::debug!(service, account_id = ?selected, "Reserved provider account");
    Ok(selected)
}

pub struct DaemonCredentialResolver {
    pub availability: custos_adapters::providers::catalog::ModelAvailability,
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
        if !custos_adapters::providers::probe::conversation_model_is_visible(
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
                    custos_adapters::providers::oauth_pkce::oauth_settings(&tok.service_type)?;
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
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

/// Recompute identity from current provider/credential metadata, never from model family.
pub(crate) fn provider_catalog_scope(
    repo: &custos_persistence::ProviderRepository,
    account: &str,
) -> Result<String, DomainError> {
    let record = repo.get_provider(account)?.ok_or_else(|| {
        DomainError::Validation("Selected provider account no longer exists".into())
    })?;
    let metadata = repo.credential_metadata(account)?;
    let oauth = metadata
        .as_ref()
        .is_some_and(|m| m.has_refresh_token || m.scope.as_deref().is_some_and(|s| !s.is_empty()));
    let endpoint = record
        .endpoint_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(match record.service_type.as_str() {
            "openai" => "https://api.openai.com/v1",
            "anthropic" => "https://api.anthropic.com/v1",
            "gemini" => "https://generativelanguage.googleapis.com",
            "copilot" => "https://api.githubcopilot.com",
            "antigravity" => "https://cloudcode-pa.googleapis.com",
            "deepseek" => "https://api.deepseek.com/v1",
            "local" => "http://localhost:11434",
            _ => "",
        });
    Ok(custos_adapters::providers::probe::verified_catalog_key(
        account,
        endpoint,
        oauth,
        metadata.as_ref().map(|m| m.created_at).unwrap_or(0),
        &record.service_type,
    ))
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    fn token(oauth: bool) -> custos_domain::OAuthTokenRecord {
        custos_domain::OAuthTokenRecord {
            provider_id: "openai".into(),
            service_type: "openai".into(),
            access_token: if oauth { "opaque-oauth" } else { "eyJ-api-key" }.into(),
            refresh_token: if oauth { Some("refresh".into()) } else { None },
            expires_at: i64::MAX / 2,
            token_type: "bearer".into(),
            scope: None,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[tokio::test]
    async fn credential_metadata_controls_authentication_not_token_prefix() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let resolver = DaemonCredentialResolver {
            availability: Default::default(),
            store: store.clone(),
            credentials: crate::runtime::CustosRuntime::credential_service(&store),
        };
        resolver
            .credentials
            .save_oauth_token(&token(true))
            .await
            .unwrap();
        assert!(resolver.uses_oauth("openai").await.unwrap());
        assert_eq!(
            resolver.get_token("openai").await.unwrap().unwrap(),
            "opaque-oauth"
        );
        resolver
            .credentials
            .save_oauth_token(&token(false))
            .await
            .unwrap();
        assert!(!resolver.uses_oauth("openai").await.unwrap());
        assert_eq!(
            resolver.get_token("openai").await.unwrap().unwrap(),
            "eyJ-api-key"
        );
    }

    #[tokio::test]
    async fn expired_oauth_without_refresh_fails_closed() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let resolver = DaemonCredentialResolver {
            availability: Default::default(),
            store: store.clone(),
            credentials: crate::runtime::CustosRuntime::credential_service(&store),
        };
        let mut expired = token(false);
        expired.scope = Some("openid".into());
        expired.expires_at = 0;
        resolver
            .credentials
            .save_oauth_token(&expired)
            .await
            .unwrap();
        assert!(resolver.uses_oauth("openai").await.unwrap());
        assert!(resolver
            .get_token("openai")
            .await
            .unwrap_err()
            .to_string()
            .contains("expired"));
    }

    #[tokio::test]
    async fn account_pool_priority_rotation_catalog_binding_and_delete() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let service = Arc::new(crate::credentials::ProviderCredentials::new(
            Arc::new(store.providers().clone()),
            Arc::new(crate::credential_tests::FakeVault::default()),
        ));
        let repo = store.providers();
        for id in ["A", "B", "C"] {
            let config = custos_domain::ProviderConfig {
                id: id.into(),
                name: id.into(),
                service_type: "openai".into(),
                api_key_masked: "none".into(),
                status: "active".into(),
                endpoint_url: Some("https://api.openai.com/v1".into()),
                default_model: None,
                context_window: None,
                fast_mode: None,
                created_at: 0,
                updated_at: 0,
            };
            service
                .save_provider(&config, Some(&format!("fixture-{id}")))
                .await
                .unwrap();
            let meta = repo.credential_metadata(id).unwrap().unwrap();
            let key = format!(
                "verified-probe:{id}:https://api.openai.com/v1:api_key:{}",
                meta.created_at
            );
            let model = custos_domain::ModelCatalogOption {
                id: if id == "C" { "gpt-other" } else { "gpt-test" }.into(),
                label: id.into(),
                provider_type: "openai".into(),
                description: None,
                is_default: false,
                default_effort: None,
                efforts: vec![],
                supports_fast_mode: false,
                context_window: None,
                pricing: None,
            };
            repo.save_catalog_models(&key, &[model]).unwrap();
        }
        assert_eq!(
            reserve_provider_account(repo, "openai", "gpt-test")
                .unwrap()
                .as_deref(),
            Some("A")
        );
        repo.save_routing_policy("openai", false, &["B".into(), "A".into(), "C".into()])
            .unwrap();
        for _ in 0..3 {
            assert_eq!(
                reserve_provider_account(repo, "openai", "gpt-test")
                    .unwrap()
                    .as_deref(),
                Some("B")
            );
        }
        repo.save_routing_policy("openai", true, &["B".into(), "A".into(), "C".into()])
            .unwrap();
        for expected in ["B", "A", "B", "A"] {
            assert_eq!(
                reserve_provider_account(repo, "openai", "gpt-test")
                    .unwrap()
                    .as_deref(),
                Some(expected)
            );
        }
        assert!(repo
            .save_routing_policy("openai", true, &["A".into(), "A".into()])
            .is_err());
        assert!(repo
            .save_routing_policy("copilot", true, &["A".into()])
            .is_err());
        assert!(reserve_provider_account(repo, "openai", "missing").is_err());
        service.delete("B", true).await.unwrap();
        assert_eq!(
            reserve_provider_account(repo, "openai", "gpt-test")
                .unwrap()
                .as_deref(),
            Some("A")
        );
        assert_eq!(
            service
                .get_oauth_token("A")
                .await
                .unwrap()
                .unwrap()
                .access_token,
            "fixture-A"
        );
        let mut config = repo.get_provider("A").unwrap().unwrap();
        config.endpoint_url = Some("https://different.invalid/v1".into());
        repo.save_provider(&config).unwrap();
        assert!(reserve_provider_account(repo, "openai", "gpt-test").is_err());
    }

    #[tokio::test]
    async fn account_pool_concurrent_reservations_are_balanced_and_locked_selection_fails_closed() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let vault = Arc::new(crate::credential_tests::FakeVault::default());
        let service = Arc::new(crate::credentials::ProviderCredentials::new(
            Arc::new(store.providers().clone()),
            vault.clone(),
        ));
        let repo = store.providers();
        for id in ["A", "B"] {
            let config = custos_domain::ProviderConfig {
                id: id.into(),
                name: id.into(),
                service_type: "copilot".into(),
                api_key_masked: "none".into(),
                status: "active".into(),
                endpoint_url: Some("https://api.githubcopilot.com".into()),
                default_model: None,
                context_window: None,
                fast_mode: None,
                created_at: 0,
                updated_at: 0,
            };
            let mut tok = token(true);
            tok.provider_id = id.into();
            tok.service_type = "copilot".into();
            service
                .save_oauth_connection(&tok, Some(&config))
                .await
                .unwrap();
            let key = format!(
                "verified-probe:{id}:https://api.githubcopilot.com:oauth:{}",
                tok.created_at
            );
            let model = custos_domain::ModelCatalogOption {
                id: "copilot/claude-test".into(),
                label: "Claude".into(),
                provider_type: "copilot".into(),
                description: None,
                is_default: false,
                default_effort: None,
                efforts: vec![],
                supports_fast_mode: false,
                context_window: None,
                pricing: None,
            };
            repo.save_catalog_models(&key, &[model]).unwrap();
        }
        repo.save_routing_policy("copilot", true, &["A".into(), "B".into()])
            .unwrap();
        let mut handles = vec![];
        for _ in 0..20 {
            let repo = (*repo).clone();
            handles.push(tokio::task::spawn_blocking(move || {
                reserve_provider_account(&repo, "copilot", "copilot/claude-test")
                    .unwrap()
                    .unwrap()
            }));
        }
        let mut a = 0;
        for handle in handles {
            if handle.await.unwrap() == "A" {
                a += 1;
            }
        }
        assert_eq!(a, 10);
        vault
            .fail_read
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let resolver = DaemonCredentialResolver {
            store,
            credentials: service,
            availability: Default::default(),
        };
        let selected = resolver
            .select_account("copilot", "copilot/claude-test")
            .await
            .unwrap()
            .unwrap();
        assert!(resolver.get_token(&selected).await.is_err());
        assert!(resolver
            .validate_account(&selected, "anthropic")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn reasoning_admission_uses_the_selected_account_catalog_and_scope() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let credentials = Arc::new(crate::credentials::ProviderCredentials::new(
            Arc::new(store.providers().clone()),
            Arc::new(crate::credential_tests::FakeVault::default()),
        ));
        let availability = custos_adapters::providers::catalog::ModelAvailability::default();
        let resolver = DaemonCredentialResolver {
            store: store.clone(),
            credentials: credentials.clone(),
            availability: availability.clone(),
        };
        for account in ["A", "B"] {
            let config = custos_domain::ProviderConfig {
                id: account.into(),
                name: account.into(),
                service_type: "openai".into(),
                api_key_masked: "none".into(),
                status: "active".into(),
                endpoint_url: Some("https://api.openai.com/v1".into()),
                default_model: None,
                context_window: None,
                fast_mode: None,
                created_at: 0,
                updated_at: 0,
            };
            credentials
                .save_provider(&config, Some("fixture-key"))
                .await
                .unwrap();
            let scope = provider_catalog_scope(store.providers(), account).unwrap();
            let model = custos_domain::ModelCatalogOption {
                id: "gpt-6.1-sol".into(),
                label: "GPT-6.1 Sol".into(),
                provider_type: "openai".into(),
                description: None,
                is_default: true,
                default_effort: Some("low".into()),
                efforts: if account == "A" {
                    vec!["low".into(), "xhigh".into(), "max".into(), "ultra".into()]
                } else {
                    vec!["low".into()]
                },
                supports_fast_mode: false,
                context_window: None,
                pricing: None,
            };
            store
                .providers()
                .save_catalog_models(&scope, &[model])
                .unwrap();
        }
        store
            .providers()
            .save_routing_policy("openai", false, &["B".into(), "A".into()])
            .unwrap();
        assert_eq!(
            reserve_available_provider_account(
                store.providers(),
                "openai",
                "gpt-6.1-sol",
                Some("low"),
                &availability
            )
            .unwrap()
            .as_deref(),
            Some("B")
        );
        assert_eq!(
            reserve_available_provider_account(
                store.providers(),
                "openai",
                "gpt-6.1-sol",
                Some("ultra"),
                &availability
            )
            .unwrap()
            .as_deref(),
            Some("A")
        );
        let b_scope = provider_catalog_scope(store.providers(), "B").unwrap();
        availability
            .observe(
                &b_scope,
                "gpt-6.1-sol",
                403,
                r#"{"error":{"code":"model_not_found"}}"#,
                None,
            )
            .unwrap();
        assert_eq!(
            reserve_available_provider_account(
                store.providers(),
                "openai",
                "gpt-6.1-sol",
                Some("low"),
                &availability
            )
            .unwrap()
            .as_deref(),
            Some("A")
        );
        availability.refresh(&b_scope).unwrap();
        assert!(resolver
            .validate_model("A", "gpt-6.1-sol", Some("ultra"))
            .await
            .is_ok());
        assert!(resolver
            .validate_model("B", "gpt-6.1-sol", Some("ultra"))
            .await
            .is_err());
        assert!(resolver
            .validate_model("B", "gpt-6.1-sol", None)
            .await
            .is_ok());
        assert!(resolver.validate_model("A", "missing", None).await.is_err());
        resolver
            .observe_response(
                "A",
                "gpt-6.1-sol",
                429,
                r#"{"error":{"code":"insufficient_quota"}}"#,
                None,
            )
            .await
            .unwrap();
        assert!(resolver
            .validate_model("A", "gpt-6.1-sol", Some("low"))
            .await
            .is_err());
        assert!(resolver
            .validate_model("B", "gpt-6.1-sol", Some("low"))
            .await
            .is_ok());
        assert!(reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("ultra"),
            &availability
        )
        .is_err());
        assert_eq!(
            reserve_available_provider_account(
                store.providers(),
                "openai",
                "gpt-6.1-sol",
                Some("low"),
                &availability
            )
            .unwrap()
            .as_deref(),
            Some("B")
        );
        availability
            .observe(
                &b_scope,
                "gpt-6.1-sol",
                429,
                r#"{"error":{"code":"insufficient_quota"}}"#,
                None,
            )
            .unwrap();
        assert!(reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("low"),
            &availability
        )
        .is_err());
        availability.quota_recovered(&b_scope).unwrap();
        let scope = provider_catalog_scope(store.providers(), "A").unwrap();
        availability.refresh(&scope).unwrap();
        assert!(resolver
            .validate_model("A", "gpt-6.1-sol", None)
            .await
            .is_err());
        availability.quota_recovered(&scope).unwrap();
        assert!(resolver
            .validate_model("A", "gpt-6.1-sol", None)
            .await
            .is_ok());
        let mut record = store.providers().get_provider("A").unwrap().unwrap();
        record.endpoint_url = Some("https://different.invalid/v1".into());
        store.providers().save_provider(&record).unwrap();
        assert!(resolver
            .validate_model("A", "gpt-6.1-sol", None)
            .await
            .is_err());
    }
}
