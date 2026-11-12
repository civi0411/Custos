use super::super::ProviderRepository;
use custos_domain::{ClientApiKeyRecord, DomainError, ProviderConfig};
use rusqlite::params;

impl ProviderRepository {
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, service_type, api_key_masked, status, endpoint_url,
                        default_model, context_window, fast_mode, created_at, updated_at
                 FROM providers_config ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let fast_mode_val: Option<i64> = row.get(8)?;
                Ok(ProviderConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    service_type: row.get(2)?,
                    api_key_masked: row.get(3)?,
                    status: row.get(4)?,
                    endpoint_url: row.get(5)?,
                    default_model: row.get(6)?,
                    context_window: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    fast_mode: fast_mode_val.map(|v| v != 0),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_provider(&self, id: &str) -> Result<Option<ProviderConfig>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, service_type, api_key_masked, status, endpoint_url,
                        default_model, context_window, fast_mode, created_at, updated_at
                 FROM providers_config WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let fast_mode_val: Option<i64> = row.get(8)?;
                Ok(ProviderConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    service_type: row.get(2)?,
                    api_key_masked: row.get(3)?,
                    status: row.get(4)?,
                    endpoint_url: row.get(5)?,
                    default_model: row.get(6)?,
                    context_window: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    fast_mode: fast_mode_val.map(|v| v != 0),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(first) = rows.next() {
            Ok(Some(
                first.map_err(|e| DomainError::Validation(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn save_provider(&self, config: &ProviderConfig) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::write_provider(&tx, config)?;
        Self::record_provider_event(
            &tx,
            "provider.config.updated",
            &config.id,
            &config.service_type,
            config.updated_at,
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub(crate) fn write_provider(
        conn: &rusqlite::Connection,
        config: &ProviderConfig,
    ) -> Result<(), DomainError> {
        let fast_mode_int = config.fast_mode.map(|b| if b { 1i64 } else { 0i64 });
        let context_win_int = config.context_window.map(|w| w as i64);

        conn.execute(
            "INSERT INTO providers_config (id, name, service_type, api_key_masked, status, endpoint_url,
                                          default_model, context_window, fast_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                service_type = excluded.service_type,
                api_key_masked = excluded.api_key_masked,
                status = excluded.status,
                endpoint_url = excluded.endpoint_url,
                default_model = excluded.default_model,
                context_window = excluded.context_window,
                fast_mode = excluded.fast_mode,
                updated_at = excluded.updated_at",
            params![
                config.id,
                config.name,
                config.service_type,
                config.api_key_masked,
                config.status,
                config.endpoint_url,
                config.default_model,
                context_win_int,
                fast_mode_int,
                config.created_at,
                config.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn delete_provider(&self, id: &str) -> Result<bool, DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|_| DomainError::Validation("Provider deletion failed".into()))?;
        tx.execute(
            "DELETE FROM provider_model_catalog WHERE provider_id = ?1",
            params![id],
        )
        .map_err(|_| DomainError::Validation("Provider deletion failed".into()))?;
        let changed = tx
            .execute("DELETE FROM providers_config WHERE id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::record_provider_event(
            &tx,
            "provider.config.deleted",
            id,
            "metadata",
            chrono::Utc::now().timestamp_millis(),
        )?;
        tx.commit()
            .map_err(|_| DomainError::Validation("Provider deletion failed".into()))?;
        Ok(changed > 0)
    }

    pub fn list_client_keys(&self) -> Result<Vec<ClientApiKeyRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, name, token, created_at, revoked FROM client_api_keys WHERE revoked = 0 ORDER BY created_at DESC")
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ClientApiKeyRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    token: row.get(2)?,
                    created_at: row.get(3)?,
                    revoked: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn create_client_key(&self, key: &ClientApiKeyRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO client_api_keys (id, name, token, created_at, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                token = excluded.token,
                revoked = excluded.revoked",
            params![
                key.id,
                key.name,
                key.token,
                key.created_at,
                if key.revoked { 1 } else { 0 }
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn revoke_client_key(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "UPDATE client_api_keys SET revoked = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(crate) fn record_provider_event(
        conn: &rusqlite::Connection,
        topic: &str,
        provider_id: &str,
        service_type: &str,
        updated_at: i64,
    ) -> Result<(), DomainError> {
        let payload =
            serde_json::json!({"provider_id":provider_id,"service_type":service_type,"updated_at":updated_at}).to_string();
        conn.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("provider"),
                topic,
                payload,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }
}

