use crate::credentials::ProviderCredentials;
use custos_core::contracts::credentials::{
    CredentialLocation, CredentialVault, CredentialVaultError as Error,
};
use custos_domain::OAuthTokenRecord;
use custos_persistence::SqliteTaskStore;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

#[derive(Default)]
pub(crate) struct FakeVault {
    entries: Mutex<HashMap<String, Vec<u8>>>,
    pub fail_write: AtomicBool,
    pub fail_read: AtomicBool,
    pub fail_delete: AtomicBool,
    pub write_delay: AtomicU64,
    pub entered_write: AtomicBool,
}
fn key(location: &CredentialLocation) -> String {
    format!(
        "{}:{}:{}",
        location.vault_id, location.account_id, location.generation
    )
}
impl CredentialVault for FakeVault {
    fn write(&self, location: &CredentialLocation, secret: &[u8]) -> Result<(), Error> {
        self.entered_write.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(
            self.write_delay.load(Ordering::SeqCst),
        ));
        if self.fail_write.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        self.entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .insert(key(location), secret.to_vec());
        Ok(())
    }
    fn read(&self, location: &CredentialLocation) -> Result<Option<Vec<u8>>, Error> {
        if self.fail_read.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        Ok(self
            .entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .get(&key(location))
            .cloned())
    }
    fn delete(&self, location: &CredentialLocation, _: usize) -> Result<(), Error> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(Error::CleanupPending);
        }
        self.entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&key(location));
        Ok(())
    }
}
pub(crate) fn fake_credentials(
    repo: Arc<custos_persistence::ProviderRepository>,
) -> Arc<ProviderCredentials> {
    Arc::new(ProviderCredentials::new(
        repo,
        Arc::new(FakeVault::default()),
    ))
}
fn token(account: &str, value: &str) -> OAuthTokenRecord {
    OAuthTokenRecord {
        provider_id: account.into(),
        service_type: "openai".into(),
        access_token: value.into(),
        refresh_token: Some("fixture-refresh".into()),
        expires_at: i64::MAX / 2,
        token_type: "bearer".into(),
        scope: Some("openid".into()),
        created_at: 1,
        updated_at: 2,
    }
}
#[tokio::test]
async fn failed_save_preserves_previous_generation_and_deletion_fails_closed() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    service
        .save_oauth_token(&token("A", "old-fixture-secret"))
        .await
        .unwrap();
    vault.fail_write.store(true, Ordering::SeqCst);
    assert!(service
        .save_oauth_token(&token("A", "new-fixture-secret"))
        .await
        .is_err());
    vault.fail_write.store(false, Ordering::SeqCst);
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "old-fixture-secret"
    );
    vault.fail_delete.store(true, Ordering::SeqCst);
    assert!(service.delete("A", false).await.is_err());
    assert_eq!(
        repo.credential_metadata("A").unwrap().unwrap().state,
        "deleting"
    );
    assert!(service.get_oauth_token("A").await.is_err());
    vault.fail_delete.store(false, Ordering::SeqCst);
    service.initialize().await.unwrap();
    assert!(service.get_oauth_token("A").await.unwrap().is_none());
}

#[tokio::test]
async fn failed_verification_never_replaces_the_active_credential() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    service
        .save_oauth_token(&token("A", "old-value"))
        .await
        .unwrap();
    let generation = repo.credential_metadata("A").unwrap().unwrap().generation;
    vault.fail_read.store(true, Ordering::SeqCst);
    assert!(service
        .save_oauth_token(&token("A", "new-value"))
        .await
        .is_err());
    assert_eq!(
        repo.credential_metadata("A").unwrap().unwrap().generation,
        generation
    );
    vault.fail_read.store(false, Ordering::SeqCst);
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "old-value"
    );
}

#[tokio::test]
async fn caller_cancellation_cannot_race_delete_with_a_late_keyring_write() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    vault.write_delay.store(200, Ordering::SeqCst);
    let worker = service.clone();
    let request = tokio::spawn(async move {
        worker
            .save_oauth_token(&token("A", "cancelled-request"))
            .await
    });
    while !vault.entered_write.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    request.abort();
    service.delete("A", true).await.unwrap();
    assert!(service.get_oauth_token("A").await.unwrap().is_none());
    assert!(vault.entries.lock().unwrap().is_empty());
}

