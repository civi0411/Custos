//! OAuth 2.0 PKCE Flow Manager
//!
//! Implements RFC 7636 Proof Key for Code Exchange for model provider authentication
//! (e.g., OpenAI, Anthropic, or external OAuth 2.0 providers).

pub mod copilot;
pub mod pkce_manager;
pub mod provider_defaults;
pub mod types;

pub use copilot::{complete_copilot_device, start_copilot_device};
pub use pkce_manager::OAuthPkceManager;
pub use provider_defaults::{
    google_oauth_client, google_oauth_client_with, oauth_identity, oauth_settings,
    public_google_oauth_defaults,
};
pub use types::{
    resolve_openai_client_id, resolve_openai_scope, OAuthAuthorizeUrlResult, PkceChallenge,
    DEFAULT_OPENAI_AUTH_URL, DEFAULT_OPENAI_CLIENT_ID, DEFAULT_OPENAI_SCOPE,
    DEFAULT_OPENAI_TOKEN_URL, DEFAULT_REDIRECT_URI,
};

#[cfg(test)]
mod tests;
