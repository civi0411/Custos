use super::super::ProviderRepository;
use custos_domain::{DomainError, OAuthTokenRecord, ProviderConfig};
use rusqlite::params;

impl ProviderRepository {
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
    pub fn has_oauth_scope(
        &self,
        provider_id: &str,
        required_scope: &str,
    ) -> Result<bool, DomainError> {
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
                if scopes.contains("email.send")
                    || scopes.contains("mail.send")
                    || scopes.contains("*")
                {
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
                if scopes.contains("calendar.events")
                    || scopes.contains("calendar.write")
                    || scopes.contains("*")
                {
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

