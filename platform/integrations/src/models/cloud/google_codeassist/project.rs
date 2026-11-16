use custos_domain::DomainError;

use super::transport::cloud_call;

pub async fn discover_project(
    service: &str,
    base: &str,
    token: &str,
) -> Result<String, DomainError> {
    // Antigravity's bootstrap endpoint expects the native minimal metadata object.
    let metadata = if service == "gemini" {
        serde_json::json!({"ideType":"GEMINI_CLI","platform":"WINDOWS_AMD64","pluginType":"GEMINI"})
    } else {
        serde_json::json!({"ideType":"ANTIGRAVITY"})
    };
    let mut value = cloud_call(
        service,
        base,
        token,
        "loadCodeAssist",
        serde_json::json!({"metadata":metadata}),
    )
    .await?;
    let project = |v: &serde_json::Value| {
        v.get("cloudaicompanionProject")
            .and_then(|v| v.as_str().or_else(|| v.get("id").and_then(|v| v.as_str())))
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    if let Some(id) = project(&value) {
        return Ok(id);
    }
    if service != "antigravity" {
        return Err(DomainError::Validation(
            "Gemini account did not return a managed project".into(),
        ));
    }
    let tier = value
        .pointer("/paidTier/id")
        .or_else(|| value.pointer("/currentTier/id"))
        .or_else(|| {
            value
                .get("allowedTiers")
                .and_then(|v| v.as_array())
                .and_then(|tiers| {
                    tiers
                        .iter()
                        .find(|v| v.get("isDefault").and_then(|v| v.as_bool()) == Some(true))
                })
                .and_then(|v| v.get("id"))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("legacy-tier")
        .to_string();
    tracing::debug!(
        provider_id = "antigravity",
        "Resolving Antigravity IDE managed project through native onboarding"
    );
    let onboard = cloud_call(
        service,
        base,
        token,
        "onboardUser",
        serde_json::json!({"tier_id":tier,"metadata":metadata}),
    )
    .await?;
    if let Some(id) = project(&onboard).or_else(|| onboard.get("response").and_then(project)) {
        return Ok(id);
    }
    // An accepted onboarding can precede asynchronous assignment; bound discovery retries.
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        value = cloud_call(
            service,
            base,
            token,
            "loadCodeAssist",
            serde_json::json!({"metadata":metadata}),
        )
        .await?;
        if let Some(id) = project(&value) {
            return Ok(id);
        }
    }
    Err(DomainError::Validation("Antigravity IDE backend did not assign a managed project after onboarding; retry the request later. No CLI installation is required".into()))
}
