use super::server::{OAuthCallbackServer, PendingOAuthContext};
use super::utils::extract_query_param;
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

#[test]
fn manual_callback_requires_matching_state_and_is_single_use() {
    let server = OAuthCallbackServer::new(Arc::new(SqliteTaskStore::new_in_memory().unwrap()));
    *server.pending.lock().unwrap() = Some(PendingOAuthContext {
        provider_id: "antigravity".into(),
        service_type: "antigravity".into(),
        client_id: "google-client".into(),
        code_verifier: "verifier".into(),
        state: "expected-state".into(),
        redirect_uri: "http://localhost:51121/oauth-callback".into(),
        expires_at: i64::MAX,
    });
    assert!(server
        .validate_manual_callback(
            "antigravity",
            "verifier",
            "http://localhost:51121/oauth-callback",
            "http://localhost:51121/oauth-callback?code=code&state=wrong"
        )
        .is_err());
    assert!(server
        .validate_manual_callback(
            "antigravity",
            "wrong",
            "http://localhost:51121/oauth-callback",
            "http://localhost:51121/oauth-callback?code=code&state=expected-state"
        )
        .is_err());
    assert!(server
        .validate_manual_callback(
            "antigravity",
            "verifier",
            "http://localhost:51121/oauth-callback",
            "http://localhost:51121/oauth-callback?code=code&state=expected-state"
        )
        .is_ok());
    assert!(server
        .validate_manual_callback(
            "antigravity",
            "verifier",
            "http://localhost:51121/oauth-callback",
            "http://localhost:51121/oauth-callback?code=code&state=expected-state"
        )
        .is_err());
}

#[test]
fn manual_callback_rejects_expired_and_cancelled_authorizations() {
    let server = OAuthCallbackServer::new(Arc::new(SqliteTaskStore::new_in_memory().unwrap()));
    *server.pending.lock().unwrap() = Some(PendingOAuthContext {
        provider_id: "anthropic".into(),
        service_type: "anthropic".into(),
        client_id: "client".into(),
        code_verifier: "verifier".into(),
        state: "state".into(),
        redirect_uri: "https://platform.claude.com/oauth/code/callback".into(),
        expires_at: 0,
    });
    assert!(server
        .validate_manual_callback(
            "anthropic",
            "verifier",
            "https://platform.claude.com/oauth/code/callback",
            "code#state"
        )
        .is_err());
    server.pending.lock().unwrap().as_mut().unwrap().expires_at = i64::MAX;
    server.cancel();
    assert!(server.pending.lock().unwrap().is_none());
    assert!(server
        .validate_manual_callback(
            "anthropic",
            "verifier",
            "https://platform.claude.com/oauth/code/callback",
            "code#state"
        )
        .is_err());
}

#[test]
fn test_extract_query_param_basic() {
    let uri = "/auth/callback?code=abc123xyz&state=state_secret";
    assert_eq!(
        extract_query_param(uri, "code"),
        Some("abc123xyz".to_string())
    );
    assert_eq!(
        extract_query_param(uri, "state"),
        Some("state_secret".to_string())
    );
    assert_eq!(extract_query_param(uri, "missing"), None);
}

#[test]
fn test_extract_query_param_url_encoded() {
    let uri = "/oauth_callback?code=test%2F123%2Bcode&state=val+space";
    assert_eq!(
        extract_query_param(uri, "code"),
        Some("test/123+code".to_string())
    );
    assert_eq!(
        extract_query_param(uri, "state"),
        Some("val space".to_string())
    );
}

#[test]
fn test_render_success_html() {
    let html = super::html::render_success_html();
    assert!(html.contains("Authorization Received"));
    assert!(html.contains("Custos received the authorization callback"));
}

