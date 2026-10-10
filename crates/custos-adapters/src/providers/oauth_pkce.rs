//! OAuth 2.0 PKCE Flow Manager
//!
//! Implements RFC 7636 Proof Key for Code Exchange for model provider authentication
//! (e.g., OpenAI, Anthropic, or external OAuth 2.0 providers).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use custos_domain::{DomainError, OAuthTokenRecord};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const DEFAULT_OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const DEFAULT_OPENAI_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const DEFAULT_OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const DEFAULT_OPENAI_SCOPE: &str = "openid profile email offline_access";
pub const DEFAULT_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";

/// Resolves standard Codex / OpenAI OAuth public client ID.
/// Checks CODEX_OAUTH_CLIENT_ID and OPENAI_OAUTH_CLIENT_ID environment variables,
/// falling back to the canonical public client ID `app_EMoamEEZ73f0CkXaXp7hrann`.
pub fn resolve_openai_client_id() -> String {
    std::env::var("CODEX_OAUTH_CLIENT_ID")
        .or_else(|_| std::env::var("OPENAI_OAUTH_CLIENT_ID"))
        .unwrap_or_else(|_| DEFAULT_OPENAI_CLIENT_ID.to_string())
}

/// Resolves standard OpenAI OAuth scope.
/// Allows OPENAI_OAUTH_SCOPE override, defaulting to standard `openid profile email offline_access`.
pub fn resolve_openai_scope() -> String {
    std::env::var("OPENAI_OAUTH_SCOPE")
        .unwrap_or_else(|_| DEFAULT_OPENAI_SCOPE.to_string())
}

/// Helper to extract `chatgpt_account_id` from a JWT access token's `https://api.openai.com/auth` claim.
pub fn extract_chatgpt_account_id(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let raw = parts[1];
    let padded = match raw.len() % 4 {
        2 => format!("{raw}=="),
        3 => format!("{raw}="),
        _ => raw.to_string(),
    };
    let decoded = URL_SAFE_NO_PAD
        .decode(padded.trim_end_matches('='))
        .ok()
        .or_else(|| base64::engine::general_purpose::STANDARD.decode(&padded).ok())?;
    let val: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    val.get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthorizeUrlResult {
    pub authorization_url: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenEndpointResponse {
    pub access_token: String,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

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
        let mut verifier_bytes = [0u8; 48];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        let mut state_bytes = [0u8; 16];
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
        code_or_url: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenRecord, DomainError> {
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

        let form_body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        let resp = self
            .http_client
            .post(&self.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body)
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

        Ok(OAuthTokenRecord {
            provider_id: provider_id.to_string(),
            service_type: service_type.to_string(),
            access_token: body.access_token,
            refresh_token: body.refresh_token,
            expires_at,
            token_type: body.token_type.unwrap_or_else(|| "Bearer".into()),
            scope: body.scope,
            created_at: now_ms,
            updated_at: now_ms,
        })
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

        let form_body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        let resp = self
            .http_client
            .post(&self.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body)
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
            refresh_token: body.refresh_token.or_else(|| existing.refresh_token.clone()),
            expires_at,
            token_type: body.token_type.unwrap_or_else(|| existing.token_type.clone()),
            scope: body.scope.or_else(|| existing.scope.clone()),
            created_at: existing.created_at,
            updated_at: now_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_challenge_generation() {
        let challenge = OAuthPkceManager::generate_pkce_challenge();
        assert!(challenge.code_verifier.len() >= 43);
        assert!(!challenge.code_challenge.is_empty());
        assert!(!challenge.state.is_empty());

        let mut hasher = Sha256::new();
        hasher.update(challenge.code_verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(challenge.code_challenge, expected);
    }

    #[test]
    fn test_build_authorization_url() {
        let manager = OAuthPkceManager::default();
        let challenge = OAuthPkceManager::generate_pkce_challenge();
        let res = manager
            .build_authorization_url(
                "custos-client-id",
                "http://localhost:1420/callback",
                None,
                &challenge,
            )
            .unwrap();

        assert!(res.authorization_url.contains("client_id=custos-client-id"));
        assert!(res.authorization_url.contains("code_challenge="));
        assert!(res.authorization_url.contains("code_challenge_method=S256"));
        assert!(res.authorization_url.contains("response_type=code"));
    }

    #[test]
    fn test_build_authorization_url_canonical_defaults() {
        let manager = OAuthPkceManager::default();
        let challenge = OAuthPkceManager::generate_pkce_challenge();
        let res = manager
            .build_authorization_url(
                "",
                "",
                None,
                &challenge,
            )
            .unwrap();

        assert!(res.authorization_url.contains("client_id=app_EMoamEEZ73f0CkXaXp7hrann"));
        assert!(res.authorization_url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
        assert_eq!(res.redirect_uri, "http://localhost:1455/auth/callback");
    }

    #[test]
    fn test_extract_code_from_input() {
        let raw = "code_123456789";
        assert_eq!(OAuthPkceManager::extract_code_from_input(raw), "code_123456789");

        let url = "http://localhost:1420/auth/callback?code=auth_code_abc&state=xyz";
        assert_eq!(
            OAuthPkceManager::extract_code_from_input(url),
            "auth_code_abc"
        );

        let url_no_scheme = "localhost:1420/callback?code=auth_code_xyz&other=1";
        assert_eq!(
            OAuthPkceManager::extract_code_from_input(url_no_scheme),
            "auth_code_xyz"
        );
    }

    #[test]
    fn test_extract_chatgpt_account_id() {
        // Construct a sample JWT payload with chatgpt_account_id
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";
        let payload_json = r#"{"https://api.openai.com/auth":{"chatgpt_account_id":"772c9eaf-a4ba-4606-b55a-a15c6b23e52a","chatgpt_plan_type":"plus"}}"#;
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json);
        let sig = "signature_part";
        let jwt = format!("{header}.{payload_b64}.{sig}");

        let account_id = extract_chatgpt_account_id(&jwt);
        assert_eq!(account_id.as_deref(), Some("772c9eaf-a4ba-4606-b55a-a15c6b23e52a"));

        // Invalid JWT returns None
        assert_eq!(extract_chatgpt_account_id("sk-proj-12345"), None);
        assert_eq!(extract_chatgpt_account_id("bad.token"), None);
    }
}
