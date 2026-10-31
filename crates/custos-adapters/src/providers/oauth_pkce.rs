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
    std::env::var("OPENAI_OAUTH_SCOPE").unwrap_or_else(|_| DEFAULT_OPENAI_SCOPE.to_string())
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
        .or_else(|| {
            base64::engine::general_purpose::STANDARD
                .decode(&padded)
                .ok()
        })?;
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
    pub id_token: Option<String>,
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
            .build_authorization_url("", "", None, &challenge)
            .unwrap();

        assert!(res
            .authorization_url
            .contains("client_id=app_EMoamEEZ73f0CkXaXp7hrann"));
        assert!(res
            .authorization_url
            .contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
        assert_eq!(res.redirect_uri, "http://localhost:1455/auth/callback");
    }

    #[test]
    fn test_extract_code_from_input() {
        let raw = "code_123456789";
        assert_eq!(
            OAuthPkceManager::extract_code_from_input(raw),
            "code_123456789"
        );

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
        assert_eq!(
            account_id.as_deref(),
            Some("772c9eaf-a4ba-4606-b55a-a15c6b23e52a")
        );

        // Invalid JWT returns None
        assert_eq!(extract_chatgpt_account_id("sk-proj-12345"), None);
        assert_eq!(extract_chatgpt_account_id("bad.token"), None);
    }
}

/// Provider-specific OAuth defaults; absent configuration never uses another issuer.
pub fn oauth_settings(service: &str) -> Result<(String, String, String, String), DomainError> {
    let service = service.split('-').next().unwrap_or(service);
    match service {
        "openai" | "codex" => Ok((DEFAULT_OPENAI_AUTH_URL.into(), DEFAULT_OPENAI_TOKEN_URL.into(), resolve_openai_client_id(), resolve_openai_scope())),
        "anthropic" => Ok(("https://claude.ai/oauth/authorize".into(), "https://api.anthropic.com/v1/oauth/token".into(), std::env::var("CLAUDE_OAUTH_CLIENT_ID").unwrap_or_else(|_| "9d1c250a-e61b-44d9-88ed-5944d1962f5e".into()), "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers".into())),
        "gemini" => Ok(("https://accounts.google.com/o/oauth2/v2/auth".into(), "https://oauth2.googleapis.com/token".into(), google_oauth_client("gemini")?.0, "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs".into())),
        "google" | "antigravity" => Ok(("https://accounts.google.com/o/oauth2/v2/auth".into(), "https://oauth2.googleapis.com/token".into(), google_oauth_client("antigravity")?.0, "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs".into())),
        _ => Err(DomainError::Validation(format!("OAuth for {service} requires a verified client and inference configuration; this provider is not configured"))),
    }
}

// ID tokens supply display claims for Codex; network identity APIs require access tokens.
fn identity_token<'a>(service: &str, access_token: &'a str, id_token: Option<&'a str>) -> &'a str {
    let service = service.split('-').next().unwrap_or(service);
    if matches!(service, "openai" | "codex") {
        id_token.unwrap_or(access_token)
    } else {
        access_token
    }
}

pub async fn oauth_identity(service: &str, token: &str) -> Result<Option<String>, DomainError> {
    let service = service.split('-').next().unwrap_or(service);
    let endpoint = if service == "anthropic" {
        "https://api.anthropic.com/api/claude_cli/bootstrap"
    } else {
        "https://www.googleapis.com/oauth2/v2/userinfo"
    };
    oauth_identity_at(service, token, endpoint).await
}

async fn oauth_identity_at(
    service: &str,
    token: &str,
    endpoint: &str,
) -> Result<Option<String>, DomainError> {
    let service = service.split('-').next().unwrap_or(service);
    if matches!(service, "google" | "antigravity" | "gemini" | "anthropic") {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let response = client
            .get(endpoint)
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", "claude-cli/2.1.280 (external, cli)")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                DomainError::Validation(format!("OAuth {service} identity lookup failed: {e}"))
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(DomainError::Validation(format!("OAuth {service} identity lookup returned HTTP {status}; token exchange succeeded, but account identity could not be fetched")));
        }
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        return Ok(value
            .get("email")
            .or_else(|| value.pointer("/oauth_account/account_email"))
            .and_then(|v| v.as_str())
            .map(str::to_string));
    }
    let raw = token
        .split('.')
        .nth(1)
        .and_then(|v| URL_SAFE_NO_PAD.decode(v).ok());
    Ok(raw
        .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
        .and_then(|v| v.get("email").and_then(|v| v.as_str()).map(str::to_string)))
}

fn service_google(endpoint: &str) -> bool {
    endpoint == "https://oauth2.googleapis.com/token"
}

