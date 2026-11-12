//! Inference token extraction and session-token exchange helpers.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use custos_domain::DomainError;

/// Extracts `chatgpt_account_id` from a JWT access token's `https://api.openai.com/auth` claim.
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
        .map(str::to_string)
}

/// Exchanges a GitHub OAuth token for a transient Copilot session token.
pub async fn copilot_session_token(token: &str) -> Result<String, DomainError> {
    tracing::debug!(
        provider_id = "copilot",
        "Acquiring Copilot subscription token"
    );
    copilot_session_token_at(token, "https://api.github.com/copilot_internal/v2/token").await
}

pub(crate) async fn copilot_session_token_at(
    token: &str,
    endpoint: &str,
) -> Result<String, DomainError> {
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
mod tests {
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

    #[test]
    fn test_extract_chatgpt_account_id() {
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

        assert_eq!(extract_chatgpt_account_id("sk-proj-12345"), None);
        assert_eq!(extract_chatgpt_account_id("bad.token"), None);
    }
}

