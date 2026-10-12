//! Recoverable metadata operations; secret bytes are never written here.
use super::providers::ProviderRepository;
pub use custos_core::contracts::credentials::{CredentialMetadata, CredentialOperation};
use custos_domain::DomainError;
use rusqlite::{params, OptionalExtension};

// Do not delete an old OS secret until the new database reference is fsynced.
struct DurableWriter<'a> {
    connection: &'a mut rusqlite::Connection,
    previous: i64,
}
impl<'a> DurableWriter<'a> {
    fn new(connection: &'a mut rusqlite::Connection) -> Result<Self, DomainError> {
        let previous = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(error)?;
        connection
            .pragma_update(None, "synchronous", 2)
            .map_err(error)?;
        Ok(Self {
            connection,
            previous,
        })
    }
}
impl std::ops::Deref for DurableWriter<'_> {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &Self::Target {
        self.connection
    }
}
impl std::ops::DerefMut for DurableWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.connection
    }
}
impl Drop for DurableWriter<'_> {
    fn drop(&mut self) {
        if self
            .connection
            .pragma_update(None, "synchronous", self.previous)
            .is_err()
        {
            eprintln!(
                "Credential storage: previous SQLite durability setting could not be restored"
            );
        }
    }
}

fn error(_: impl std::fmt::Display) -> DomainError {
    DomainError::Validation(
        "secure_storage_save_failed: Credential metadata could not be committed".into(),
    )
}
impl ProviderRepository {
    pub fn credential_vault_id(&self) -> Result<String, DomainError> {
        let mut conn = self.db.lock()?;
        let mut conn = DurableWriter::new(&mut conn)?;
        if let Some(id) = conn
            .query_row(
                "SELECT vault_id FROM provider_credential_vault WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?
        {
            return Ok(id);
        }
        let tx = conn.transaction().map_err(error)?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT OR IGNORE INTO provider_credential_vault(singleton,vault_id) VALUES(1,?1)",
            params![id],
        )
        .map_err(error)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.vault.created",
            "database",
            "metadata",
            0,
        )?;
        let id = tx
            .query_row(
                "SELECT vault_id FROM provider_credential_vault WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(id)
    }
    pub fn credential_metadata(
        &self,
        account: &str,
    ) -> Result<Option<CredentialMetadata>, DomainError> {
        let conn = self.db.lock()?;
        let json: Option<String> = conn
            .query_row(
                "SELECT metadata_json FROM provider_credentials WHERE provider_id=?1",
                params![account],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?;
        json.map(|value| serde_json::from_str(&value).map_err(error))
            .transpose()
    }
    pub fn credential_accounts(&self) -> Result<Vec<String>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn.prepare("SELECT provider_id FROM provider_credentials UNION SELECT provider_id FROM provider_oauth_tokens UNION SELECT provider_id FROM provider_credential_operations").map_err(error)?;
        let rows = stmt.query_map([], |row| row.get(0)).map_err(error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(error)
    }
    pub fn credential_operations(
        &self,
        account: &str,
    ) -> Result<Vec<CredentialOperation>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn.prepare("SELECT operation_json FROM provider_credential_operations WHERE provider_id=?1 ORDER BY rowid").map_err(error)?;
        let rows = stmt
            .query_map(params![account], |row| row.get::<_, String>(0))
            .map_err(error)?;
        rows.map(|row| serde_json::from_str(&row.map_err(error)?).map_err(error))
            .collect()
    }
    pub fn prepare_credential_operation(
        &self,
        op: &CredentialOperation,
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let mut conn = DurableWriter::new(&mut conn)?;
        let tx = conn.transaction().map_err(error)?;
        let generation: Option<String> = tx
            .query_row(
                "SELECT generation FROM provider_credentials WHERE provider_id=?1",
                params![op.metadata.provider_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?;
        if generation != op.previous.as_ref().map(|value| value.generation.clone()) {
            return Err(DomainError::Validation(
                "secure_storage_conflict: Account changed; retry".into(),
            ));
        }
        if op.phase == "delete" {
            let mut metadata = op.metadata.clone();
            metadata.state = "deleting".into();
            tx.execute("UPDATE provider_credentials SET state='deleting',metadata_json=?2 WHERE provider_id=?1",params![metadata.provider_id,serde_json::to_string(&metadata).map_err(error)?]).map_err(error)?;
        }
        tx.execute("INSERT INTO provider_credential_operations(id,provider_id,operation_json) VALUES(?1,?2,?3)",params![op.id,op.metadata.provider_id,serde_json::to_string(op).map_err(error)?]).map_err(error)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.prepared",
            &op.metadata.provider_id,
            &op.metadata.service_type,
            op.metadata.updated_at,
        )?;
        tx.commit().map_err(error)
    }
    pub fn update_credential_operation(&self, op: &CredentialOperation) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let mut conn = DurableWriter::new(&mut conn)?;
        let tx = conn.transaction().map_err(error)?;
        tx.execute(
            "UPDATE provider_credential_operations SET operation_json=?2 WHERE id=?1",
            params![op.id, serde_json::to_string(op).map_err(error)?],
        )
        .map_err(error)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.operation.updated",
            &op.metadata.provider_id,
            &op.metadata.service_type,
            op.metadata.updated_at,
        )?;
        tx.commit().map_err(error)?;
        Ok(())
    }
    pub fn complete_credential_write(&self, op: &CredentialOperation) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let mut conn = DurableWriter::new(&mut conn)?;
        let tx = conn.transaction().map_err(error)?;
        let generation: Option<String> = tx
            .query_row(
                "SELECT generation FROM provider_credentials WHERE provider_id=?1",
                params![op.metadata.provider_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?;
        if generation != op.previous.as_ref().map(|value| value.generation.clone()) {
            return Err(DomainError::Validation(
                "secure_storage_conflict: Account changed; retry".into(),
            ));
        }
        tx.execute("INSERT INTO provider_credentials(provider_id,metadata_json,generation,parts,state) VALUES(?1,?2,?3,?4,'active') ON CONFLICT(provider_id) DO UPDATE SET metadata_json=excluded.metadata_json,generation=excluded.generation,parts=excluded.parts,state='active'",params![op.metadata.provider_id,serde_json::to_string(&op.metadata).map_err(error)?,op.metadata.generation,op.metadata.parts as i64]).map_err(error)?;
        if let Some(config) = &op.config {
            Self::write_provider(&tx, config)?;
        }
        tx.execute_batch("PRAGMA secure_delete=ON;")
            .map_err(error)?;
        tx.execute(
            "DELETE FROM provider_oauth_tokens WHERE provider_id=?1",
            params![op.metadata.provider_id],
        )
        .map_err(error)?;
        tx.execute(
            "UPDATE provider_credential_vault SET cleanup_pending=1 WHERE singleton=1",
            [],
        )
        .map_err(error)?;
        let mut cleanup = op.clone();
        cleanup.phase = "cleanup-old".into();
        tx.execute(
            "UPDATE provider_credential_operations SET operation_json=?2 WHERE id=?1",
            params![op.id, serde_json::to_string(&cleanup).map_err(error)?],
        )
        .map_err(error)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.updated",
            &op.metadata.provider_id,
            &op.metadata.service_type,
            op.metadata.updated_at,
        )?;
        tx.commit().map_err(error)
    }
    pub fn finish_credential_operation(&self, op: &CredentialOperation) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let mut conn = DurableWriter::new(&mut conn)?;
        let tx = conn.transaction().map_err(error)?;
        if op.phase == "delete" {
            tx.execute_batch("PRAGMA secure_delete=ON;")
                .map_err(error)?;
            tx.execute(
                "DELETE FROM provider_credentials WHERE provider_id=?1 AND generation=?2",
                params![op.metadata.provider_id, op.metadata.generation],
            )
            .map_err(error)?;
            tx.execute(
                "DELETE FROM provider_oauth_tokens WHERE provider_id=?1",
                params![op.metadata.provider_id],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE provider_credential_vault SET cleanup_pending=1 WHERE singleton=1",
                [],
            )
            .map_err(error)?;
            if op.remove_provider {
                tx.execute(
                    "DELETE FROM provider_model_catalog WHERE provider_id=?1",
                    params![op.metadata.provider_id],
                )
                .map_err(error)?;
                tx.execute(
                    "DELETE FROM providers_config WHERE id=?1",
                    params![op.metadata.provider_id],
                )
                .map_err(error)?;
            } else {
                tx.execute("UPDATE providers_config SET api_key_masked='none',status='unconfigured' WHERE id=?1",params![op.metadata.provider_id]).map_err(error)?;
            }
            Self::record_provider_event(
                &tx,
                "provider.credentials.deleted",
                &op.metadata.provider_id,
                &op.metadata.service_type,
                op.metadata.updated_at,
            )?;
        }
        tx.execute(
            "DELETE FROM provider_credential_operations WHERE id=?1",
            params![op.id],
        )
        .map_err(error)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.cleanup.completed",
            &op.metadata.provider_id,
            &op.metadata.service_type,
            op.metadata.updated_at,
        )?;
        tx.commit().map_err(error)
    }
    pub fn checkpoint_credentials(&self) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let pending: i64 = conn
            .query_row(
                "SELECT cleanup_pending FROM provider_credential_vault WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .map_err(error)?;
        if pending == 0 {
            return Ok(());
        }
        let busy: i64 = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
            .map_err(error)?;
        if busy != 0 {
            return Err(DomainError::Validation(
                "secure_storage_cleanup_pending: Database readers prevented WAL cleanup".into(),
            ));
        }
        let mut conn = DurableWriter::new(&mut conn)?;
        let tx = conn.transaction().map_err(error)?;
        tx.execute(
            "UPDATE provider_credential_vault SET cleanup_pending=0 WHERE singleton=1",
            [],
        )
        .map_err(error)?;
        Self::record_provider_event(&tx,"provider.credentials.wal.cleaned","database","metadata",0)?;
        tx.commit().map_err(error)?;
        Ok(())
    }
}
