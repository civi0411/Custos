use crate::connection::DbConnection;
use custos_domain::{
    ClientApiKeyRecord, DomainError, LocalModelDescriptor, LocalModelFolder,
    LocalModelLoadProfile, ModelCatalogOption, ModelCompatibility, ModelPricing, OAuthTokenRecord,
    ProviderConfig,
};
use rusqlite::params;

#[derive(Clone)]
pub struct ProviderRepository {
    pub(super) db: DbConnection,
}

impl ProviderRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn routing_policy(&self, service: &str) -> Result<serde_json::Value, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        let policy: Option<(bool, String)> = conn.query_row(
            "SELECT round_robin, account_order FROM provider_account_routing WHERE service_type=?1",
            params![service], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| DomainError::Validation(e.to_string()))?;
        let (enabled, order) = policy.unwrap_or((false, "[]".into()));
        let order: Vec<String> = serde_json::from_str(&order).map_err(|_| DomainError::Validation("Invalid account routing policy".into()))?;
        Ok(serde_json::json!({"service_type":service,"round_robin":enabled,"account_order":order}))
    }

    pub fn save_routing_policy(&self, service: &str, enabled: bool, order: &[String]) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn.transaction().map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut seen = std::collections::HashSet::new();
        for account in order {
            let matches: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM providers_config WHERE id=?1 AND service_type=?2)", params![account,service], |row| row.get(0)).map_err(|e| DomainError::Validation(e.to_string()))?;
            if !matches || !seen.insert(account) { return Err(DomainError::Validation("Account order must contain distinct accounts belonging to this provider".into())); }
        }
        let encoded = serde_json::to_string(order).map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute("INSERT INTO provider_account_routing(service_type,round_robin,account_order,cursor) VALUES(?1,?2,?3,0) ON CONFLICT(service_type) DO UPDATE SET round_robin=excluded.round_robin,account_order=excluded.account_order,cursor=0",params![service,enabled,encoded]).map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::record_provider_event(&tx,"provider.routing.updated",service,service,chrono::Utc::now().timestamp_millis())?;
        tx.commit().map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn select_account(&self, service: &str, eligible: &[String]) -> Result<Option<String>, DomainError> {
        use rusqlite::OptionalExtension;
        if eligible.is_empty() { return Ok(None); }
        let mut conn = self.db.lock()?;
        let tx = conn.transaction().map_err(|e| DomainError::Validation(e.to_string()))?;
        let row: Option<(bool,String,i64)> = tx.query_row("SELECT round_robin,account_order,cursor FROM provider_account_routing WHERE service_type=?1",params![service],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|DomainError::Validation(e.to_string()))?;
        let (enabled,encoded,cursor)=row.unwrap_or((false,"[]".into(),0));
        let order: Vec<String>=serde_json::from_str(&encoded).map_err(|_|DomainError::Validation("Invalid account routing policy".into()))?;
        let mut pool=Vec::new();
        for id in eligible {
            let available: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM providers_config p WHERE p.id=?1 AND p.service_type=?2 AND p.status IN ('active','configured','primary','standby') AND (?2='local' OR EXISTS(SELECT 1 FROM provider_credentials c WHERE c.provider_id=p.id AND c.state='active')))",params![id,service],|r|r.get(0)).map_err(|e|DomainError::Validation(e.to_string()))?;
            if available && !pool.contains(id) { pool.push(id.clone()); }
        }
        if pool.is_empty() { return Err(DomainError::Validation("No eligible provider account remains available".into())); }
        pool.sort_by_key(|id| order.iter().position(|candidate|candidate==id).unwrap_or(usize::MAX));
        let index=if enabled { (cursor.max(0) as usize)%pool.len() } else {0};
        let selected=pool[index].clone();
        if enabled {
            tx.execute("UPDATE provider_account_routing SET cursor=?2 WHERE service_type=?1",params![service,((index+1)%pool.len()) as i64]).map_err(|e|DomainError::Validation(e.to_string()))?;
        }
        Self::record_provider_event(&tx,"provider.account.selected",&selected,service,chrono::Utc::now().timestamp_millis())?;
        tx.commit().map_err(|e|DomainError::Validation(e.to_string()))?;
        Ok(Some(selected))
    }

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

    pub(super) fn write_provider(
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
        let tx=conn.transaction().map_err(|_|DomainError::Validation("Provider deletion failed".into()))?;
        tx.execute(
            "DELETE FROM provider_model_catalog WHERE provider_id = ?1",
            params![id],
        ).map_err(|_|DomainError::Validation("Provider deletion failed".into()))?;
        let changed = tx
            .execute("DELETE FROM providers_config WHERE id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::record_provider_event(&tx,"provider.config.deleted",id,"metadata",chrono::Utc::now().timestamp_millis())?;
        tx.commit().map_err(|_|DomainError::Validation("Provider deletion failed".into()))?;
        Ok(changed > 0)
    }

    pub fn save_catalog_models(
        &self,
        provider_id: &str,
        models: &[ModelCatalogOption],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let payload = serde_json::to_string(
            &serde_json::json!({"provider_id": provider_id, "models": models}),
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("catalog"),
                "provider.models.probed",
                payload,
                now
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "DELETE FROM provider_model_catalog WHERE provider_id = ?1",
            params![provider_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        for m in models {
            let row_id = format!("{}:{}", provider_id, m.id);
            let ctx_int = m.context_window.map(|w| w as i64);
            let fast_mode_int = if m.supports_fast_mode { 1i64 } else { 0i64 };
            let pricing_json = m
                .pricing
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok());
            let fetched_at = chrono::Utc::now().timestamp_millis();
            let capabilities_json = serde_json::to_string(&serde_json::json!({"efforts":m.efforts,"is_default":m.is_default})).map_err(|e|DomainError::Validation(e.to_string()))?;

            tx.execute(
                "INSERT INTO provider_model_catalog (id, provider_id, model_id, label, description, context_window,
                                                    default_effort, supports_fast_mode, pricing_json, fetched_at, capabilities_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    description = excluded.description,
                    context_window = excluded.context_window,
                    default_effort = excluded.default_effort,
                    supports_fast_mode = excluded.supports_fast_mode,
                    pricing_json = excluded.pricing_json,
                    fetched_at = excluded.fetched_at,
                    capabilities_json = excluded.capabilities_json",
                params![
                    row_id,
                    provider_id,
                    m.id,
                    m.label,
                    m.description,
                    ctx_int,
                    m.default_effort,
                    fast_mode_int,
                    pricing_json,
                    fetched_at,
                    capabilities_json,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn catalog_fetched_at(&self, provider_id: &str) -> Result<i64, DomainError> {
        let conn = self.db.lock()?;
        conn.query_row("SELECT COALESCE(MAX(fetched_at), 0) FROM provider_model_catalog WHERE provider_id = ?1",
            params![provider_id], |row| row.get(0))
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn list_catalog_models(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<ModelCatalogOption>, DomainError> {
        let conn = self.db.lock()?;
        let query = if provider_id.is_some() {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json, capabilities_json
             FROM provider_model_catalog WHERE provider_id = ?1 ORDER BY label ASC"
        } else {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json, capabilities_json
             FROM provider_model_catalog ORDER BY label ASC"
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let map_fn = |row: &rusqlite::Row| {
            let model_id: String = row.get(0)?;
            let label: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let p_type: String = row.get(3)?;
            let context_window: Option<i64> = row.get(4)?;
            let mut default_effort: Option<String> = row.get(5)?;
            let supports_fast_mode: i64 = row.get(6)?;
            let pricing_json: Option<String> = row.get(7)?;

            let pricing: Option<ModelPricing> = pricing_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());

            let capabilities_json: String = row.get(8)?;
            let capabilities: serde_json::Value = serde_json::from_str(&capabilities_json).map_err(|error|rusqlite::Error::FromSqlConversionFailure(8,rusqlite::types::Type::Text,Box::new(error)))?;
            let efforts: Vec<String> = serde_json::from_value(capabilities.get("efforts").cloned().unwrap_or_else(||serde_json::json!([]))).map_err(|error|rusqlite::Error::FromSqlConversionFailure(8,rusqlite::types::Type::Text,Box::new(error)))?;
            if default_effort.as_ref().is_some_and(|default| !efforts.contains(default)) { default_effort = None; }
            let is_default = capabilities.get("is_default").and_then(|value|value.as_bool()).unwrap_or(false);

            Ok(ModelCatalogOption {
                id: model_id,
                label,
                description,
                provider_type: p_type,
                is_default,
                default_effort,
                efforts,
                supports_fast_mode: supports_fast_mode != 0,
                context_window: context_window.map(|w| w as u64),
                pricing,
            })
        };

        let mut list = Vec::new();
        if let Some(pid) = provider_id {
            let rows = stmt
                .query_map(params![pid], map_fn)
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            for r in rows {
                list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
            }
        } else {
            let rows = stmt
                .query_map([], map_fn)
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            for r in rows {
                list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
            }
        }

        Ok(list)
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

    #[cfg(test)]
    pub fn save_oauth_token(&self, token: &OAuthTokenRecord) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::write_oauth_token(&tx, token)?;
        Self::record_provider_event(
            &tx,
            "provider.credentials.updated",
            &token.provider_id,
            &token.service_type,
            token.updated_at,
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    #[cfg(test)]
    pub fn save_oauth_connection(
        &self,
        token: &OAuthTokenRecord,
        config: &ProviderConfig,
    ) -> Result<(), DomainError> {
        if token.provider_id != config.id || token.service_type != config.service_type {
            return Err(DomainError::Validation(
                "OAuth credential and provider identity mismatch".into(),
            ));
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::write_oauth_token(&tx, token)?;
        Self::write_provider(&tx, config)?;
        Self::record_provider_event(
            &tx,
            "provider.oauth.connected",
            &config.id,
            &config.service_type,
            config.updated_at,
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub(super) fn record_provider_event(
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

    #[cfg(test)]
    fn write_oauth_token(
        conn: &rusqlite::Connection,
        token: &OAuthTokenRecord,
    ) -> Result<(), DomainError> {
        conn.execute(
            "INSERT INTO provider_oauth_tokens (
                provider_id, service_type, access_token, refresh_token,
                expires_at, token_type, scope, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(provider_id) DO UPDATE SET
                service_type = excluded.service_type,
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                expires_at = excluded.expires_at,
                token_type = excluded.token_type,
                scope = excluded.scope,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            params![
                token.provider_id,
                token.service_type,
                token.access_token,
                token.refresh_token,
                token.expires_at,
                token.token_type,
                token.scope,
                token.created_at,
                token.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn get_oauth_token(
        &self,
        provider_id: &str,
    ) -> Result<Option<OAuthTokenRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT provider_id, service_type, access_token, refresh_token,
                        expires_at, token_type, scope, created_at, updated_at
                 FROM provider_oauth_tokens WHERE provider_id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![provider_id], |row| {
                Ok(OAuthTokenRecord {
                    provider_id: row.get(0)?,
                    service_type: row.get(1)?,
                    access_token: row.get(2)?,
                    refresh_token: row.get(3)?,
                    expires_at: row.get(4)?,
                    token_type: row.get(5)?,
                    scope: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(row) = rows.next() {
            Ok(Some(
                row.map_err(|e| DomainError::Validation(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    pub fn delete_oauth_token(&self, provider_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "DELETE FROM provider_oauth_tokens WHERE provider_id = ?1",
            params![provider_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    #[cfg(test)]
    pub fn list_expiring_oauth_tokens(
        &self,
        threshold_ms: i64,
    ) -> Result<Vec<OAuthTokenRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT provider_id, service_type, access_token, refresh_token,
                        expires_at, token_type, scope, created_at, updated_at
                 FROM provider_oauth_tokens WHERE expires_at <= ?1 AND refresh_token IS NOT NULL",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![threshold_ms], |row| {
                Ok(OAuthTokenRecord {
                    provider_id: row.get(0)?,
                    service_type: row.get(1)?,
                    access_token: row.get(2)?,
                    refresh_token: row.get(3)?,
                    expires_at: row.get(4)?,
                    token_type: row.get(5)?,
                    scope: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    #[cfg(test)]
    pub fn list_oauth_tokens(&self) -> Result<Vec<OAuthTokenRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT provider_id, service_type, access_token, refresh_token,
                        expires_at, token_type, scope, created_at, updated_at
                 FROM provider_oauth_tokens ORDER BY updated_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(OAuthTokenRecord {
                    provider_id: row.get(0)?,
                    service_type: row.get(1)?,
                    access_token: row.get(2)?,
                    refresh_token: row.get(3)?,
                    expires_at: row.get(4)?,
                    token_type: row.get(5)?,
                    scope: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    // ── Local Model Folders & Model Cache ─────────────────────────────────────

    pub fn save_local_folder(&self, folder: &LocalModelFolder) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO local_model_folders(id, folder_path, label, is_active, auto_scan, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                folder_path = excluded.folder_path,
                label = excluded.label,
                is_active = excluded.is_active,
                auto_scan = excluded.auto_scan,
                updated_at = excluded.updated_at",
            params![
                folder.id,
                folder.folder_path,
                folder.label,
                folder.is_active as i32,
                folder.auto_scan as i32,
                folder.created_at,
                folder.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_local_folders(&self) -> Result<Vec<LocalModelFolder>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, folder_path, label, is_active, auto_scan, created_at, updated_at
                 FROM local_model_folders ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(LocalModelFolder {
                    id: row.get(0)?,
                    folder_path: row.get(1)?,
                    label: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    auto_scan: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_local_folder(&self, id: &str) -> Result<Option<LocalModelFolder>, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT id, folder_path, label, is_active, auto_scan, created_at, updated_at
             FROM local_model_folders WHERE id = ?1",
            params![id],
            |row| {
                Ok(LocalModelFolder {
                    id: row.get(0)?,
                    folder_path: row.get(1)?,
                    label: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    auto_scan: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn delete_local_folder(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM local_model_folders WHERE id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        conn.execute("DELETE FROM local_model_cache WHERE folder_id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn upsert_cached_model(&self, model: &LocalModelDescriptor) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let compat_str = match model.compatibility {
            ModelCompatibility::Green => "green",
            ModelCompatibility::Yellow => "yellow",
            ModelCompatibility::Red => "red",
        };
        conn.execute(
            "INSERT INTO local_model_cache(
                file_path, folder_id, file_name, file_size_bytes, file_mtime,
                architecture, model_name, context_length, quantization, chat_template,
                total_memory_required, compatibility, scanned_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(file_path) DO UPDATE SET
                folder_id = excluded.folder_id,
                file_name = excluded.file_name,
                file_size_bytes = excluded.file_size_bytes,
                file_mtime = excluded.file_mtime,
                architecture = excluded.architecture,
                model_name = excluded.model_name,
                context_length = excluded.context_length,
                quantization = excluded.quantization,
                chat_template = excluded.chat_template,
                total_memory_required = excluded.total_memory_required,
                compatibility = excluded.compatibility,
                scanned_at = excluded.scanned_at",
            params![
                model.file_path,
                model.folder_id,
                model.file_name,
                model.file_size_bytes as i64,
                model.file_mtime,
                model.architecture,
                model.model_name,
                model.context_length as i64,
                model.quantization,
                model.chat_template,
                model.total_memory_required as i64,
                compat_str,
                model.scanned_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_cached_models(&self, folder_id: Option<&str>) -> Result<Vec<LocalModelDescriptor>, DomainError> {
        let conn = self.db.lock()?;
        let query = if folder_id.is_some() {
            "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                    architecture, model_name, context_length, quantization, chat_template,
                    total_memory_required, compatibility, scanned_at
             FROM local_model_cache WHERE folder_id = ?1 ORDER BY model_name ASC"
        } else {
            "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                    architecture, model_name, context_length, quantization, chat_template,
                    total_memory_required, compatibility, scanned_at
             FROM local_model_cache ORDER BY model_name ASC"
        };

        let mut stmt = conn.prepare(query).map_err(|e| DomainError::Validation(e.to_string()))?;
        let mapper = |row: &rusqlite::Row| {
            let compat_str: String = row.get(11)?;
            let compatibility = match compat_str.as_str() {
                "yellow" => ModelCompatibility::Yellow,
                "red" => ModelCompatibility::Red,
                _ => ModelCompatibility::Green,
            };
            Ok(LocalModelDescriptor {
                file_path: row.get(0)?,
                folder_id: row.get(1)?,
                file_name: row.get(2)?,
                file_size_bytes: row.get::<_, i64>(3)? as u64,
                file_mtime: row.get(4)?,
                architecture: row.get(5)?,
                model_name: row.get(6)?,
                context_length: row.get::<_, i64>(7)? as u64,
                quantization: row.get(8)?,
                chat_template: row.get(9)?,
                total_memory_required: row.get::<_, i64>(10)? as u64,
                compatibility,
                scanned_at: row.get(12)?,
            })
        };

        let rows = if let Some(fid) = folder_id {
            stmt.query_map(params![fid], mapper)
        } else {
            stmt.query_map([], mapper)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_cached_model_by_path(&self, file_path: &str) -> Result<Option<LocalModelDescriptor>, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                    architecture, model_name, context_length, quantization, chat_template,
                    total_memory_required, compatibility, scanned_at
             FROM local_model_cache WHERE file_path = ?1",
            params![file_path],
            |row| {
                let compat_str: String = row.get(11)?;
                let compatibility = match compat_str.as_str() {
                    "yellow" => ModelCompatibility::Yellow,
                    "red" => ModelCompatibility::Red,
                    _ => ModelCompatibility::Green,
                };
                Ok(LocalModelDescriptor {
                    file_path: row.get(0)?,
                    folder_id: row.get(1)?,
                    file_name: row.get(2)?,
                    file_size_bytes: row.get::<_, i64>(3)? as u64,
                    file_mtime: row.get(4)?,
                    architecture: row.get(5)?,
                    model_name: row.get(6)?,
                    context_length: row.get::<_, i64>(7)? as u64,
                    quantization: row.get(8)?,
                    chat_template: row.get(9)?,
                    total_memory_required: row.get::<_, i64>(10)? as u64,
                    compatibility,
                    scanned_at: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn get_local_model_load_profile(&self, model_path: &str) -> Result<Option<LocalModelLoadProfile>, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT model_path, revision, context_size, gpu_mode, gpu_layers, eval_batch_size,
                    cpu_threads, flash_attention, mmap, mlock, idle_timeout_seconds, updated_at
             FROM local_model_load_profiles WHERE model_path = ?1",
            params![model_path],
            |row| Ok(LocalModelLoadProfile {
                model_path: row.get(0)?,
                revision: row.get::<_, i64>(1)? as u64,
                context_size: row.get::<_, i64>(2)? as u64,
                gpu_mode: row.get(3)?,
                gpu_layers: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                eval_batch_size: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
                cpu_threads: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                flash_attention: row.get::<_, Option<i32>>(7)?.map(|v| v != 0),
                mmap: row.get::<_, i32>(8)? != 0,
                mlock: row.get::<_, i32>(9)? != 0,
                idle_timeout_seconds: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
                updated_at: row.get(11)?,
            }),
        ).optional().map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn save_local_model_load_profile(&self, profile: &LocalModelLoadProfile) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO local_model_load_profiles(model_path, revision, context_size, gpu_mode,
                gpu_layers, eval_batch_size, cpu_threads, flash_attention, mmap, mlock,
                idle_timeout_seconds, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(model_path) DO UPDATE SET revision=excluded.revision,
                context_size=excluded.context_size, gpu_mode=excluded.gpu_mode,
                gpu_layers=excluded.gpu_layers, eval_batch_size=excluded.eval_batch_size,
                cpu_threads=excluded.cpu_threads, flash_attention=excluded.flash_attention,
                mmap=excluded.mmap, mlock=excluded.mlock,
                idle_timeout_seconds=excluded.idle_timeout_seconds, updated_at=excluded.updated_at",
            params![profile.model_path, profile.revision as i64, profile.context_size as i64,
                profile.gpu_mode, profile.gpu_layers.map(i64::from),
                profile.eval_batch_size.map(i64::from), profile.cpu_threads.map(i64::from),
                profile.flash_attention.map(i32::from), profile.mmap as i32, profile.mlock as i32,
                profile.idle_timeout_seconds.map(|v| v as i64), profile.updated_at],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod catalog_snapshot_tests {
    use super::*;
    #[test]
    fn oauth_connection_is_atomic_and_outbox_is_redacted() {
        let store = crate::SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.providers();
        let token = OAuthTokenRecord {
            provider_id: "antigravity".into(),
            service_type: "antigravity".into(),
            access_token: "secret-access".into(),
            refresh_token: Some("secret-refresh".into()),
            expires_at: 10000,
            token_type: "Bearer".into(),
            scope: Some("cloud-platform".into()),
            created_at: 1,
            updated_at: 1,
        };
        let config = ProviderConfig {
            id: "antigravity".into(),
            service_type: "antigravity".into(),
            name: "person@example.com".into(),
            api_key_masked: "oauth".into(),
            status: "active".into(),
            endpoint_url: Some("https://cloudcode-pa.googleapis.com".into()),
            default_model: None,
            context_window: None,
            fast_mode: None,
            created_at: 1,
            updated_at: 1,
        };
        repo.save_oauth_connection(&token, &config).unwrap();
        assert_eq!(
            repo.get_provider("antigravity").unwrap().unwrap().name,
            "person@example.com"
        );
        {
            let conn = repo.db.lock().unwrap();
            let payload: String = conn
                .query_row(
                    "SELECT payload FROM outbox_messages WHERE topic='provider.oauth.connected'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!payload.contains("secret-access"));
            assert!(!payload.contains("secret-refresh"));
            conn.execute_batch("CREATE TRIGGER reject_provider BEFORE INSERT ON providers_config BEGIN SELECT RAISE(ABORT, 'test rejection'); END;").unwrap();
        }
        let mut changed = token.clone();
        changed.access_token = "replacement".into();
        assert!(repo.save_oauth_connection(&changed, &config).is_err());
        assert_eq!(
            repo.get_oauth_token("antigravity")
                .unwrap()
                .unwrap()
                .access_token,
            "secret-access"
        );
    }

    #[test]
    fn verified_snapshot_replaces_models_and_records_outbox_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            crate::SqliteTaskStore::new(dir.path().join("catalog.db").to_str().unwrap()).unwrap();
        let repo = store.providers();
        let model = |id: &str| ModelCatalogOption {
            id: id.into(),
            label: id.into(),
            description: None,
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: false,
            context_window: None,
            pricing: None,
        };
        repo.save_catalog_models("verified-probe:account", &[model("removed"), model("kept")])
            .unwrap();
        let timestamp = repo.catalog_fetched_at("verified-probe:account").unwrap();
        assert!(timestamp > 0);
        repo.save_catalog_models("verified-probe:account", &[model("kept")])
            .unwrap();
        let cached = repo
            .list_catalog_models(Some("verified-probe:account"))
            .unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].id, "kept");
        assert!(repo
            .list_catalog_models(Some("other-account"))
            .unwrap()
            .is_empty());
        let conn = repo.db.lock().unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM outbox_messages WHERE topic = 'provider.models.probed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(events, 2);
    }
    #[test]
    fn routing_policy_and_cursor_survive_restart() {
        let dir=tempfile::tempdir().unwrap();
        let path=dir.path().join("routing.db");
        {
            let store=crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
            let repo=store.providers();
            for id in ["A","B"] {
                let config=ProviderConfig{id:id.into(),name:id.into(),service_type:"local".into(),api_key_masked:"none".into(),status:"active".into(),endpoint_url:None,default_model:None,context_window:None,fast_mode:None,created_at:0,updated_at:0};
                repo.save_provider(&config).unwrap();
            }
            repo.save_routing_policy("local",true,&["B".into(),"A".into()]).unwrap();
            assert_eq!(repo.select_account("local",&["A".into(),"B".into()]).unwrap().as_deref(),Some("B"));
        }
        let store=crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
        let repo=store.providers();
        assert_eq!(repo.routing_policy("local").unwrap()["round_robin"],true);
        assert_eq!(repo.select_account("local",&["A".into(),"B".into()]).unwrap().as_deref(),Some("A"));
        repo.save_routing_policy("local",false,&["A".into(),"B".into()]).unwrap();
        assert_eq!(repo.select_account("local",&["A".into(),"B".into()]).unwrap().as_deref(),Some("A"));
        let conn=repo.db.lock().unwrap();
        let count:i64=conn.query_row("SELECT COUNT(*) FROM outbox_messages WHERE topic='provider.account.selected'",[],|r|r.get(0)).unwrap();
        assert_eq!(count,3);
    }

    #[test]
    fn local_model_folders_and_cache_crud() {
        let store = crate::SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.providers();
        let folder = LocalModelFolder {
            id: "fld-1".into(),
            folder_path: "/tmp/models".into(),
            label: Some("Test Folder".into()),
            is_active: true,
            auto_scan: true,
            created_at: 100,
            updated_at: 100,
        };
        repo.save_local_folder(&folder).unwrap();
        let list = repo.list_local_folders().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "fld-1");
        assert_eq!(list[0].folder_path, "/tmp/models");

        let model = LocalModelDescriptor {
            file_path: "/tmp/models/qwen2.5-7b.gguf".into(),
            folder_id: "fld-1".into(),
            file_name: "qwen2.5-7b.gguf".into(),
            file_size_bytes: 4_500_000_000,
            file_mtime: 200,
            architecture: "qwen2".into(),
            model_name: "Qwen2.5-7B-Instruct".into(),
            context_length: 32768,
            quantization: "Q4_K_M".into(),
            chat_template: Some("<chat_template>".into()),
            total_memory_required: 5_200_000_000,
            compatibility: ModelCompatibility::Green,
            scanned_at: 300,
        };
        repo.upsert_cached_model(&model).unwrap();
        let cached = repo.list_cached_models(None).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].model_name, "Qwen2.5-7B-Instruct");
        assert_eq!(cached[0].compatibility, ModelCompatibility::Green);

        let by_path = repo.get_cached_model_by_path("/tmp/models/qwen2.5-7b.gguf").unwrap().unwrap();
        assert_eq!(by_path.file_size_bytes, 4_500_000_000);

        let profile = LocalModelLoadProfile {
            model_path: model.file_path.clone(), revision: 1, context_size: 4096,
            gpu_mode: "auto".into(), gpu_layers: None, eval_batch_size: Some(512),
            cpu_threads: Some(8), flash_attention: Some(true), mmap: true, mlock: false,
            idle_timeout_seconds: Some(900), updated_at: 400,
        };
        repo.save_local_model_load_profile(&profile).unwrap();
        assert_eq!(repo.get_local_model_load_profile(&model.file_path).unwrap().unwrap(), profile);

        repo.delete_local_folder("fld-1").unwrap();
        assert!(repo.list_local_folders().unwrap().is_empty());
        assert!(repo.list_cached_models(None).unwrap().is_empty());
        assert!(repo.get_local_model_load_profile(&model.file_path).unwrap().is_none());
    }

}

#[cfg(test)]
mod catalog_capability_tests {
    use super::*;
    #[test]
    fn exact_capabilities_survive_restart_and_empty_refresh() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("catalog-capabilities.db");
        let model=ModelCatalogOption{id:"gpt-6.1-sol".into(),label:"GPT-6.1 Sol".into(),provider_type:"openai".into(),description:None,is_default:true,default_effort:Some("low".into()),efforts:vec!["low".into(),"xhigh".into(),"max".into(),"ultra".into()],supports_fast_mode:false,context_window:Some(272000),pricing:None};
        {
            let store=crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
            store.providers().save_catalog_models("account-scope",&[model.clone()]).unwrap();
        }
        let store=crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
        let saved=store.providers().list_catalog_models(Some("account-scope")).unwrap();
        assert_eq!(saved[0].efforts,model.efforts);assert_eq!(saved[0].default_effort,model.default_effort);assert!(saved[0].is_default);
        store.providers().save_catalog_models("account-scope",&[]).unwrap();
        assert!(store.providers().list_catalog_models(Some("account-scope")).unwrap().is_empty());
    }
    #[test]
    fn legacy_catalog_migration_is_idempotent_and_does_not_invent_efforts() {
        let conn=rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0025_provider_model_catalog.sql")).unwrap();
        conn.execute("INSERT INTO provider_model_catalog(id,provider_id,model_id,label,default_effort,fetched_at) VALUES('legacy','A','old','Old','medium',1)",[]).unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        let metadata:String=conn.query_row("SELECT capabilities_json FROM provider_model_catalog WHERE id='legacy'",[],|row|row.get(0)).unwrap();
        assert_eq!(metadata,"{}");
    }
}