#[cfg(test)]
mod provider_oauth_tests {
    use super::*;
    #[test]
    fn antigravity_authorization_has_google_issuer_and_pkce() {
        let settings = (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "test-google-client",
            "https://www.googleapis.com/auth/cloud-platform",
        );
        let result = OAuthPkceManager::new(settings.0, settings.1)
            .build_authorization_url(
                settings.2,
                "http://localhost:51121/oauth-callback",
                Some(settings.3),
                &OAuthPkceManager::generate_pkce_challenge(),
            )
            .unwrap();
        let url = Url::parse(&result.authorization_url).unwrap();
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(
            query.get("code_challenge_method").map(|v| v.as_ref()),
            Some("S256")
        );
        assert_eq!(
            query.get("access_type").map(|v| v.as_ref()),
            Some("offline")
        );
        assert!(oauth_settings("copilot").is_err());
    }
    #[test]
    fn google_public_defaults_and_environment_overrides_are_provider_scoped() {
        for service in ["gemini", "antigravity"] {
            let defaults = public_google_oauth_defaults(service).unwrap();
            assert_eq!(
                google_oauth_client_with(service, |_| None).unwrap(),
                defaults
            );
            assert_eq!(
                google_oauth_client_with(service, |_| Some("  ".into())).unwrap(),
                defaults
            );
            assert_eq!(
                google_oauth_client_with(service, |name| if name.ends_with("CLIENT_ID") {
                    Some(defaults.0.clone())
                } else {
                    None
                })
                .unwrap(),
                defaults
            );
            let error = google_oauth_client_with(service, |name| {
                if name.ends_with("CLIENT_ID") {
                    Some("test-client".into())
                } else {
                    Some("  ".into())
                }
            })
            .unwrap_err()
            .to_string();
            assert!(error.contains("CLIENT_SECRET"));
            let prefix = if service == "gemini" {
                "GEMINI_"
            } else {
                "ANTIGRAVITY_"
            };
            let pair = google_oauth_client_with(service, |name| {
                assert!(name.starts_with(prefix));
                Some(
                    if name.ends_with("CLIENT_ID") {
                        "test-client"
                    } else {
                        "test-secret"
                    }
                    .into(),
                )
            })
            .unwrap();
            assert_eq!(pair, ("test-client".into(), "test-secret".into()));
        }
    }

    #[test]
    fn public_native_client_defaults_match_reference_fingerprints() {
        let pair = public_google_oauth_defaults("gemini").unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(pair.0.as_bytes())),
            "636d7183f3ca4ad92c7803811a3f635643f1da657a22eeedf280283161cb6e17"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(pair.1.as_bytes())),
            "6a5f78b8b99dd4025e41ba11bf54c304c6af29f924c5569cc7865b2428ce03a9"
        );
        let pair = public_google_oauth_defaults("antigravity").unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(pair.0.as_bytes())),
            "bf00c418024ba6bf606ccdc37120976e41bc429dd1d46ecf16a729aa532626ea"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(pair.1.as_bytes())),
            "1d2f041093fd95aa8995a038c711d50a7960da09a505381c09a745d6ad0ecc60"
        );
    }

    #[test]
    fn claude_authorization_matches_native_pkce_request() {
        let settings = oauth_settings("anthropic").unwrap();
        let challenge = OAuthPkceManager::generate_pkce_challenge();
        assert_eq!(challenge.state.len(), 43);
        assert_eq!(challenge.code_verifier.len(), 43);
        let result = OAuthPkceManager::new(settings.0, settings.1)
            .build_authorization_url(
                &settings.2,
                "https://platform.claude.com/oauth/code/callback",
                Some(&settings.3),
                &challenge,
            )
            .unwrap();
        let url = Url::parse(&result.authorization_url).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(url.host_str(), Some("claude.ai"));
        assert_eq!(
            query.get("client_id").unwrap(),
            "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
        );
        assert_eq!(query.get("code").unwrap(), "true");
        assert_eq!(query.get("scope").unwrap(), "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers");
        assert_eq!(
            query.get("redirect_uri").unwrap(),
            "https://platform.claude.com/oauth/code/callback"
        );
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(query.get("state").unwrap().as_ref(), challenge.state);
    }

    #[tokio::test]
    async fn google_identity_uses_access_token_when_id_token_is_present() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .and(header("authorization", "Bearer access-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"email":"person@example.com"})),
            )
            .expect(2)
            .mount(&server)
            .await;
        for service in ["antigravity", "gemini"] {
            let token = identity_token(service, "access-token", Some("id-token"));
            assert_eq!(
                oauth_identity_at(service, token, &format!("{}/userinfo", server.uri()))
                    .await
                    .unwrap()
                    .as_deref(),
                Some("person@example.com")
            );
        }
        assert_eq!(
            identity_token("openai", "access-token", Some("id-token")),
            "id-token"
        );
        assert_eq!(
            identity_token("anthropic", "access-token", Some("id-token")),
            "access-token"
        );
        server.reset().await;
        Mock::given(path("/userinfo"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let error = oauth_identity_at(
            "antigravity",
            "bad-token",
            &format!("{}/userinfo", server.uri()),
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("identity lookup returned HTTP 401"));
        assert!(error.contains("token exchange succeeded"));
    }

    #[tokio::test]
    async fn id_token_identity_is_used_for_openai() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"email":"person@example.com"}"#);
        assert_eq!(
            oauth_identity("openai", &format!("header.{payload}.sig"))
                .await
                .unwrap()
                .as_deref(),
            Some("person@example.com")
        );
    }
}

