//! Composition-root service coordinating OS secrets and durable metadata.
use custos_core::contracts::credentials::{
    credential_part_count, CredentialLocation, CredentialVault, CredentialVaultError,
    MAX_CREDENTIAL_BYTES,
};
use custos_domain::{DomainError, OAuthTokenRecord, ProviderConfig};
use custos_persistence::{CredentialMetadata, CredentialOperation, ProviderRepository};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
enum CredentialError {
    #[error(transparent)]
    Vault(#[from] CredentialVaultError),
    #[error("secure_storage_save_failed: Background credential operation failed")]
    Worker,
    #[error("secure_storage_conflict: Account changed; retry")]
    Conflict,
}
impl From<CredentialError> for DomainError {
    fn from(value: CredentialError) -> Self {
        DomainError::Validation(value.to_string())
    }
}

pub struct ProviderCredentials {
    repo: Arc<ProviderRepository>,
    vault: Arc<dyn CredentialVault>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}
impl ProviderCredentials {
    pub fn new(repo: Arc<ProviderRepository>, vault: Arc<dyn CredentialVault>) -> Self {
        Self {
            repo,
            vault,
            locks: Mutex::new(HashMap::new()),
        }
    }
    async fn lock(&self, account: &str) -> tokio::sync::OwnedMutexGuard<()> {
        self.locks
            .lock()
            .await
            .entry(account.into())
            .or_default()
            .clone()
            .lock_owned()
            .await
    }
    fn location(&self, metadata: &CredentialMetadata) -> Result<CredentialLocation, DomainError> {
        Ok(CredentialLocation {
            vault_id: self.repo.credential_vault_id()?,
            account_id: metadata.provider_id.clone(),
            generation: metadata.generation.clone(),
        })
    }
    async fn read(
        &self,
        metadata: &CredentialMetadata,
    ) -> Result<Option<OAuthTokenRecord>, DomainError> {
        let location = self.location(metadata)?;
        let vault = self.vault.clone();
        let bytes = tokio::task::spawn_blocking(move || vault.read(&location))
            .await
            .map_err(|_| CredentialError::Worker)?
            .map_err(CredentialError::from)?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let token: OAuthTokenRecord = serde_json::from_slice(&bytes)
            .map_err(|_| CredentialError::Vault(CredentialVaultError::Corrupt))?;
        if token.provider_id != metadata.provider_id
            || token.service_type != metadata.service_type
            || token.updated_at != metadata.updated_at
            || token.expires_at != metadata.expires_at
            || token.created_at != metadata.created_at
            || token.token_type != metadata.token_type
            || token.scope != metadata.scope
            || token.refresh_token.is_some() != metadata.has_refresh_token
            || token.access_token.is_empty()
        {
            return Err(CredentialError::Vault(CredentialVaultError::Corrupt).into());
        }
        Ok(Some(token))
    }
    async fn remove(&self, metadata: &CredentialMetadata) -> Result<(), DomainError> {
        let location = self.location(metadata)?;
        let parts = metadata.parts;
        let vault = self.vault.clone();
        tokio::task::spawn_blocking(move || vault.delete(&location, parts))
            .await
            .map_err(|_| CredentialError::Worker)?
            .map_err(CredentialError::from)?;
        Ok(())
    }
    async fn cleanup(&self, op: &CredentialOperation) -> Result<(), DomainError> {
        match op.phase.as_str() {
            "cleanup-old" => {
                if let Some(old) = &op.previous {
                    self.remove(old).await?;
                }
            }
            "cleanup-new" | "delete" => self.remove(&op.metadata).await?,
            _ => return Err(CredentialError::Vault(CredentialVaultError::Corrupt).into()),
        }
        self.repo.finish_credential_operation(op)
    }
    async fn recover_locked(&self, account: &str) -> Result<(), DomainError> {
        for mut op in self.repo.credential_operations(account)? {
            if op.phase == "write" {
                let current = self.repo.credential_metadata(account)?;
                if current.as_ref().map(|m| &m.generation)
                    != op.previous.as_ref().map(|m| &m.generation)
                {
                    op.phase = "cleanup-new".into();
                } else {
                    match self.read(&op.metadata).await {
                        Ok(Some(_)) => {
                            self.repo.complete_credential_write(&op)?;
                            op.phase = "cleanup-old".into();
                        }
                        Err(DomainError::Validation(ref message))
                            if message.starts_with("secure_storage_corrupt") =>
                        {
                            op.phase = "cleanup-new".into()
                        }
                        Ok(None) => op.phase = "cleanup-new".into(),
                        Err(error) => return Err(error),
                    }
                }
                self.repo.update_credential_operation(&op)?;
            }
            if let Err(error) = self.cleanup(&op).await {
                // Old/staging cleanup must not make a committed generation unusable.
                if op.phase == "delete" {
                    return Err(error);
                }
                tracing::warn!(provider_id=%account, operation_id=%op.id, "Credential cleanup remains pending");
            }
        }
        Ok(())
    }
    async fn save_locked(
        &self,
        token: &OAuthTokenRecord,
        config: Option<&ProviderConfig>,
        expected: Option<&OAuthTokenRecord>,
    ) -> Result<(), DomainError> {
        if token.access_token.trim().is_empty() || token.provider_id.is_empty() {
            return Err(CredentialError::Vault(CredentialVaultError::Invalid).into());
        }
        if let Some(config) = config {
            if config.id != token.provider_id || config.service_type != token.service_type {
                return Err(CredentialError::Conflict.into());
            }
        }
        let previous = self.repo.credential_metadata(&token.provider_id)?;
        if let Some(expected) = expected {
            let Some(metadata) = &previous else {
                return Err(CredentialError::Conflict.into());
            };
            let stored = self
                .read(metadata)
                .await?
                .ok_or(CredentialError::Conflict)?;
            if metadata.state != "active"
                || stored.service_type != expected.service_type
                || stored.updated_at != expected.updated_at
                || stored.scope != expected.scope
                || stored.access_token != expected.access_token
                || stored.refresh_token != expected.refresh_token
            {
                return Err(CredentialError::Conflict.into());
            }
        }
        let bytes = serde_json::to_vec(token)
            .map_err(|_| CredentialError::Vault(CredentialVaultError::Invalid))?;
        if bytes.len() > MAX_CREDENTIAL_BYTES {
            return Err(CredentialError::Vault(CredentialVaultError::Invalid).into());
        }
        let op = CredentialOperation {
            id: custos_domain::new_id("credential"),
            metadata: CredentialMetadata::from_token(
                token,
                uuid::Uuid::new_v4().to_string(),
                credential_part_count(bytes.len()),
            ),
            previous,
            config: config.cloned(),
            phase: "write".into(),
            remove_provider: false,
        };
        self.repo.prepare_credential_operation(&op)?;
        tracing::info!(provider_id=%token.provider_id, operation_id=%op.id, "Saving provider credential generation");
        let location = self.location(&op.metadata)?;
        let vault = self.vault.clone();
        let result = tokio::task::spawn_blocking(move || vault.write(&location, &bytes))
            .await
            .map_err(|_| CredentialError::Worker)
            .and_then(|result| result.map_err(CredentialError::from));
        let result: Result<(), DomainError> = match result {
            Ok(()) => match self.read(&op.metadata).await {
                Ok(Some(saved))
                    if saved.access_token == token.access_token
                        && saved.refresh_token == token.refresh_token =>
                {
                    self.repo.complete_credential_write(&op)
                }
                Ok(_) => Err(CredentialError::Vault(CredentialVaultError::Corrupt).into()),
                Err(error) => Err(error),
            },
            Err(error) => Err(error.into()),
        };
        if let Err(error) = result {
            let mut abort = op.clone();
            abort.phase = "cleanup-new".into();
            self.repo.update_credential_operation(&abort)?;
            if self.cleanup(&abort).await.is_err() {
                tracing::warn!(provider_id=%token.provider_id,operation_id=%op.id,"Failed save cleanup remains pending");
            }
            return Err(error);
        }
        let mut cleanup = op;
        cleanup.phase = "cleanup-old".into();
        if self.cleanup(&cleanup).await.is_err() {
            tracing::warn!(provider_id=%token.provider_id,operation_id=%cleanup.id,"Previous credential cleanup remains pending");
        }
        if self.repo.checkpoint_credentials().is_err() {
            tracing::warn!(provider_id=%token.provider_id,"Credential WAL cleanup remains pending");
        }
        Ok(())
    }
    async fn migrate_locked(&self, account: &str) -> Result<(), DomainError> {
        if self.repo.credential_metadata(account)?.is_none() {
            if let Some(token) = self.repo.get_oauth_token(account)? {
                let config = self.repo.get_provider(account)?.map(|mut config| {
                    config.api_key_masked =
                        if token.refresh_token.is_some() || token.scope.is_some() {
                            "oauth"
                        } else {
                            "••••••••"
                        }
                        .into();
                    config
                });
                self.save_locked(&token, config.as_ref(), None).await?;
            }
        }
        Ok(())
    }
    pub async fn initialize(self: &Arc<Self>) -> Result<(), DomainError> {
        let service = self.clone();
        tokio::spawn(async move { service.initialize_owned().await })
            .await
            .map_err(|_| CredentialError::Worker)?
    }
    async fn initialize_owned(&self) -> Result<(), DomainError> {
        self.repo.credential_vault_id()?;
        let mut failure = None;
        for account in self.repo.credential_accounts()? {
            let _guard = self.lock(&account).await;
            if let Err(error) = async {
                self.recover_locked(&account).await?;
                self.migrate_locked(&account).await
            }
            .await
            {
                tracing::warn!(provider_id=%account,"Provider credential migration/recovery blocked; unlock store to retry");
                failure = Some(error);
            }
        }
        self.repo.checkpoint_credentials()?;
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(())
    }
    pub async fn get_oauth_token(
        self: &Arc<Self>,
        account: &str,
    ) -> Result<Option<OAuthTokenRecord>, DomainError> {
        let account = account.to_string();
        let service = self.clone();
        tokio::spawn(async move { service.get_oauth_token_owned(&account).await })
            .await
            .map_err(|_| CredentialError::Worker)?
    }
    async fn get_oauth_token_owned(
        &self,
        account: &str,
    ) -> Result<Option<OAuthTokenRecord>, DomainError> {
        let _guard = self.lock(account).await;
        self.recover_locked(account).await?;
        self.migrate_locked(account).await?;
        let Some(metadata) = self.repo.credential_metadata(account)? else {
            return Ok(None);
        };
        if metadata.state != "active" {
            return Err(CredentialError::Vault(CredentialVaultError::CleanupPending).into());
        }
        self.read(&metadata)
            .await?
            .map(Some)
            .ok_or_else(|| CredentialError::Vault(CredentialVaultError::Missing).into())
    }
    pub async fn save_oauth_token(
        self: &Arc<Self>,
        token: &OAuthTokenRecord,
    ) -> Result<(), DomainError> {
        self.save_oauth_connection(token, None).await
    }
    pub async fn save_provider(
        self: &Arc<Self>,
        config: &ProviderConfig,
        api_key: Option<&str>,
    ) -> Result<(), DomainError> {
        let config = config.clone();
        let api_key = api_key.map(str::to_string);
        let service = self.clone();
        tokio::spawn(async move {
            service
                .save_provider_owned(&config, api_key.as_deref())
                .await
        })
        .await
        .map_err(|_| CredentialError::Worker)?
    }
    async fn save_provider_owned(
        &self,
        config: &ProviderConfig,
        api_key: Option<&str>,
    ) -> Result<(), DomainError> {
        let _guard = self.lock(&config.id).await;
        self.recover_locked(&config.id).await?;
        let mut config = config.clone();
        if let Some(previous) = self.repo.get_provider(&config.id)? {
            config.created_at = previous.created_at;
        }
        if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) {
            let now = chrono::Utc::now().timestamp_millis();
            let token = OAuthTokenRecord {
                provider_id: config.id.clone(),
                service_type: config.service_type.clone(),
                access_token: key.trim().into(),
                refresh_token: None,
                expires_at: i64::MAX / 2,
                token_type: "bearer".into(),
                scope: None,
                created_at: now,
                updated_at: now,
            };
            config.api_key_masked = "••••••••".into();
            return self.save_locked(&token, Some(&config), None).await;
        }
        self.migrate_locked(&config.id).await?;
        if let Some(metadata) = self.repo.credential_metadata(&config.id)? {
            if metadata.service_type != config.service_type {
                return Err(CredentialError::Conflict.into());
            }
            if metadata.state != "active" {
                return Err(CredentialError::Vault(CredentialVaultError::CleanupPending).into());
            }
            let token = self
                .read(&metadata)
                .await?
                .ok_or(CredentialError::Vault(CredentialVaultError::Missing))?;
            config.api_key_masked = if token.refresh_token.is_some() || token.scope.is_some() {
                "oauth"
            } else {
                "••••••••"
            }
            .into();
        } else {
            config.api_key_masked = "none".into();
            if !matches!(config.service_type.as_str(), "local" | "ollama") {
                config.status = "unconfigured".into();
            }
        }
        self.repo.save_provider(&config)
    }
    pub async fn save_oauth_connection(
        self: &Arc<Self>,
        token: &OAuthTokenRecord,
        config: Option<&ProviderConfig>,
    ) -> Result<(), DomainError> {
        let token = token.clone();
        let config = config.cloned();
        let service = self.clone();
        tokio::spawn(async move {
            service
                .save_oauth_connection_owned(&token, config.as_ref())
                .await
        })
        .await
        .map_err(|_| CredentialError::Worker)?
    }
    async fn save_oauth_connection_owned(
        &self,
        token: &OAuthTokenRecord,
        config: Option<&ProviderConfig>,
    ) -> Result<(), DomainError> {
        let _guard = self.lock(&token.provider_id).await;
        self.recover_locked(&token.provider_id).await?;
        self.save_locked(token, config, None).await
    }
    pub async fn save_refreshed(
        self: &Arc<Self>,
        previous: &OAuthTokenRecord,
        refreshed: &OAuthTokenRecord,
    ) -> Result<(), DomainError> {
        let previous = previous.clone();
        let refreshed = refreshed.clone();
        let service = self.clone();
        tokio::spawn(async move { service.save_refreshed_owned(&previous, &refreshed).await })
            .await
            .map_err(|_| CredentialError::Worker)?
    }
    async fn save_refreshed_owned(
        &self,
        previous: &OAuthTokenRecord,
        refreshed: &OAuthTokenRecord,
    ) -> Result<(), DomainError> {
        let _guard = self.lock(&previous.provider_id).await;
        self.recover_locked(&previous.provider_id).await?;
        self.save_locked(refreshed, None, Some(previous)).await
    }
    pub async fn delete(
        self: &Arc<Self>,
        account: &str,
        remove_provider: bool,
    ) -> Result<(), DomainError> {
        let account = account.to_string();
        let service = self.clone();
        tokio::spawn(async move { service.delete_owned(&account, remove_provider).await })
            .await
            .map_err(|_| CredentialError::Worker)?
    }
    async fn delete_owned(&self, account: &str, remove_provider: bool) -> Result<(), DomainError> {
        let _guard = self.lock(account).await;
        self.recover_locked(account).await?;
        let metadata = self.repo.credential_metadata(account)?;
        if let Some(metadata) = metadata {
            let op = CredentialOperation {
                id: custos_domain::new_id("credential"),
                previous: Some(metadata.clone()),
                metadata,
                config: None,
                phase: "delete".into(),
                remove_provider,
            };
            self.repo.prepare_credential_operation(&op)?;
            self.cleanup(&op).await?;
        } else {
            // Legacy deletion also uses the journal, never requires reading secret bytes.
            if let Some(token) = self.repo.get_oauth_token(account)? {
                let metadata =
                    CredentialMetadata::from_token(&token, uuid::Uuid::new_v4().to_string(), 0);
                let op = CredentialOperation {
                    id: custos_domain::new_id("credential"),
                    metadata,
                    previous: None,
                    config: None,
                    phase: "delete".into(),
                    remove_provider,
                };
                self.repo.prepare_credential_operation(&op)?;
                self.cleanup(&op).await?;
            } else if remove_provider {
                self.repo.delete_provider(account)?;
            }
        }
        if self.repo.checkpoint_credentials().is_err() {
            tracing::warn!(provider_id=%account,"Credential WAL cleanup remains pending");
        }
        Ok(())
    }
    pub async fn list_oauth_tokens(self: &Arc<Self>) -> Result<Vec<OAuthTokenRecord>, DomainError> {
        let mut tokens = Vec::new();
        for account in self.repo.credential_accounts()? {
            if let Some(token) = self.get_oauth_token(&account).await? {
                tokens.push(token);
            }
        }
        Ok(tokens)
    }
    pub async fn list_expiring_oauth_tokens(
        self: &Arc<Self>,
        threshold: i64,
    ) -> Result<Vec<OAuthTokenRecord>, DomainError> {
        Ok(self
            .list_oauth_tokens()
            .await?
            .into_iter()
            .filter(|token| token.refresh_token.is_some() && token.expires_at <= threshold)
            .collect())
    }
}
