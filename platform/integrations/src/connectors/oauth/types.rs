//! Constants and data types for OAuth 2.0 PKCE.

use serde::{Deserialize, Serialize};

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
pub(crate) struct TokenEndpointResponse {
    pub access_token: String,
    pub id_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}