fn google_oauth_client(service: &str) -> Result<(String, String), DomainError> {
    google_oauth_client_with(service, |name| std::env::var(name).ok())
}

fn google_oauth_client_with(
    service: &str,
    read: impl Fn(&str) -> Option<String>,
) -> Result<(String, String), DomainError> {
    let (id_name, secret_name) = match service {
        "gemini" => ("GEMINI_OAUTH_CLIENT_ID", "GEMINI_OAUTH_CLIENT_SECRET"),
        "antigravity" => (
            "ANTIGRAVITY_OAUTH_CLIENT_ID",
            "ANTIGRAVITY_OAUTH_CLIENT_SECRET",
        ),
        _ => {
            return Err(DomainError::Validation(
                "Unsupported Google OAuth provider".into(),
            ))
        }
    };
    let (default_id, default_secret) = public_google_oauth_defaults(service)?;
    let override_value = |name: &str| {
        read(name)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let client_id = override_value(id_name).unwrap_or_else(|| default_id.clone());
    let secret = match override_value(secret_name) {
        Some(secret) => secret,
        None if client_id == default_id => default_secret,
        None => {
            return Err(DomainError::Validation(format!(
                "Custom Google OAuth client requires {secret_name} in the daemon environment"
            )))
        }
    };
    Ok((client_id, secret))
}

// Public installed-app credentials from OmniRoute; XOR changes representation only.
// Never use this mechanism to store private operator credentials or account tokens.
fn public_google_oauth_defaults(service: &str) -> Result<(String, String), DomainError> {
    const MASK: &[u8] = b"omniroute-public-v1";
    const GEMINI_ID: &[u8] = &[
        89, 85, 95, 91, 71, 90, 77, 68, 92, 30, 73, 64, 79, 3, 6, 91, 75, 2, 3, 0, 29, 28, 13, 0,
        1, 5, 77, 0, 30, 17, 4, 4, 90, 8, 21, 30, 30, 92, 11, 4, 12, 88, 65, 90, 31, 90, 4, 93, 0,
        6, 76, 11, 6, 12, 74, 26, 84, 26, 30, 11, 27, 17, 0, 27, 0, 0, 67, 4, 91, 1, 3, 4,
    ];
    const GEMINI_ALT: &[u8] = &[
        40, 34, 45, 58, 34, 55, 88, 64, 16, 101, 23, 56, 50, 1, 68, 82, 66, 65, 98, 4, 64, 9, 12,
        36, 89, 54, 1, 80, 78, 28, 45, 36, 31, 17, 15,
    ];
    const ANTIGRAVITY_ID: &[u8] = &[
        94, 93, 89, 88, 66, 95, 67, 68, 83, 29, 69, 76, 83, 65, 29, 14, 69, 5, 66, 6, 3, 92, 1, 64,
        94, 25, 23, 23, 72, 66, 70, 87, 26, 29, 12, 65, 25, 91, 7, 89, 9, 93, 66, 92, 16, 4, 75,
        76, 0, 5, 17, 66, 14, 12, 66, 17, 93, 10, 24, 29, 12, 0, 12, 26, 26, 17, 72, 30, 1, 76, 15,
        6, 14,
    ];
    const ANTIGRAVITY_ALT: &[u8] = &[
        40, 34, 45, 58, 34, 55, 88, 63, 80, 21, 54, 34, 48, 88, 81, 85, 97, 18, 125, 37, 92, 3, 37,
        48, 87, 6, 44, 38, 25, 10, 67, 19, 40, 40, 5,
    ];
    let (id, secret) = match service {
        "gemini" => (GEMINI_ID, GEMINI_ALT),
        "google" | "antigravity" => (ANTIGRAVITY_ID, ANTIGRAVITY_ALT),
        _ => {
            return Err(DomainError::Validation(
                "Unsupported public Google OAuth client".into(),
            ))
        }
    };
    let decode = |bytes: &[u8]| {
        String::from_utf8(
            bytes
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ MASK[index % MASK.len()])
                .collect(),
        )
        .map_err(|_| DomainError::Validation("Invalid public OAuth default representation".into()))
    };
    Ok((decode(id)?, decode(secret)?))
}