#[tokio::test]
async fn recovery_commits_complete_bundles_and_discards_partial_staging() {
    use custos_core::contracts::credentials::credential_part_count;
    use custos_persistence::{CredentialMetadata, CredentialOperation};
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    let value = token("A", "recovered-value");
    let bytes = serde_json::to_vec(&value).unwrap();
    let metadata = CredentialMetadata::from_token(
        &value,
        uuid::Uuid::new_v4().to_string(),
        credential_part_count(bytes.len()),
    );
    let op = CredentialOperation {
        id: "recovery".into(),
        metadata: metadata.clone(),
        previous: None,
        config: None,
        phase: "write".into(),
        remove_provider: false,
    };
    repo.prepare_credential_operation(&op).unwrap();
    let location = CredentialLocation {
        vault_id: repo.credential_vault_id().unwrap(),
        account_id: "A".into(),
        generation: metadata.generation.clone(),
    };
    vault.write(&location, &bytes).unwrap();
    service.initialize().await.unwrap();
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "recovered-value"
    );
    let mut partial = op.clone();
    partial.id = "partial".into();
    partial.metadata.provider_id = "B".into();
    partial.metadata.generation = uuid::Uuid::new_v4().to_string();
    repo.prepare_credential_operation(&partial).unwrap();
    service.initialize().await.unwrap();
    assert!(service.get_oauth_token("B").await.unwrap().is_none());
    assert!(repo.credential_operations("B").unwrap().is_empty());
}

#[tokio::test]
async fn profile_isolation_and_metadata_updates_do_not_cross_accounts() {
    let a = SqliteTaskStore::new_in_memory().unwrap();
    let b = SqliteTaskStore::new_in_memory().unwrap();
    let vault = Arc::new(FakeVault::default());
    let a = Arc::new(ProviderCredentials::new(
        Arc::new(a.providers().clone()),
        vault.clone(),
    ));
    let b = Arc::new(ProviderCredentials::new(
        Arc::new(b.providers().clone()),
        vault,
    ));
    a.save_oauth_token(&token("same-account", "profile-a"))
        .await
        .unwrap();
    b.save_oauth_token(&token("same-account", "profile-b"))
        .await
        .unwrap();
    assert_eq!(
        a.get_oauth_token("same-account")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "profile-a"
    );
    assert_eq!(
        b.get_oauth_token("same-account")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "profile-b"
    );
}

#[tokio::test]
async fn database_commit_failure_keeps_old_secret_and_provider_config() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    service
        .save_oauth_token(&token("A", "old-secret"))
        .await
        .unwrap();
    store.db().lock().unwrap().execute_batch("CREATE TRIGGER reject_credential_commit BEFORE INSERT ON provider_credentials BEGIN SELECT RAISE(ABORT,'fixture commit failure'); END;").unwrap();
    assert!(service
        .save_oauth_token(&token("A", "new-secret"))
        .await
        .is_err());
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "old-secret"
    );
    assert_eq!(vault.entries.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn legacy_migration_preserves_data_when_locked_then_scrubs_database_and_wal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.db");
    let store = SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
    store.db().lock().unwrap().execute_batch("INSERT INTO provider_oauth_tokens(provider_id,service_type,access_token,refresh_token,expires_at,created_at,updated_at) VALUES('legacy','openai','legacy-fixture-secret-unique','legacy-refresh-unique',99999999999999,1,2);").unwrap();
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault.clone()));
    vault.fail_write.store(true, Ordering::SeqCst);
    assert!(service.initialize().await.is_err());
    assert!(service.get_oauth_token("legacy").await.is_err());
    assert!(repo.get_oauth_token("legacy").unwrap().is_some());
    vault.fail_write.store(false, Ordering::SeqCst);
    service.initialize().await.unwrap();
    service.initialize().await.unwrap();
    assert!(repo.get_oauth_token("legacy").unwrap().is_none());
    assert_eq!(
        service
            .get_oauth_token("legacy")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "legacy-fixture-secret-unique"
    );
    for file in [path.clone(), path.with_extension("db-wal")] {
        if file.exists() {
            let bytes = std::fs::read(file).unwrap();
            assert!(!bytes
                .windows(b"legacy-fixture-secret-unique".len())
                .any(|window| window == b"legacy-fixture-secret-unique"));
        }
    }
    let payloads: String = store
        .db()
        .lock()
        .unwrap()
        .query_row(
            "SELECT group_concat(payload) FROM outbox_messages",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!payloads.contains("legacy-fixture-secret-unique"));
    assert!(!payloads.contains("legacy-refresh-unique"));
}

#[tokio::test]
async fn metadata_update_keeps_secret_and_service_change_requires_reauthentication() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let service = fake_credentials(repo.clone());
    let mut config = custos_domain::ProviderConfig {
        id: "account".into(),
        name: "Account".into(),
        service_type: "openai".into(),
        api_key_masked: "untrusted-display".into(),
        status: "active".into(),
        endpoint_url: None,
        default_model: None,
        context_window: None,
        fast_mode: None,
        created_at: 1,
        updated_at: 2,
    };
    service
        .save_provider(&config, Some("fixture-key"))
        .await
        .unwrap();
    service.save_provider(&config, Some("")).await.unwrap();
    assert_eq!(
        service
            .get_oauth_token("account")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "fixture-key"
    );
    assert!(
        service.get_oauth_token("openai").await.unwrap().is_none(),
        "Saving must not create a service alias"
    );
    config.service_type = "anthropic".into();
    assert!(service.save_provider(&config, None).await.is_err());
    assert_eq!(
        repo.get_provider("account").unwrap().unwrap().service_type,
        "openai"
    );
}

