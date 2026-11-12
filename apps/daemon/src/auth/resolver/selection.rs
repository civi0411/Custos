//! Provider account reservation and catalog scoping logic.

use custos_domain::DomainError;

#[cfg(test)]
pub(crate) fn reserve_provider_account(
    repo: &custos_persistence::ProviderRepository,
    service: &str,
    model: &str,
) -> Result<Option<String>, DomainError> {
    reserve_available_provider_account(repo, service, model, None, &Default::default())
}

pub(crate) fn reserve_available_provider_account(
    repo: &custos_persistence::ProviderRepository,
    service: &str,
    model: &str,
    effort: Option<&str>,
    availability: &custos_provider::routing::availability::ModelAvailability,
) -> Result<Option<String>, DomainError> {
    let providers = repo.list_providers()?;
    let mut eligible = Vec::new();
    let mut stored = false;
    for account in providers.iter().filter(|p| {
        p.service_type == service
            || (matches!(service, "antigravity" | "gemini")
                && matches!(p.service_type.as_str(), "antigravity" | "gemini"))
    }) {
        stored = true;
        if !matches!(
            account.status.as_str(),
            "active" | "configured" | "primary" | "standby"
        ) {
            continue;
        }
        let metadata = repo.credential_metadata(&account.id)?;
        let effective_service = account.service_type.as_str();
        if effective_service != "local"
            && !metadata.as_ref().is_some_and(|m| {
                m.state == "active"
                    && (m.has_refresh_token || m.expires_at > chrono::Utc::now().timestamp_millis())
            })
        {
            continue;
        }
        let oauth = metadata.as_ref().is_some_and(|m| {
            m.has_refresh_token || m.scope.as_deref().is_some_and(|s| !s.is_empty())
        });
        let endpoint = account
            .endpoint_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(match effective_service {
                "openai" => "https://api.openai.com/v1",
                "anthropic" => "https://api.anthropic.com/v1",
                "gemini" => "https://cloudcode-pa.googleapis.com",
                "copilot" => "https://api.githubcopilot.com",
                "antigravity" => "https://cloudcode-pa.googleapis.com",
                "deepseek" => "https://api.deepseek.com/v1",
                "local" => "http://localhost:11434",
                _ => "",
            });
        let key = custos_provider::routing::probe::verified_catalog_key(
            &account.id,
            endpoint,
            oauth,
            metadata.as_ref().map(|m| m.created_at).unwrap_or(0),
            effective_service,
        );
        if availability.check(&key, model).is_err() {
            continue;
        }
        if repo.list_catalog_models(Some(&key))?.iter().any(|m| {
            m.id == model
                && custos_provider::routing::probe::conversation_model_is_visible(
                    &m.id,
                    Some(&m.label),
                )
                && effort.is_none_or(|value| m.efforts.iter().any(|supported| supported == value))
        }) {
            eligible.push(account.id.clone());
        }
    }
    if stored && eligible.is_empty() {
        return Err(DomainError::Validation(
            "No active account advertises this model. Refresh models or reconnect the provider."
                .into(),
        ));
    }
    let selected = repo.select_account(service, &eligible)?;
    tracing::debug!(service, account_id = ?selected, "Reserved provider account");
    Ok(selected)
}

/// Recompute identity from current provider/credential metadata, never from model family.
pub(crate) fn provider_catalog_scope(
    repo: &custos_persistence::ProviderRepository,
    account: &str,
) -> Result<String, DomainError> {
    let record = repo.get_provider(account)?.ok_or_else(|| {
        DomainError::Validation("Selected provider account no longer exists".into())
    })?;
    let metadata = repo.credential_metadata(account)?;
    let oauth = metadata
        .as_ref()
        .is_some_and(|m| m.has_refresh_token || m.scope.as_deref().is_some_and(|s| !s.is_empty()));
    let endpoint = record
        .endpoint_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(match record.service_type.as_str() {
            "openai" => "https://api.openai.com/v1",
            "anthropic" => "https://api.anthropic.com/v1",
            "gemini" => "https://generativelanguage.googleapis.com",
            "copilot" => "https://api.githubcopilot.com",
            "antigravity" => "https://cloudcode-pa.googleapis.com",
            "deepseek" => "https://api.deepseek.com/v1",
            "local" => "http://localhost:11434",
            _ => "",
        });
    Ok(custos_provider::routing::probe::verified_catalog_key(
        account,
        endpoint,
        oauth,
        metadata.as_ref().map(|m| m.created_at).unwrap_or(0),
        &record.service_type,
    ))
}
