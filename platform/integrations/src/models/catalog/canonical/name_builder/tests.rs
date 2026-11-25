use super::*;

#[test]
fn test_map_to_canonical_model() {
    let r = crate::catalog::canonical::CanonicalModelRegistry::bundled().unwrap();

    // === Direct provider (non-hosting) ===
    assert_eq!(
        map_to_canonical_model("anthropic", "claude-sonnet-4-5-20250929", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("openai", "gpt-4o-latest", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("xai_oauth", "grok-4.5", r),
        Some("x-ai/grok-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("openai", "gpt-4-turbo-2024-04-09", r),
        Some("openai/gpt-4-turbo".to_string())
    );
    assert_eq!(
        map_to_canonical_model("opencode_go", "kimi-k2.6", r),
        Some("opencode-go/kimi-k2.6".to_string())
    );
    assert_eq!(
        map_to_canonical_model("opencode_zen", "kimi-k3", r),
        Some("opencode/kimi-k3".to_string())
    );

    // === OpenRouter ===
    assert_eq!(
        map_to_canonical_model("openrouter", "anthropic/claude-sonnet-4.5", r),
        Some("openrouter/anthropic/claude-sonnet-4.5".to_string())
    );

    // === Anthropic Claude - basic ===
    assert_eq!(
        map_to_canonical_model("databricks", "claude-sonnet-4-5", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "claude-sonnet-4-5-20250929", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "claude-sonnet-4-5-latest", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );

    // 4.x: {version}-{model} → {model}-{version}
    assert_eq!(
        map_to_canonical_model("databricks", "claude-sonnet-4-5", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );

    // 4.x with minor version + prefix stripping
    assert_eq!(
        map_to_canonical_model("databricks", "raml-claude-opus-4-5", r),
        Some("anthropic/claude-opus-4.5".to_string())
    );

    // === Claude with platform suffixes ===
    assert_eq!(
        map_to_canonical_model("databricks", "claude-sonnet-4-5-bedrock", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "goose-claude-sonnet-4-5-bedrock", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("bedrock", "claude-sonnet-4-5", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("aws_bedrock", "global.anthropic.claude-sonnet-5", r),
        Some("amazon-bedrock/global.anthropic.claude-sonnet-5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("anthropic", "claude-sonnet-5", r),
        Some("anthropic/claude-sonnet-5".to_string())
    );

    // === OpenAI GPT ===
    assert_eq!(
        map_to_canonical_model("databricks", "gpt-4o", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "gpt-4o-2024-11-20", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "gpt-4o-latest", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "kgoose-gpt-4o", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("azure", "gpt-4o", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("azure_foundry", "gpt-4o", r),
        Some("openai/gpt-4o".to_string())
    );

    // === OpenAI O-series ===
    assert_eq!(
        map_to_canonical_model("databricks", "goose-o1", r),
        Some("openai/o1".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "kgoose-o3", r),
        Some("openai/o3".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "headless-goose-o3-mini", r),
        Some("openai/o3-mini".to_string())
    );

    // === Google Gemini ===
    assert_eq!(
        map_to_canonical_model("databricks", "gemini-2-5-flash", r),
        Some("google/gemini-2.5-flash".to_string())
    );

    // === Meta Llama ===
    assert_eq!(
        map_to_canonical_model("databricks", "meta-llama-3-3-70b-instruct", r),
        Some("meta-llama/llama-3.3-70b-instruct".to_string())
    );

    // === Mistral variants ===
    assert_eq!(
        map_to_canonical_model("databricks", "codestral", r),
        Some("mistralai/codestral".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "ministral-3b", r),
        Some("mistralai/ministral-3b".to_string())
    );

    // === DeepSeek ===
    assert_eq!(
        map_to_canonical_model("databricks", "databricks-deepseek-v4-flash", r),
        Some("deepseek/deepseek-v4-flash".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "deepseek-v4-pro", r),
        Some("deepseek/deepseek-v4-pro".to_string())
    );

    // === Grok (X.AI) ===
    assert_eq!(
        map_to_canonical_model("databricks", "grok-4.3", r),
        Some("x-ai/grok-4.3".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "databricks-grok-4.3", r),
        Some("x-ai/grok-4.3".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "kgoose-grok-4.3", r),
        Some("x-ai/grok-4.3".to_string())
    );

    // === Cohere Command ===
    // Note: version suffix "-2024" is stripped by canonical_name
    assert_eq!(
        map_to_canonical_model("databricks", "command-r-plus-08-2024", r),
        Some("cohere/command-r-plus-08".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "goose-command-r-08-2024", r),
        Some("cohere/command-r-08".to_string())
    );

    // === Provider-prefixed extraction ===
    assert_eq!(
        map_to_canonical_model("databricks", "anthropic-claude-sonnet-4-5", r),
        Some("anthropic/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "openai-gpt-4o", r),
        Some("openai/gpt-4o".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "google-gemini-2-5-flash", r),
        Some("google/gemini-2.5-flash".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "mistralai-codestral", r),
        Some("mistralai/codestral".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "deepseek-deepseek-v4-flash", r),
        Some("deepseek/deepseek-v4-flash".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks", "x-ai-grok-4.3", r),
        Some("x-ai/grok-4.3".to_string())
    );

    // === Zhipu AI ===
    assert_eq!(
        map_to_canonical_model("zhipu", "glm-4.7", r),
        Some("zhipuai/glm-4.7".to_string())
    );
    assert_eq!(
        map_to_canonical_model("zhipu", "glm-5", r),
        Some("zhipuai/glm-5".to_string())
    );

    // === Kimi Code ===
    assert_eq!(
        map_to_canonical_model("kimi_code", "kimi-for-coding", r),
        Some("kimi-code-plan-cn/kimi-for-coding".to_string())
    );
    assert_eq!(
        map_to_canonical_model("kimi_code", "kimi-for-coding-highspeed", r),
        Some("kimi-code-plan-cn/kimi-for-coding-highspeed".to_string())
    );
    assert_eq!(
        map_to_canonical_model("kimi_code", "k3", r),
        Some("kimi-code-plan-cn/k3".to_string())
    );

    // === GCP Vertex AI ===
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "gemini-2.5-flash", r),
        Some("google-vertex/gemini-2.5-flash".to_string())
    );
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "gemini-2.5-pro", r),
        Some("google-vertex/gemini-2.5-pro".to_string())
    );
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "claude-sonnet-4@20250514", r),
        Some("google-vertex/claude-sonnet-4".to_string())
    );
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "claude-sonnet-4-5@20250929", r),
        Some("google-vertex/claude-sonnet-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "claude-opus-4-5@20251101", r),
        Some("google-vertex/claude-opus-4.5".to_string())
    );
    assert_eq!(
        map_to_canonical_model("gcp_vertex_ai", "claude-haiku-4-5@20251001", r),
        Some("google-vertex/claude-haiku-4.5".to_string())
    );
}

#[test]
fn test_databricks_native_open_weight_ids_resolve() {
    let r = crate::catalog::canonical::CanonicalModelRegistry::bundled().unwrap();

    assert_eq!(
        map_to_canonical_model("databricks_v2", "databricks-gpt-oss-120b", r),
        Some("databricks/databricks-gpt-oss-120b".to_string())
    );
    assert_eq!(
        map_to_canonical_model("databricks_v2", "databricks-gpt-oss-20b", r),
        Some("databricks/databricks-gpt-oss-20b".to_string())
    );
    // Legacy provider name resolves identically.
    assert_eq!(
        map_to_canonical_model("databricks", "databricks-gpt-oss-120b", r),
        Some("databricks/databricks-gpt-oss-120b".to_string())
    );

    // Regression guard: the meta-provider lookup must remain a *fallback*
    // after inference. databricks-claude-* aliases infer back to the richer
    // first-party "anthropic/*" entry (which carries thinking_mode used for
    // adaptive thinking), not the metadata-poor "databricks/databricks-*" one.
    assert_eq!(
        map_to_canonical_model("databricks", "databricks-claude-opus-4-7", r),
        Some("anthropic/claude-opus-4.7".to_string())
    );
}
