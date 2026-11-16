//! GitHub Copilot device authorization grant and session token acquisition.

use custos_domain::{DomainError, OAuthTokenRecord};

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
