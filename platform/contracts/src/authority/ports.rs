//! Authority, Policy, Outbox, and Credentials SPI Trait Ports

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::identity::error::DomainError;
use crate::authority::action::{EffectAttempt, EffectStatus};
use crate::authority::permit::ExecutionReceipt;
use crate::inference::provider::config::{OAuthTokenRecord, ProviderConfig};

// ---------------------------------------------------------------------------
// 1. Transactional Outbox Port (RFC 001)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    Pending,
    Approved,
    Dispatching,
    Receipted,
    Uncertain,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub permit_id: String,
    pub argument_digest: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub status: OutboxStatus,
    pub created_at: DateTime<Utc>,
    pub receipt: Option<ExecutionReceipt>,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(default)]
    pub payload_json: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

impl OutboxEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        task_id: impl Into<String>,
        action_id: impl Into<String>,
        permit_id: impl Into<String>,
        argument_digest: impl Into<String>,
        idempotency_key: Option<String>,
        status: OutboxStatus,
        created_at: DateTime<Utc>,
        receipt: Option<ExecutionReceipt>,
        recipient: Option<String>,
        payload_json: Option<String>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            task_id: task_id.into(),
            action_id: action_id.into(),
            permit_id: permit_id.into(),
            argument_digest: argument_digest.into(),
            idempotency_key,
            status,
            created_at,
            receipt,
            recipient,
            payload_json,
            error_message,
        }
    }
}

#[async_trait]
pub trait OutboxPort: Send + Sync {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError>;
    async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError>;
    async fn mark_approved(&self, id: &str) -> Result<(), DomainError>;
    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError>;
    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError>;
    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError>;
    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), DomainError>;
    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError>;
    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError>;
}

// ---------------------------------------------------------------------------
// 2. Effect Ledger Port (Gate 3)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EffectLedgerPort: Send + Sync {
    async fn record_effect(&self, effect: &EffectAttempt) -> Result<(), DomainError>;
    async fn update_effect_status(
        &self,
        id: &str,
        status: EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError>;
    async fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<EffectAttempt>, DomainError>;
    async fn reconcile_on_startup(&self) -> Result<usize, DomainError>;
}

// ---------------------------------------------------------------------------
// 3. Command Ledger & Idempotency Port (R1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandLedgerEntry {
    pub command_id: String,
    pub method: String,
    pub payload_digest: String,
    pub scope: Option<String>,
    pub status: String,
    pub result_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAdmission {
    Admitted,
    Completed(String),
    InFlight,
    Conflict(String),
}

#[async_trait]
pub trait CommandLedgerPort: Send + Sync {
    async fn get_command(
        &self,
        command_id: &str,
    ) -> Result<Option<CommandLedgerEntry>, DomainError>;
    async fn record_command(&self, entry: &CommandLedgerEntry) -> Result<(), DomainError>;
    async fn admit(
        &self,
        command_id: &str,
        method: &str,
        payload_digest: &str,
        scope: Option<&str>,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Result<CommandAdmission, DomainError>;
    async fn complete(&self, command_id: &str, result_json: &str) -> Result<(), DomainError>;
    async fn abandon(&self, command_id: &str) -> Result<(), DomainError>;
}

// ---------------------------------------------------------------------------
// 4. Secure Credential Vault Port
// ---------------------------------------------------------------------------

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

pub use CredentialVault as CredentialsVaultPort;
pub use CredentialVault as CredentialsPort;

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

