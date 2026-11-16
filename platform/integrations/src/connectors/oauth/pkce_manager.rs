//! RFC 7636 Proof Key for Code Exchange (PKCE) flow manager.

use super::provider_defaults::{
    google_oauth_secret, identity_token, oauth_identity, oauth_settings, service_google,
};
use super::types::{
    resolve_openai_client_id, resolve_openai_scope, OAuthAuthorizeUrlResult, PkceChallenge,
    TokenEndpointResponse, DEFAULT_OPENAI_AUTH_URL, DEFAULT_OPENAI_TOKEN_URL, DEFAULT_REDIRECT_URI,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use custos_domain::{DomainError, OAuthTokenRecord};
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Clone)]
pub struct OAuthPkceManager {
    http_client: reqwest::Client,
    auth_endpoint: String,
    token_endpoint: String,
}

impl Default for OAuthPkceManager {
    fn default() -> Self {
        Self::new(DEFAULT_OPENAI_AUTH_URL, DEFAULT_OPENAI_TOKEN_URL)
    }
}

impl OAuthPkceManager {
    pub fn new(auth_endpoint: impl Into<String>, token_endpoint: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            auth_endpoint: auth_endpoint.into(),
            token_endpoint: token_endpoint.into(),
        }
    }

    /// Generates high-entropy PKCE code_verifier and SHA-256 code_challenge per RFC 7636.
    pub fn generate_pkce_challenge() -> PkceChallenge {
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        let mut state_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut state_bytes);
        let state = URL_SAFE_NO_PAD.encode(state_bytes);

        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        PkceChallenge {
            code_verifier,
            code_challenge,
            state,
        }
    }

    /// Builds authorization URL with PKCE parameters.
    pub fn build_authorization_url(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scope: Option<&str>,
        challenge: &PkceChallenge,
    ) -> Result<OAuthAuthorizeUrlResult, DomainError> {
        let mut parsed_url = Url::parse(&self.auth_endpoint)
            .map_err(|e| DomainError::Validation(format!("Invalid auth endpoint URL: {e}")))?;

        let effective_client_id = if client_id.is_empty() || client_id == "custos-openai-desktop" {
            resolve_openai_client_id()
        } else {
            client_id.to_string()
        };

        let effective_redirect_uri = if redirect_uri.is_empty() {
            DEFAULT_REDIRECT_URI
        } else {
            redirect_uri
        };

        let default_scope = resolve_openai_scope();
        let scope_str = scope.unwrap_or(&default_scope);

        parsed_url
            .query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &effective_client_id)
            .append_pair("redirect_uri", effective_redirect_uri)
            .append_pair("scope", scope_str)
            .append_pair("code_challenge", &challenge.code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &challenge.state);
        if self.auth_endpoint.contains("claude.ai") {
            parsed_url
                .query_pairs_mut()
                .append_pair("code", "true")
                .append_pair("prompt", "login");
        }
        if self.auth_endpoint.contains("accounts.google.com") {
            parsed_url
                .query_pairs_mut()
                .append_pair("access_type", "offline")
                .append_pair("prompt", "select_account consent");
        }

        Ok(OAuthAuthorizeUrlResult {
            authorization_url: parsed_url.to_string(),
            code_verifier: challenge.code_verifier.clone(),
            code_challenge: challenge.code_challenge.clone(),
            state: challenge.state.clone(),
            redirect_uri: effective_redirect_uri.to_string(),
        })
    }

    /// Extracts authorization code from raw input, which may be either a raw code
    /// or a full callback redirect URL (e.g., http://localhost:1420/auth/callback?code=xxx&state=yyy).
    pub fn extract_code_from_input(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            if let Ok(parsed) = Url::parse(trimmed) {
                for (k, v) in parsed.query_pairs() {
                    if k == "code" {
                        return v.to_string();
                    }
                }
            }
        }

        if let Some(pos) = trimmed.find("code=") {
            let after = &trimmed[pos + 5..];
            let code_part = after.split('&').next().unwrap_or(after);
            return code_part.trim().to_string();
        }

        trimmed.to_string()
    }

    /// Exchanges authorization code and code_verifier for OAuth tokens at the token endpoint.
    pub async fn exchange_code(
        &self,
        provider_id: &str,
        service_type: &str,
        client_id: &str,
        raw_code_or_url: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenRecord, DomainError> {
        self.exchange_code_with_identity(
            provider_id,
            service_type,
            client_id,
            raw_code_or_url,
            redirect_uri,
            code_verifier,
        )
        .await
        .map(|v| v.0)
    }

    pub async fn exchange_code_with_identity(
        &self,
        provider_id: &str,
        service_type: &str,
        client_id: &str,
        code_or_url: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<(OAuthTokenRecord, Option<String>), DomainError> {
        let code = Self::extract_code_from_input(code_or_url);
        if code.is_empty() {
            return Err(DomainError::Validation(
                "Authorization code cannot be empty".into(),
            ));
        }

        let effective_client_id = if client_id.is_empty() || client_id == "custos-openai-desktop" {
            resolve_openai_client_id()
        } else {
            client_id.to_string()
        };

        let effective_redirect_uri = if redirect_uri.is_empty() {
            DEFAULT_REDIRECT_URI
        } else {
            redirect_uri
        };

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", effective_client_id.as_str()),
            ("code", &code),
            ("redirect_uri", effective_redirect_uri),
            ("code_verifier", code_verifier),
        ];

        let mut form_body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        if service_google(&self.token_endpoint) {
            let secret = google_oauth_secret(service_type, &effective_client_id)?;
            form_body.push_str("&client_secret=");
            form_body.push_str(
                &url::form_urlencoded::byte_serialize(secret.as_bytes()).collect::<String>(),
            );
        }
        let (content_type, request_body) =
            if self.token_endpoint == "https://api.anthropic.com/v1/oauth/token" {
                let mut object: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(form_body.as_bytes())
                        .into_owned()
                        .collect();
                if let Some(code) = object.get("code").cloned() {
                    if let Some((code, state)) = code.split_once('#') {
                        object.insert("state".into(), state.into());
                        object.insert("code".into(), code.into());
                    }
                }
                (
                    "application/json",
                    serde_json::to_string(&object)
                        .map_err(|e| DomainError::Validation(e.to_string()))?,
                )
            } else {
                ("application/x-www-form-urlencoded", form_body)
            };
        let resp = self
            .http_client
            .post(&self.token_endpoint)
            .header("Content-Type", content_type)
            .body(request_body)
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("OAuth token request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(DomainError::Validation(format!(
                "OAuth token endpoint returned HTTP {status}: {body}"
            )));
        }

        let body: TokenEndpointResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!("Failed to parse OAuth token response: {e}"))
        })?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        let expires_in_secs = body.expires_in.unwrap_or(3600);
        let expires_at = now_ms + (expires_in_secs * 1000);

        let identity = oauth_identity(
            service_type,
            identity_token(service_type, &body.access_token, body.id_token.as_deref()),
        )
        .await?;
        Ok((
            OAuthTokenRecord {
                provider_id: provider_id.to_string(),
                service_type: service_type.to_string(),
                access_token: body.access_token,
                refresh_token: body.refresh_token,
                expires_at,
                token_type: body.token_type.unwrap_or_else(|| "Bearer".into()),
                scope: body
                    .scope
                    .or_else(|| oauth_settings(service_type).ok().map(|v| v.3)),
                created_at: now_ms,
                updated_at: now_ms,
            },
            identity,
        ))
    }

    /// Refreshes an expired access token using the stored refresh_token.
    pub async fn refresh_token(
        &self,
        client_id: &str,
        existing: &OAuthTokenRecord,
    ) -> Result<OAuthTokenRecord, DomainError> {
        let refresh_token = existing.refresh_token.as_ref().ok_or_else(|| {
            DomainError::Validation(format!(
                "Cannot refresh OAuth token for {}: no refresh_token present",
                existing.provider_id
            ))
        })?;

        let effective_client_id = if client_id.is_empty() || client_id == "custos-openai-desktop" {
            resolve_openai_client_id()
        } else {
            client_id.to_string()
        };

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", effective_client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
        ];

        let mut form_body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        if service_google(&self.token_endpoint) {
            let secret = google_oauth_secret(&existing.service_type, &effective_client_id)?;
            form_body.push_str("&client_secret=");
            form_body.push_str(
                &url::form_urlencoded::byte_serialize(secret.as_bytes()).collect::<String>(),
            );
        }
        let (content_type, request_body) =
            if self.token_endpoint == "https://api.anthropic.com/v1/oauth/token" {
                let mut object: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(form_body.as_bytes())
                        .into_owned()
                        .collect();
                if let Some(code) = object.get("code").cloned() {
                    if let Some((code, state)) = code.split_once('#') {
                        object.insert("state".into(), state.into());
                        object.insert("code".into(), code.into());
                    }
                }
                (
                    "application/json",
                    serde_json::to_string(&object)
                        .map_err(|e| DomainError::Validation(e.to_string()))?,
                )
            } else {
                ("application/x-www-form-urlencoded", form_body)
            };
        let resp = self
            .http_client
            .post(&self.token_endpoint)
            .header("Content-Type", content_type)
            .body(request_body)
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("OAuth refresh request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(DomainError::Validation(format!(
                "OAuth refresh endpoint returned HTTP {status}: {body}"
            )));
        }

        let body: TokenEndpointResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!("Failed to parse OAuth refresh response: {e}"))
        })?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        let expires_in_secs = body.expires_in.unwrap_or(3600);
        let expires_at = now_ms + (expires_in_secs * 1000);

        Ok(OAuthTokenRecord {
            provider_id: existing.provider_id.clone(),
            service_type: existing.service_type.clone(),
            access_token: body.access_token,
            refresh_token: body
                .refresh_token
                .or_else(|| existing.refresh_token.clone()),
            expires_at,
            token_type: body
                .token_type
                .unwrap_or_else(|| existing.token_type.clone()),
            scope: body.scope.or_else(|| existing.scope.clone()),
            created_at: existing.created_at,
            updated_at: now_ms,
        })
    }
}
