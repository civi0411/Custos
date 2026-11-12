use crate::connectors::oauth::OAuthPkceManager;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

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
