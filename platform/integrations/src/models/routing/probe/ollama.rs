use custos_domain::{DomainError, ProbedModel};
use serde_json::Value;

/// Normalize an Ollama root without duplicating compatibility API suffixes.
pub fn ollama_root(base: &str) -> Result<String, DomainError> {
    let root = base.trim().trim_end_matches('/').trim_end_matches("/v1");
    let url = reqwest::Url::parse(root)
        .map_err(|e| DomainError::Validation(format!("Invalid Ollama URL: {e}")))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DomainError::Validation(
            "Ollama URL must be an HTTP(S) base URL without query or fragment".into(),
        ));
    }
    Ok(root.to_string())
}

/// Ollama native API:
/// GET /api/tags -> lists tags/models
/// POST /api/show with {"model": name} -> inspects model_info["<arch>.context_length"]
pub(crate) async fn probe_ollama_endpoint(
    client: &reqwest::Client,
    root: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = format!("{root}/api/tags");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Ollama probe connect error: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Ollama /api/tags returned status: {}",
            resp.status()
        )));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to read Ollama body: {e}")))?;

    let names = parse_ollama_tags(&text).ok_or_else(|| {
        DomainError::Validation("Invalid Ollama /api/tags response schema".into())
    })?;

    let mut probed = Vec::new();
    for name in names {
        // Query show endpoint for context length (best-effort)
        let show_url = format!("{root}/api/show");
        let context = match client
            .post(&show_url)
            .json(&serde_json::json!({ "model": &name }))
            .send()
            .await
        {
            Ok(show_resp) if show_resp.status().is_success() => {
                if let Ok(show_text) = show_resp.text().await {
                    parse_ollama_show_context(&show_text)
                } else {
                    None
                }
            }
            _ => None,
        };

        probed.push(ProbedModel {
            id: format!("ollama/{name}"),
            name: Some(name),
            context_window: context,
            owned_by: Some("ollama".into()),
            description: None,
            ..Default::default()
        });
    }

    Ok(probed)
}

pub fn parse_ollama_tags(body: &str) -> Option<Vec<String>> {
    let v: Value = serde_json::from_str(body).ok()?;
    let models = v.get("models")?.as_array()?;
    let names: Vec<String> = models
        .iter()
        .filter_map(|m| m.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    Some(names)
}

pub fn parse_ollama_show_context(body: &str) -> Option<u64> {
    let v: Value = serde_json::from_str(body).ok()?;
    let info = v.get("model_info")?.as_object()?;
    info.iter()
        .find(|(k, _)| k.ends_with(".context_length"))
        .and_then(|(_, v)| v.as_u64())
}
