use super::helpers::{custom_config, make_provider};
use crate::cloud::openai::builder::{
    derive_base_path, ensure_url_scheme, from_declarative_config,
};
use crate::cloud::openai::provider::OpenAiProvider;
use crate::model::ModelConfig;
use serde_json::json;

#[test]
fn coding_plan_tool_stream_is_scoped_and_preserves_explicit_overrides() {
    let model = ModelConfig::new("glm-future");
    for (provider, payload, expected) in [
        (
            "zai_coding_plan",
            json!({"stream": true}),
            Some(json!(true)),
        ),
        ("zai_coding_plan", json!({"stream": false}), None),
        ("openai", json!({"stream": true}), None),
        (
            "zai_coding_plan",
            json!({"stream": true, "tool_stream": false}),
            Some(json!(false)),
        ),
    ] {
        let request = make_provider(provider).sanitize_request_for_compat(payload, &model);
        assert_eq!(request.get("tool_stream"), expected.as_ref());
    }
}

#[test]
fn nearai_uses_chat_completions_for_openai_reasoning_models() {
    let provider = make_provider("nearai");

    assert!(!provider.should_use_responses_api_for_provider("openai/gpt-5"));
    assert!(!provider.should_use_responses_api_for_provider("openai/o3"));
}

#[test]
fn responses_api_routing_uses_model_family_unless_path_forces_chat() {
    for (model_name, base_path, expected) in [
        ("gpt-5.4", "v1/chat/completions", true),
        ("gpt-5.4-xhigh", "v1/chat/completions", true),
        ("gpt-5.6-sol", "v1/chat/completions", true),
        ("gpt-5.6-terra-xhigh", "v1/chat/completions", true),
        ("gpt-5.2-pro-2025-12-11", "v1/chat/completions", true),
        ("gpt-4o", "v1/chat/completions", false),
        ("gpt-5.2-codex", "openai/v1/chat/completions", false),
    ] {
        assert_eq!(
            OpenAiProvider::should_use_responses_api(model_name, base_path),
            expected,
            "unexpected routing for {model_name} via {base_path}"
        );
    }
}

#[test]
fn custom_chat_path_maps_to_responses_path() {
    let responses_path =
        OpenAiProvider::map_base_path("openai/v1/chat/completions", "responses", "v1/responses");
    assert_eq!(responses_path, "openai/v1/responses");
}

#[test]
fn responses_path_maps_to_models_path() {
    let models_path =
        OpenAiProvider::map_base_path("openai/v1/responses", "models", "v1/models");
    assert_eq!(models_path, "openai/v1/models");
}

#[test]
fn unknown_path_falls_back_to_default_models_path() {
    let models_path = OpenAiProvider::map_base_path("custom/path", "models", "v1/models");
    assert_eq!(models_path, "v1/models");
}

#[test]
fn absolute_chat_path_maps_to_absolute_responses_path() {
    let responses_path =
        OpenAiProvider::map_base_path("/v1/chat/completions", "responses", "v1/responses");
    assert_eq!(responses_path, "/v1/responses");
}

#[test]
fn unknown_absolute_path_falls_back_to_absolute_models_path() {
    let models_path = OpenAiProvider::map_base_path("/custom/path", "models", "v1/models");
    assert_eq!(models_path, "/v1/models");
}

#[test]
fn versionless_base_path_opts_out_of_responses_for_codex_models() {
    assert!(!OpenAiProvider::should_use_responses_api(
        "gpt-5-codex",
        "chat/completions"
    ));
}

#[test]
fn custom_host_with_chat_completions_path_avoids_responses_routing() {
    // Custom hosts must use versionless "chat/completions" to keep should_use_responses_api
    // returning false — the path is detected as custom AND matches chat completions.
    assert!(!OpenAiProvider::should_use_responses_api(
        "o3",
        "chat/completions"
    ));
    assert!(!OpenAiProvider::should_use_responses_api(
        "gpt-5",
        "chat/completions"
    ));
}

#[test]
fn ensure_url_scheme_adds_http_for_local_hosts() {
    assert_eq!(ensure_url_scheme("localhost:1234"), "http://localhost:1234");
    assert_eq!(
        ensure_url_scheme("127.0.0.1:8080/v1"),
        "http://127.0.0.1:8080/v1"
    );
    assert_eq!(ensure_url_scheme("0.0.0.0:3000"), "http://0.0.0.0:3000");
    assert_eq!(ensure_url_scheme("[::1]:1234"), "http://[::1]:1234");
}

#[test]
fn ensure_url_scheme_adds_https_for_remote_hosts() {
    assert_eq!(
        ensure_url_scheme("api.example.com:8443/v1"),
        "https://api.example.com:8443/v1"
    );
    assert_eq!(ensure_url_scheme("example.com"), "https://example.com");
}

#[test]
fn ensure_url_scheme_preserves_existing_scheme() {
    assert_eq!(
        ensure_url_scheme("http://localhost:1234"),
        "http://localhost:1234"
    );
    assert_eq!(
        ensure_url_scheme("https://api.openai.com/v1"),
        "https://api.openai.com/v1"
    );
}

#[test]
fn from_custom_config_preserves_ipv6_authority() {
    let provider = from_declarative_config(
        custom_config("http://[::1]:1234/v1"),
        None,
        crate::cloud::declarative::EnvKeyResolver,
    )
    .unwrap()
    .build();

    assert_eq!(provider.api_client.host(), "http://[::1]:1234");
}

#[test]
fn from_custom_config_preserves_userinfo_authority() {
    let provider = from_declarative_config(
        custom_config("https://user:pass@gateway.example/v1"),
        None,
        crate::cloud::declarative::EnvKeyResolver,
    )
    .unwrap()
    .build();

    assert_eq!(
        provider.api_client.host(),
        "https://user:pass@gateway.example"
    );
}

#[test]
fn derive_base_path_not_removing_api_path() {
    let r = derive_base_path("https://opencode.ai/zen/go");
    assert_eq!(r, "https://opencode.ai/zen/go/v1/chat/completions");
}

#[test]
fn derive_base_path_should_support_v1() {
    let r = derive_base_path("https://opencode.ai/zen/go/v1");
    assert_eq!(r, "https://opencode.ai/zen/go/v1/chat/completions");
}

#[test]
fn derive_base_path_should_support_no_base_path() {
    let r = derive_base_path("https://opencode.ai/");
    assert_eq!(r, "https://opencode.ai/v1/chat/completions");
}

#[test]
fn derive_base_path_preserves_non_v1_version_prefix() {
    let r = derive_base_path("/api/paas/v4");
    assert_eq!(r, "api/paas/v4/chat/completions");
}

#[test]
fn derive_base_path_does_not_treat_v_word_as_version() {
    let r = derive_base_path("/api/voice");
    assert_eq!(r, "api/voice/v1/chat/completions");
}

#[test]
fn metadata_comes_from_the_registry() {
    use crate::base::ProviderDescriptor;
    let metadata = OpenAiProvider::metadata();
    let gpt_4o = metadata
        .known_models
        .iter()
        .find(|model| model.name == "gpt-4o")
        .expect("gpt-4o should come from the catalog");
    assert_eq!(gpt_4o.context_limit, Some(128_000));
    assert!(
        metadata
            .known_models
            .iter()
            .any(|model| model.name == "gpt-4"),
        "registry-only list should include models the old hardcoded list had drifted past"
    );
}
