use crate::connectors::oauth::provider_defaults::{
    google_oauth_client_with, identity_token, oauth_identity, oauth_identity_at, oauth_settings,
    public_google_oauth_defaults,
};
use crate::connectors::oauth::OAuthPkceManager;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use url::Url;

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
    assert_eq!(
        query.get("scope").unwrap(),
        "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers"
    );
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
