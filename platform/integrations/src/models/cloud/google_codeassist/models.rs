use custos_domain::DomainError;

use super::project::discover_project;
use super::transport::cloud_call;

/// Verified Antigravity transport capability, applied only to discovered IDs.
pub fn supports_tiered_reasoning(id: &str) -> bool {
    id.strip_prefix("antigravity/").unwrap_or(id) == "gemini-3.8-flash-tiered"
}

pub async fn probe_models(
    base: &str,
    token: &str,
) -> Result<Vec<custos_domain::ProbedModel>, DomainError> {
    probe_codeassist_models("antigravity", base, token).await
}

pub async fn probe_codeassist_models(
    service: &str,
    base: &str,
    token: &str,
) -> Result<Vec<custos_domain::ProbedModel>, DomainError> {
    let body = if service == "antigravity" {
        serde_json::json!({})
    } else {
        serde_json::json!({"project":discover_project(service, base, token).await?})
    };
    let value = cloud_call(service, base, token, "fetchAvailableModels", body).await?;
    let payload = value
        .get("models")
        .ok_or_else(|| DomainError::Validation("Invalid Antigravity model catalog".into()))?;
    let models: Vec<(String, &serde_json::Value)> = if let Some(map) = payload.as_object() {
        map.iter().map(|(id, model)| (id.clone(), model)).collect()
    } else if let Some(array) = payload.as_array() {
        array
            .iter()
            .filter_map(|model| {
                model
                    .get("id")
                    .or_else(|| model.get("name"))
                    .or_else(|| model.get("model"))
                    .and_then(|v| v.as_str())
                    .filter(|id| !id.is_empty())
                    .map(|id| (id.to_string(), model))
            })
            .collect()
    } else {
        return Err(DomainError::Validation(
            "Invalid Antigravity model catalog".into(),
        ));
    };
    Ok(models
        .into_iter()
        .filter(|(_, model)| crate::routing::probe::model_is_selectable(model))
        .map(|(id, model)| custos_domain::ProbedModel {
            id: format!(
                "{}/{id}",
                if service == "gemini" {
                    "gemini-oauth"
                } else {
                    "antigravity"
                }
            ),
            name: model
                .get("displayName")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            context_window: model.get("maxInputTokens").and_then(|v| v.as_u64()),
            owned_by: Some(service.into()),
            description: None,
            efforts: if service == "antigravity" && supports_tiered_reasoning(&id) {
                vec!["low".into(), "medium".into(), "high".into()]
            } else {
                vec![]
            },
            default_effort: if service == "antigravity" && supports_tiered_reasoning(&id) {
                Some("medium".into())
            } else {
                None
            },
        })
        .collect())
}
