use std::sync::Arc;
use custos_domain::DomainError;
use crate::api::LocalApiDispatcher;

impl LocalApiDispatcher {
    pub(crate) fn credential_service(
        &self,
    ) -> Result<&Arc<crate::credentials::ProviderCredentials>, DomainError> {
        self.providers.credentials.as_ref().ok_or_else(|| {
            DomainError::Validation(
                "secure_storage_unavailable: Credential service is not configured".into(),
            )
        })
    }

    pub(crate) async fn read_provider_token(
        &self,
        account: &str,
    ) -> Result<Option<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.get_oauth_token(account).await
    }

    pub(crate) async fn refresh_provider_token(
        &self,
        previous: &custos_domain::OAuthTokenRecord,
        refreshed: &custos_domain::OAuthTokenRecord,
    ) -> Result<(), DomainError> {
        self.credential_service()?
            .save_refreshed(previous, refreshed)
            .await
    }

    pub(crate) async fn write_provider_connection(
        &self,
        token: &custos_domain::OAuthTokenRecord,
        config: &custos_domain::ProviderConfig,
    ) -> Result<(), DomainError> {
        self.credential_service()?
            .save_oauth_connection(token, Some(config))
            .await
    }

    pub(crate) async fn remove_provider_credential(
        &self,
        account: &str,
        remove_provider: bool,
    ) -> Result<(), DomainError> {
        self.credential_service()?
            .delete(account, remove_provider)
            .await
    }

    pub(crate) async fn provider_tokens(
        &self,
    ) -> Result<Vec<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.list_oauth_tokens().await
    }
}
