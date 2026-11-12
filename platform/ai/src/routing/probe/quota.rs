use custos_domain::DomainError;
use serde_json::Value;
use std::time::Duration;

/// Fetch account quota independently of observed inference usage.
pub async fn probe_account_quota(service: &str, token: &str) -> Result<Value, DomainError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    if service == "antigravity" {
        if let Ok(response) = client
            .post("https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary")
            .bearer_auth(token)
            .header("User-Agent", "antigravity")
            .json(&serde_json::json!({}))
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(body) = response.json::<Value>().await {
                    let parsed = parse_account_quota("antigravity-summary", &body);
                    if parsed["status"] == "available" {
                        return Ok(parsed);
                    }
                }
            }
        }
        tracing::debug!("Antigravity grouped quota unavailable; trying existing quota fallback");
    }
    // Quota reads must not onboard or create a project.
    if matches!(service, "antigravity" | "gemini") {
        let metadata = if service == "antigravity" {
            serde_json::json!({"ideType":"ANTIGRAVITY"})
        } else {
            serde_json::json!({"ideType":"GEMINI_CLI","platform":"WINDOWS_AMD64","pluginType":"GEMINI"})
        };
        if let Ok(response) = client
            .post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .bearer_auth(token)
            .json(&serde_json::json!({"metadata":metadata}))
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(value) = response.json::<Value>().await {
                    if let Some(project) = value
                        .get("cloudaicompanionProject")
                        .and_then(|v| v.as_str().or_else(|| v.get("id").and_then(Value::as_str)))
                    {
                        if let Ok(response) = client
                            .post(
                                "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota",
                            )
                            .bearer_auth(token)
                            .json(&serde_json::json!({"project":project}))
                            .send()
                            .await
                        {
                            if response.status().is_success() {
                                if let Ok(value) = response.json::<Value>().await {
                                    let parsed = parse_account_quota("google-live", &value);
                                    if parsed["status"] == "available" {
                                        return Ok(parsed);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let request = match service {
        "openai" => {
            let mut request = client
                .get("https://chatgpt.com/backend-api/wham/usage")
                .bearer_auth(token);
            if let Some(id) = crate::cloud::transport::extract_chatgpt_account_id(token) {
                request = request.header("ChatGPT-Account-ID", id);
            }
            request
        }
        "anthropic" => client
            .get("https://api.anthropic.com/api/oauth/usage")
            .bearer_auth(token)
            .header("anthropic-beta", "oauth-2025-04-20"),
        "copilot" => client
            .get("https://api.github.com/copilot_internal/user")
            .bearer_auth(token)
            .header("User-Agent", "Custos")
            .header("Accept", "application/json"),
        "antigravity" | "gemini" => client
            .post("https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels")
            .bearer_auth(token)
            .header("User-Agent", "antigravity/1.15.8 windows/amd64")
            .json(&serde_json::json!({})),
        _ => return Ok(serde_json::json!({"status":"unsupported", "windows":[]})),
    };
    let response = request
        .send()
        .await
        .map_err(|_| DomainError::Validation("Quota service unavailable".into()))?;
    if !response.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Quota service returned {}",
            response.status()
        )));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| DomainError::Validation("Invalid quota response".into()))?;
    Ok(parse_account_quota(service, &body))
}

pub fn parse_account_quota(service: &str, body: &Value) -> Value {
    let mut windows = Vec::new();
    let mut push = |name: &str, used: Option<f64>, reset: Value| {
        if let Some(used) = used.filter(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0) {
            windows.push(serde_json::json!({"name":name,"used_percent":used,"resets_at":reset}));
        }
    };
    match service {
        "antigravity-summary" => {
            if let Some(groups) = body.get("groups").and_then(Value::as_array) {
                for group in groups {
                    if let Some(buckets) = group.get("buckets").and_then(Value::as_array) {
                        for bucket in buckets {
                            let name = bucket
                                .get("bucketId")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or_else(|| {
                                    format!(
                                        "{} {}",
                                        group
                                            .get("displayName")
                                            .and_then(Value::as_str)
                                            .unwrap_or("Unknown group"),
                                        bucket
                                            .get("displayName")
                                            .and_then(Value::as_str)
                                            .unwrap_or("Unknown limit")
                                    )
                                });
                            push(
                                &name,
                                bucket
                                    .get("remainingFraction")
                                    .and_then(Value::as_f64)
                                    .map(|v| (1.0 - v) * 100.0),
                                bucket.get("resetTime").cloned().unwrap_or(Value::Null),
                            );
                        }
                    }
                }
            }
        }
        "openai" => {
            for (name, key) in [
                ("Session", "primary_window"),
                ("Weekly", "secondary_window"),
            ] {
                if let Some(window) = body.pointer(&format!("/rate_limit/{key}")) {
                    push(
                        name,
                        window.get("used_percent").and_then(Value::as_f64),
                        window.get("reset_at").cloned().unwrap_or(Value::Null),
                    );
                }
            }
        }
        "anthropic" => {
            for key in [
                "five_hour",
                "seven_day",
                "seven_day_sonnet",
                "seven_day_opus",
            ] {
                if let Some(window) = body.get(key) {
                    push(
                        key,
                        window.get("utilization").and_then(Value::as_f64),
                        window.get("resets_at").cloned().unwrap_or(Value::Null),
                    );
                }
            }
        }
        "copilot" => {
            if let Some(snapshots) = body.get("quota_snapshots").and_then(Value::as_object) {
                for (key, window) in snapshots {
                    if window.get("unlimited").and_then(Value::as_bool) != Some(true) {
                        push(
                            key,
                            window
                                .get("percent_remaining")
                                .and_then(Value::as_f64)
                                .map(|v| 100.0 - v),
                            body.get("quota_reset_date").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
            }
        }
        "google-live" => {
            if let Some(buckets) = body.get("buckets").and_then(Value::as_array) {
                for bucket in buckets {
                    if let Some(name) = bucket.get("modelId").and_then(Value::as_str) {
                        push(
                            name,
                            bucket
                                .get("remainingFraction")
                                .and_then(Value::as_f64)
                                .map(|v| (1.0 - v) * 100.0),
                            bucket.get("resetTime").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
            }
        }
        "antigravity" | "gemini" => {
            if let Some(models) = body.get("models").and_then(Value::as_object) {
                for (key, model) in models {
                    if let Some(quota) = model.get("quotaInfo") {
                        push(
                            key,
                            quota
                                .get("remainingFraction")
                                .and_then(Value::as_f64)
                                .map(|v| (1.0 - v) * 100.0),
                            quota.get("resetTime").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
            }
        }
        _ => {}
    }
    for window in &mut windows {
        let name = window["name"].as_str().unwrap_or_default().to_lowercase();
        let period =
            if name.contains("weekly") || name.starts_with("seven_day") || name.contains("7-day") {
                "weekly"
            } else if name.contains("5h")
                || name.contains("five_hour")
                || name.contains("five hour")
                || name == "session"
            {
                "five_hour"
            } else {
                "other"
            };
        window["window_kind"] = serde_json::json!(period);
        if name.contains("gemini") {
            window["model_group"] = serde_json::json!("gemini");
        } else if name.contains("3p") || name.contains("claude") || name.contains("gpt") {
            window["model_group"] = serde_json::json!("claude_gpt");
        }
        if service == "openai" {
            let key = if name == "session" {
                "primary_window"
            } else {
                "secondary_window"
            };
            if let Some(seconds) = body
                .pointer(&format!("/rate_limit/{key}/limit_window_seconds"))
                .and_then(Value::as_u64)
            {
                window["window_kind"] = serde_json::json!(match seconds {
                    18000 => "five_hour",
                    604800 => "weekly",
                    _ => "other",
                });
            }
        }
    }
    serde_json::json!({
        "status": if windows.is_empty() { "unavailable" } else { "available" },
        "source": if matches!(service, "antigravity" | "gemini") { "provider_catalog" } else { "provider" },
        "windows": windows
    })
}