fn google_oauth_secret(service: &str, client: &str) -> Result<String, DomainError> {
    let (configured_client, secret) = google_oauth_client(service)?;
    if configured_client != client {
        return Err(DomainError::Validation("Google OAuth client does not match daemon configuration; use the client that issued this account's tokens".into()));
    }
    Ok(secret)
}

/// Start GitHub's real device grant. The first-party Copilot app supports this flow.
pub async fn start_copilot_device() -> Result<serde_json::Value, DomainError> {
    tracing::debug!(
        provider_id = "copilot",
        "Starting GitHub device authorization"
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    let form = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("client_id", "Iv1.b507a08c87ecfe98"),
            ("scope", "read:user user:email"),
        ])
        .finish();
    client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form)
        .send()
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?
        .error_for_status()
        .map_err(|e| DomainError::Validation(e.to_string()))?
        .json()
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))
}

pub async fn complete_copilot_device(
    device: &serde_json::Value,
) -> Result<(OAuthTokenRecord, Option<String>), DomainError> {
    let code = device
        .get("device_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            DomainError::Validation("GitHub device response lacks device_code".into())
        })?;
    let expiry = device
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(900)
        .min(1800);
    let mut interval = device
        .get("interval")
        .and_then(|v| v.as_u64())
        .unwrap_or(5)
        .max(5);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(expiry);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        let form = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("client_id", "Iv1.b507a08c87ecfe98"),
                ("device_code", code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .finish();
        let value: serde_json::Value = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .error_for_status()
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if let Some(token) = value.get("access_token").and_then(|v| v.as_str()) {
            let user: serde_json::Value = client
                .get("https://api.github.com/user")
                .header("User-Agent", "Custos")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .error_for_status()
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .json()
                .await
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            let mut identity = user
                .get("email")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if identity.is_none() {
                let emails: serde_json::Value = client
                    .get("https://api.github.com/user/emails")
                    .header("User-Agent", "Custos")
                    .bearer_auth(token)
                    .send()
                    .await
                    .map_err(|e| DomainError::Validation(e.to_string()))?
                    .error_for_status()
                    .map_err(|e| DomainError::Validation(e.to_string()))?
                    .json()
                    .await
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                identity = emails
                    .as_array()
                    .and_then(|items| {
                        items
                            .iter()
                            .find(|v| v["primary"] == true && v["verified"] == true)
                    })
                    .and_then(|v| v["email"].as_str())
                    .map(str::to_string);
            }
            if identity.is_none() {
                identity = user
                    .get("login")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            let now = chrono::Utc::now().timestamp_millis();
            return Ok((
                OAuthTokenRecord {
                    provider_id: "copilot".into(),
                    service_type: "copilot".into(),
                    access_token: token.into(),
                    refresh_token: None,
                    expires_at: i64::MAX / 2,
                    token_type: "Bearer".into(),
                    scope: Some("read:user user:email".into()),
                    created_at: now,
                    updated_at: now,
                },
                identity,
            ));
        }
        match value.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => (),
            Some("slow_down") => interval = interval.saturating_add(5),
            _ => {
                return Err(DomainError::Validation(format!(
                    "GitHub device authorization failed: {}",
                    value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("invalid response")
                )))
            }
        }
    }
    Err(DomainError::Validation(
        "GitHub device authorization expired; start again".into(),
    ))
}

pub async fn copilot_session_token(token: &str) -> Result<String, DomainError> {
    tracing::debug!(
        provider_id = "copilot",
        "Acquiring Copilot subscription token"
    );
    copilot_session_token_at(token, "https://api.github.com/copilot_internal/v2/token").await
}

async fn copilot_session_token_at(token: &str, endpoint: &str) -> Result<String, DomainError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    let value: serde_json::Value = client
        .get(endpoint)
        .bearer_auth(token)
        .header("User-Agent", "GitHubCopilotChat/0.35.0")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?
        .error_for_status()
        .map_err(|e| {
            DomainError::Validation(format!("Copilot subscription token unavailable: {e}"))
        })?
        .json()
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    value
        .get("token")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| DomainError::Validation("Copilot session response lacks token".into()))
}

#[cfg(test)]
mod copilot_token_tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    #[tokio::test]
    async fn copilot_session_exchange_requires_subscription_and_bearer_auth() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/token"))
            .and(header("authorization", "Bearer github-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"token":"session-token"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            copilot_session_token_at("github-token", &format!("{}/token", server.uri()))
                .await
                .unwrap(),
            "session-token"
        );
        server.reset().await;
        Mock::given(path("/token"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        assert!(
            copilot_session_token_at("github-token", &format!("{}/token", server.uri()))
                .await
                .unwrap_err()
                .to_string()
                .contains("subscription")
        );
    }
}
