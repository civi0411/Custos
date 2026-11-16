use async_trait::async_trait;
use custos_domain::DomainError;

use super::super::credentials::CredentialResolver;

pub(crate) struct TestCredentialResolver {
    pub(crate) oauth: bool,
    pub(crate) endpoint: String,
    pub(crate) fail: bool,
}

#[async_trait]
impl CredentialResolver for TestCredentialResolver {
    async fn uses_oauth(&self, _: &str) -> Result<bool, DomainError> {
        Ok(self.oauth)
    }

    async fn get_token(&self, _: &str) -> Result<Option<String>, DomainError> {
        if self.fail {
            return Err(DomainError::Validation("OAuth refresh failed".into()));
        }
        Ok(Some(
            if self.oauth {
                "opaque-credential"
            } else {
                "eyJ-api-key"
            }
            .into(),
        ))
    }

    async fn get_endpoint_url(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(self.endpoint.clone()))
    }
}
