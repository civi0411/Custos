use super::patterns::{
    extract_provider_prefix, infer_provider_from_model, strip_common_prefixes,
    strip_version_suffix, swap_claude_word_order,
};

/// Build canonical model name from provider and model identifiers
pub fn canonical_name(provider: &str, model: &str) -> String {
    let model_base = strip_version_suffix(model);
    format!("{}/{}", provider, model_base)
}

pub(crate) fn is_meta_provider(provider: &str) -> bool {
    matches!(
        provider,
        "databricks" | "databricks_v2" | "tetrate" | "bedrock" | "azure" | "azure_foundry"
    )
}

pub fn map_provider_name(provider: &str) -> &str {
    match provider {
        // Goose provider names that differ from models.dev names
        "xai" | "xai_oauth" => "x-ai",
        "azure_openai" | "azure_foundry" => "azure",
        "aws_bedrock" => "amazon-bedrock",
        "gcp_vertex_ai" => "google-vertex",
        "gemini_oauth" => "google",
        "databricks_v2" => "databricks",
        "zhipu" => "zhipuai",
        "together" => "togetherai",
        "novita" => "novita-ai",
        "opencode_go" => "opencode-go",
        "opencode_zen" => "opencode",
        "ollama_cloud" => "ollama-cloud",
        "kimi_code" => "kimi-code-plan-cn",
        _ => provider,
    }
}

/// Try to map a provider/model pair to a canonical model
pub fn map_to_canonical_model(
    provider: &str,
    model: &str,
    registry: &crate::catalog::canonical::CanonicalModelRegistry,
) -> Option<String> {
    let registry_provider = map_provider_name(provider);

    if provider == "gcp_vertex_ai" {
        let normalized_model = strip_version_suffix(model);
        if let Some(canonical) = registry.get(registry_provider, &normalized_model) {
            return Some(canonical.id.clone());
        }
        if let Some(canonical) = registry.get(registry_provider, model) {
            return Some(canonical.id.clone());
        }
        if model.starts_with("gemini-") {
            return None;
        }
    }

    // For normal providers (anthropic, openai, google, openrouter, etc.), just do direct lookup
    if !is_meta_provider(provider) && provider != "gcp_vertex_ai" {
        let normalized_model = strip_version_suffix(model);
        if let Some(canonical) = registry.get(registry_provider, &normalized_model) {
            return Some(canonical.id.clone());
        }
        // Also try original model name
        if let Some(canonical) = registry.get(registry_provider, model) {
            return Some(canonical.id.clone());
        }
        // If direct lookup failed, fall through to inference logic below
    }

    // For hosting/meta-providers (or unknown providers), do string matching magic to figure out the real provider and model
    let model_stripped = strip_common_prefixes(model);

    if let Some(swapped) = swap_claude_word_order(&model_stripped) {
        if let Some(inferred_provider) = infer_provider_from_model(&swapped) {
            let normalized = strip_version_suffix(&swapped);
            if let Some(canonical) = registry.get(inferred_provider, &normalized) {
                return Some(canonical.id.clone());
            }
        }
    }

    if let Some(inferred_provider) = infer_provider_from_model(&model_stripped) {
        let normalized = strip_version_suffix(&model_stripped);
        if let Some(canonical) = registry.get(inferred_provider, &normalized) {
            return Some(canonical.id.clone());
        }
    }

    if let Some(inferred_provider) = infer_provider_from_model(model) {
        let normalized = strip_version_suffix(model);
        if let Some(canonical) = registry.get(inferred_provider, &normalized) {
            return Some(canonical.id.clone());
        }
    }

    if let Some((extracted_provider, extracted_model)) = extract_provider_prefix(&model_stripped) {
        let normalized = strip_version_suffix(extracted_model);
        if let Some(canonical) = registry.get(extracted_provider, &normalized) {
            return Some(canonical.id.clone());
        }
    }

    // Fallback for meta-providers: some native aliases are keyed under the
    // meta-provider itself (e.g. "databricks/databricks-gpt-oss-120b") and do
    // not infer back to a first-party provider. Only try this after inference,
    // so models that DO infer (e.g. databricks-claude-* -> anthropic/*) keep
    // resolving to the richer first-party catalog entry.
    if is_meta_provider(provider) {
        if let Some(canonical) = registry.get(registry_provider, model) {
            return Some(canonical.id.clone());
        }
        let normalized_model = strip_version_suffix(model);
        if let Some(canonical) = registry.get(registry_provider, &normalized_model) {
            return Some(canonical.id.clone());
        }
    }

    None
}