#[tokio::test]
async fn api_save_failure_returns_error_and_never_marks_account_connected() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let repo = Arc::new(store.providers().clone());
    let vault = Arc::new(FakeVault::default());
    vault.fail_write.store(true, Ordering::SeqCst);
    let service = Arc::new(ProviderCredentials::new(repo.clone(), vault));
    let tasks = Arc::new(custos_core::TaskService::new(store.clone()));
    let sessions = Arc::new(custos_runtime::session::SessionManager::with_store(store));
    let bridge = Arc::new(custos_bridge::BridgeService::new(
        sessions.clone(),
        tasks.clone(),
    ));
    let dispatcher = crate::api::LocalApiDispatcher::new(tasks, sessions, bridge)
        .with_providers(repo.clone())
        .with_credentials(service);
    let response=dispatcher.handle_request(crate::api::ApiRequest{id:"save".into(),method:"v1.providers.save".into(),params:serde_json::json!({"id":"A","service_type":"openai","api_key":"never-expose-fixture-key","status":"active"})}).await;
    assert!(response
        .error
        .as_ref()
        .unwrap()
        .contains("secure_storage_unavailable"));
    assert!(!serde_json::to_string(&response)
        .unwrap()
        .contains("never-expose-fixture-key"));
    assert!(repo.get_provider("A").unwrap().is_none());
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "Migrates synthetic tokens into an isolated Windows credential namespace"]
async fn native_keyring_service_restart_and_legacy_migration() {
    use custos_adapters::providers::credential_vault::KeyringCredentialVault;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native-credentials.db");
    let store = SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
    let access = "synthetic-native-token-".repeat(350);
    store.db().lock().unwrap().execute("INSERT INTO provider_oauth_tokens(provider_id,service_type,access_token,refresh_token,expires_at,created_at,updated_at) VALUES('native-test','openai',?1,'synthetic-native-refresh',99999999999999,1,2)",[&access]).unwrap();
    let repo = Arc::new(store.providers().clone());
    struct Cleanup(Arc<custos_persistence::ProviderRepository>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if let Ok(vault_id) = self.0.credential_vault_id() {
                if let Ok(accounts) = self.0.credential_accounts() {
                    for account in accounts {
                        let mut entries = Vec::new();
                        if let Ok(Some(meta)) = self.0.credential_metadata(&account) {
                            entries.push(meta);
                        }
                        if let Ok(ops) = self.0.credential_operations(&account) {
                            for op in ops {
                                entries.push(op.metadata);
                                if let Some(old) = op.previous {
                                    entries.push(old);
                                }
                            }
                        }
                        for meta in entries {
                            let location = CredentialLocation {
                                vault_id: vault_id.clone(),
                                account_id: account.clone(),
                                generation: meta.generation,
                            };
                            let _ = KeyringCredentialVault.delete(&location, meta.parts);
                        }
                    }
                }
            }
        }
    }
    let _cleanup = Cleanup(repo.clone());
    let service = Arc::new(ProviderCredentials::new(
        repo.clone(),
        Arc::new(KeyringCredentialVault),
    ));
    service.initialize().await.unwrap();
    assert!(repo.get_oauth_token("native-test").unwrap().is_none());
    assert_eq!(
        service
            .get_oauth_token("native-test")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        access
    );
    drop(service);
    drop(store);
    let reopened = SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
    let service = Arc::new(ProviderCredentials::new(
        Arc::new(reopened.providers().clone()),
        Arc::new(KeyringCredentialVault),
    ));
    service.initialize().await.unwrap();
    assert_eq!(
        service
            .get_oauth_token("native-test")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        access
    );
    service.delete("native-test", false).await.unwrap();
    assert!(service
        .get_oauth_token("native-test")
        .await
        .unwrap()
        .is_none());
    for file in [path.clone(), path.with_extension("db-wal")] {
        if file.exists() {
            let bytes = std::fs::read(file).unwrap();
            assert!(!bytes
                .windows(access.len())
                .any(|window| window == access.as_bytes()));
        }
    }
}
#[tokio::test]
async fn refresh_conflicts_cannot_overwrite_a_replaced_credential() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = Arc::new(store.providers().clone());
    let service = fake_credentials(repo);
    let old = token("A", "old-fixture-secret");
    service.save_oauth_token(&old).await.unwrap();
    service
        .save_oauth_token(&token("A", "replacement"))
        .await
        .unwrap();
    assert!(service
        .save_refreshed(&old, &token("A", "stale-refresh"))
        .await
        .is_err());
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "replacement"
    );
}
