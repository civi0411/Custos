//! Internal secret storage contract. No platform I/O belongs in this module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialLocation {
    pub vault_id: String,
    pub account_id: String,
    pub generation: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialVaultError {
    #[error("secure_storage_unavailable: Unlock the operating system credential store and retry")]
    Unavailable,
    #[error("secure_storage_missing: Reconnect this provider account")]
    Missing,
    #[error("secure_storage_corrupt: Reconnect this provider account")]
    Corrupt,
    #[error("secure_storage_save_failed: Credential could not be saved")]
    SaveFailed,
    #[error("secure_storage_cleanup_pending: Unlock the credential store and retry deletion")]
    CleanupPending,
    #[error("secure_storage_invalid: Invalid credential bundle")]
    Invalid,
}

pub const MAX_CREDENTIAL_BYTES: usize = 256 * 1024;
pub const CREDENTIAL_SEGMENT_SIZE: usize = 1024;
pub fn credential_part_count(bytes: usize) -> usize {
    bytes
        .div_ceil(3)
        .saturating_mul(4)
        .div_ceil(CREDENTIAL_SEGMENT_SIZE)
}

pub trait CredentialVault: Send + Sync {
    fn write(
        &self,
        location: &CredentialLocation,
        secret: &[u8],
    ) -> Result<(), CredentialVaultError>;
    fn read(&self, location: &CredentialLocation) -> Result<Option<Vec<u8>>, CredentialVaultError>;
    fn delete(
        &self,
        location: &CredentialLocation,
        parts: usize,
    ) -> Result<(), CredentialVaultError>;
}

use custos_domain::{OAuthTokenRecord, ProviderConfig};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialMetadata {
    pub provider_id: String,
    pub service_type: String,
    pub expires_at: i64,
    pub token_type: String,
    pub scope: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub has_refresh_token: bool,
    pub generation: String,
    pub parts: usize,
    pub state: String,
}
impl CredentialMetadata {
    pub fn from_token(token: &OAuthTokenRecord, generation: String, parts: usize) -> Self {
        Self {
            provider_id: token.provider_id.clone(),
            service_type: token.service_type.clone(),
            expires_at: token.expires_at,
            token_type: token.token_type.clone(),
            scope: token.scope.clone(),
            created_at: token.created_at,
            updated_at: token.updated_at,
            has_refresh_token: token.refresh_token.is_some(),
            generation,
            parts,
            state: "active".into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CredentialOperation {
    pub id: String,
    pub metadata: CredentialMetadata,
    pub previous: Option<CredentialMetadata>,
    pub config: Option<ProviderConfig>,
    pub phase: String,
    pub remove_provider: bool,
}
