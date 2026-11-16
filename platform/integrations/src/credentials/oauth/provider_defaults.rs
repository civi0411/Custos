//! Provider-specific OAuth configurations, defaults, and identity resolution.

use super::types::{
    resolve_openai_client_id, resolve_openai_scope, DEFAULT_OPENAI_AUTH_URL,
    DEFAULT_OPENAI_TOKEN_URL,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use custos_domain::DomainError;

/// Provider-specific OAuth defaults; absent configuration never uses another issuer.
pub fn oauth_settings(service: &str) -> Result<(String, String, String, String), DomainError> {
    let service = service.split('-').next().unwrap_or(service);
    match service {
        "openai" | "codex" => Ok((
            DEFAULT_OPENAI_AUTH_URL.into(),
            DEFAULT_OPENAI_TOKEN_URL.into(),
            resolve_openai_client_id(),
            resolve_openai_scope(),
        )),
        "anthropic" => Ok((
            "https://claude.ai/oauth/authorize".into(),
            "https://api.anthropic.com/v1/oauth/token".into(),
            std::env::var("CLAUDE_OAUTH_CLIENT_ID")
                .unwrap_or_else(|_| "9d1c250a-e61b-44d9-88ed-5944d1962f5e".into()),
            "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers".into(),
        )),
        "gemini" => Ok((
            "https://accounts.google.com/o/oauth2/v2/auth".into(),
            "https://oauth2.googleapis.com/token".into(),
            google_oauth_client("gemini")?.0,
            "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs".into(),
        )),
        "google" | "antigravity" => Ok((
            "https://accounts.google.com/o/oauth2/v2/auth".into(),
            "https://oauth2.googleapis.com/token".into(),
            google_oauth_client("antigravity")?.0,
            "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs".into(),
        )),
        _ => Err(DomainError::Validation(format!(
            "OAuth for {service} requires a verified client and inference configuration; this provider is not configured"
        ))),
    }
}

// ID tokens supply display claims for Codex; network identity APIs require access tokens.
pub(crate) fn identity_token<'a>(
    service: &str,
    access_token: &'a str,
    id_token: Option<&'a str>,
) -> &'a str {
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

pub(crate) async fn oauth_identity_at(
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

pub(crate) fn service_google(endpoint: &str) -> bool {
    endpoint == "https://oauth2.googleapis.com/token"
}

pub fn google_oauth_client(service: &str) -> Result<(String, String), DomainError> {
    google_oauth_client_with(service, |name| std::env::var(name).ok())
}

pub fn google_oauth_client_with(
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
pub fn public_google_oauth_defaults(service: &str) -> Result<(String, String), DomainError> {
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

pub(crate) fn google_oauth_secret(service: &str, client: &str) -> Result<String, DomainError> {
    let (configured_client, secret) = google_oauth_client(service)?;
    if configured_client != client {
        return Err(DomainError::Validation("Google OAuth client does not match daemon configuration; use the client that issued this account's tokens".into()));
    }
    Ok(secret)
}
