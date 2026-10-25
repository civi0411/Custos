use super::ProviderRepository;
use custos_domain::{ClientApiKeyRecord, DomainError, OAuthTokenRecord, ProviderConfig};
use rusqlite::params;

impl ProviderRepository {
    pub fn routing_policy(&self, service: &str) -> Result<serde_json::Value, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        let policy: Option<(bool, String)> = conn
            .query_row(
                "SELECT round_robin, account_order FROM provider_account_routing WHERE service_type=?1",
                params![service],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let (enabled, order) = policy.unwrap_or_else(|| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM providers_config WHERE (service_type=?1 OR (service_type IN ('antigravity','gemini') AND ?1 IN ('antigravity','gemini'))) AND status IN ('active','configured','primary','standby')",
                    params![service],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            (count > 1, "[]".into())
        });
        let order: Vec<String> = serde_json::from_str(&order)
            .map_err(|_| DomainError::Validation("Invalid account routing policy".into()))?;
        Ok(serde_json::json!({"service_type":service,"round_robin":enabled,"account_order":order}))
    }

    pub fn save_routing_policy(
        &self,
        service: &str,
        enabled: bool,
        order: &[String],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut seen = std::collections::HashSet::new();
        for account in order {
            let matches: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM providers_config WHERE id=?1 AND (service_type=?2 OR (service_type IN ('antigravity','gemini') AND ?2 IN ('antigravity','gemini'))))",
                    params![account, service],
                    |row| row.get(0),
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            if !matches || !seen.insert(account) {
                return Err(DomainError::Validation(
                    "Account order must contain distinct accounts belonging to this provider"
                        .into(),
                ));
            }
        }
        let encoded = serde_json::to_string(order)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO provider_account_routing(service_type,round_robin,account_order,cursor) VALUES(?1,?2,?3,0) ON CONFLICT(service_type) DO UPDATE SET round_robin=excluded.round_robin,account_order=excluded.account_order,cursor=0",
            params![service, enabled, encoded],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::record_provider_event(
            &tx,
            "provider.routing.updated",
            service,
            service,
            chrono::Utc::now().timestamp_millis(),
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn select_account(
        &self,
        service: &str,
        eligible: &[String],
    ) -> Result<Option<String>, DomainError> {
        use rusqlite::OptionalExtension;
        if eligible.is_empty() {
            return Ok(None);
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let row: Option<(bool, String, i64)> = tx
            .query_row(
                "SELECT round_robin,account_order,cursor FROM provider_account_routing WHERE service_type=?1",
                params![service],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut pool = Vec::new();
        for id in eligible {
            let available: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM providers_config p WHERE p.id=?1 AND (p.service_type=?2 OR (p.service_type IN ('antigravity','gemini') AND ?2 IN ('antigravity','gemini'))) AND p.status IN ('active','configured','primary','standby') AND (?2='local' OR EXISTS(SELECT 1 FROM provider_credentials c WHERE c.provider_id=p.id AND c.state='active')))",
                    params![id, service],
                    |r| r.get(0),
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            if available && !pool.contains(id) {
                pool.push(id.clone());
            }
        }
        if pool.is_empty() {
            return Err(DomainError::Validation(
                "No eligible provider account remains available".into(),
            ));
        }
        let (enabled, encoded, cursor) = row.unwrap_or_else(|| (pool.len() > 1, "[]".into(), 0));
        let order: Vec<String> = serde_json::from_str(&encoded)
            .map_err(|_| DomainError::Validation("Invalid account routing policy".into()))?;
        pool.sort_by_key(|id| {
            order
                .iter()
                .position(|candidate| candidate == id)
                .unwrap_or(usize::MAX)
        });
        let index = if enabled {
            (cursor.max(0) as usize) % pool.len()
        } else {
            0
        };
        let selected = pool[index].clone();
        if enabled {
            tx.execute(
                "INSERT INTO provider_account_routing(service_type,round_robin,account_order,cursor) VALUES(?1,1,'[]',?2) ON CONFLICT(service_type) DO UPDATE SET cursor=?2",
                params![service, ((index + 1) % pool.len()) as i64],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        Self::record_provider_event(
            &tx,
            "provider.account.selected",
            &selected,
            service,
            chrono::Utc::now().timestamp_millis(),
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
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

    /// Checks if a provider has an active (non-expired) OAuth token granting the requested scope.
    pub fn has_oauth_scope(&self, provider_id: &str, required_scope: &str) -> Result<bool, DomainError> {
        let token = match self.get_oauth_token(provider_id)? {
            Some(t) => t,
            None => return Ok(false),
        };
        if token.is_expired(0) {
            return Ok(false);
        }
        let Some(scopes) = token.scope.as_deref() else {
            return Ok(false);
        };
        let has = scopes
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .any(|s| s == required_scope || s == "*");
        Ok(has)
    }

    /// Evaluates connector capabilities for a provider based on its active OAuth token scopes.
    pub fn get_connector_capabilities(
        &self,
        provider_id: &str,
    ) -> Result<Vec<custos_domain::ResourceActionCapability>, DomainError> {
        let token = self.get_oauth_token(provider_id)?;
        let mut caps = Vec::new();
        match token {
            Some(t) if !t.is_expired(0) => {
                let scopes_str = t.scope.unwrap_or_default();
                let scopes: std::collections::HashSet<&str> = scopes_str
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter(|s| !s.is_empty())
                    .collect();

                // email capability
                if scopes.contains("email.send") || scopes.contains("mail.send") || scopes.contains("*") {
                    caps.push(custos_domain::ResourceActionCapability {
                        id: "connector.email".into(),
                        label: "Email Dispatch".into(),
                        status: "available".into(),
                        reason: None,
                        primary: true,
                        shortcut: None,
                    });
                } else {
                    caps.push(custos_domain::ResourceActionCapability {
                        id: "connector.email".into(),
                        label: "Email Dispatch".into(),
                        status: "unavailable".into(),
                        reason: Some("Missing 'email.send' scope on OAuth credential".into()),
                        primary: false,
                        shortcut: None,
                    });
                }

                // calendar capability
                if scopes.contains("calendar.events") || scopes.contains("calendar.write") || scopes.contains("*") {
                    caps.push(custos_domain::ResourceActionCapability {
                        id: "connector.calendar".into(),
                        label: "Calendar Schedule".into(),
                        status: "available".into(),
                        reason: None,
                        primary: true,
                        shortcut: None,
                    });
                } else {
                    caps.push(custos_domain::ResourceActionCapability {
                        id: "connector.calendar".into(),
                        label: "Calendar Schedule".into(),
                        status: "unavailable".into(),
                        reason: Some("Missing 'calendar.events' scope on OAuth credential".into()),
                        primary: false,
                        shortcut: None,
                    });
                }
            }
            Some(_) => {
                caps.push(custos_domain::ResourceActionCapability {
                    id: "connector.email".into(),
                    label: "Email Dispatch".into(),
                    status: "unavailable".into(),
                    reason: Some("OAuth token has expired; re-authentication required".into()),
                    primary: false,
                    shortcut: None,
                });
                caps.push(custos_domain::ResourceActionCapability {
                    id: "connector.calendar".into(),
                    label: "Calendar Schedule".into(),
                    status: "unavailable".into(),
                    reason: Some("OAuth token has expired; re-authentication required".into()),
                    primary: false,
                    shortcut: None,
                });
            }
            None => {
                caps.push(custos_domain::ResourceActionCapability {
                    id: "connector.email".into(),
                    label: "Email Dispatch".into(),
                    status: "unavailable".into(),
                    reason: Some("No OAuth credential configured for provider".into()),
                    primary: false,
                    shortcut: None,
                });
                caps.push(custos_domain::ResourceActionCapability {
                    id: "connector.calendar".into(),
                    label: "Calendar Schedule".into(),
                    status: "unavailable".into(),
                    reason: Some("No OAuth credential configured for provider".into()),
                    primary: false,
                    shortcut: None,
                });
            }
        }
        Ok(caps)
    }
}
