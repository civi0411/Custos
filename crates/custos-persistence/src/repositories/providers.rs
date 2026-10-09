use crate::connection::DbConnection;
use custos_domain::{
    ClientApiKeyRecord, DomainError, ModelCatalogOption, ModelPricing, OAuthTokenRecord,
    ProviderConfig,
};
use rusqlite::params;

#[derive(Clone)]
pub struct ProviderRepository {
    db: DbConnection,
}

impl ProviderRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
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
            Ok(Some(first.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn save_provider(&self, config: &ProviderConfig) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
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
        let conn = self.db.lock()?;
        let _ = conn.execute("DELETE FROM provider_model_catalog WHERE provider_id = ?1", params![id]);
        let changed = conn
            .execute("DELETE FROM providers_config WHERE id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(changed > 0)
    }

    pub fn save_catalog_models(
        &self,
        provider_id: &str,
        models: &[ModelCatalogOption],
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        for m in models {
            let row_id = format!("{}:{}", provider_id, m.id);
            let ctx_int = m.context_window.map(|w| w as i64);
            let fast_mode_int = if m.supports_fast_mode { 1i64 } else { 0i64 };
            let pricing_json = m.pricing.as_ref().and_then(|p| serde_json::to_string(p).ok());
            let fetched_at = chrono::Utc::now().timestamp_millis();

            conn.execute(
                "INSERT INTO provider_model_catalog (id, provider_id, model_id, label, description, context_window,
                                                    default_effort, supports_fast_mode, pricing_json, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    description = excluded.description,
                    context_window = excluded.context_window,
                    default_effort = excluded.default_effort,
                    supports_fast_mode = excluded.supports_fast_mode,
                    pricing_json = excluded.pricing_json,
                    fetched_at = excluded.fetched_at",
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
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        Ok(())
    }

    pub fn list_catalog_models(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<ModelCatalogOption>, DomainError> {
        let conn = self.db.lock()?;
        let query = if provider_id.is_some() {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json
             FROM provider_model_catalog WHERE provider_id = ?1 ORDER BY label ASC"
        } else {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json
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
            let default_effort: Option<String> = row.get(5)?;
            let supports_fast_mode: i64 = row.get(6)?;
            let pricing_json: Option<String> = row.get(7)?;

            let pricing: Option<ModelPricing> = pricing_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());

            let efforts = if default_effort.is_some() {
                vec!["low".to_string(), "medium".to_string(), "high".to_string()]
            } else {
                Vec::new()
            };

            Ok(ModelCatalogOption {
                id: model_id,
                label,
                description,
                provider_type: p_type,
                is_default: false,
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
            params![key.id, key.name, key.token, key.created_at, if key.revoked { 1 } else { 0 }],
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

    pub fn save_oauth_token(&self, token: &OAuthTokenRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
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

    pub fn get_oauth_token(&self, provider_id: &str) -> Result<Option<OAuthTokenRecord>, DomainError> {
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
            Ok(Some(row.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_oauth_token(&self, provider_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "DELETE FROM provider_oauth_tokens WHERE provider_id = ?1",
            params![provider_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_expiring_oauth_tokens(&self, threshold_ms: i64) -> Result<Vec<OAuthTokenRecord>, DomainError> {
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
}
