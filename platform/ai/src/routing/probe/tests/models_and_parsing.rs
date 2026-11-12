use super::super::*;

#[test]
fn test_parse_ollama_tags() {
    let body = r#"{"models":[{"name":"llama3:8b","size":4000000},{"name":"qwen3:14b"}]}"#;
    let tags = parse_ollama_tags(body).unwrap();
    assert_eq!(tags, vec!["llama3:8b".to_string(), "qwen3:14b".to_string()]);
}

#[test]
fn test_parse_ollama_show_context() {
    let body = r#"{
        "model_info": {
            "llama.context_length": 131072,
            "general.architecture": "llama"
        }
    }"#;
    assert_eq!(parse_ollama_show_context(body), Some(131072));
}

#[test]
fn test_parse_openai_models() {
    let body = r#"{
        "data": [
            {
                "id": "qwen2.5-coder-32b",
                "owned_by": "vllm",
                "max_model_len": 65536
            },
            {
                "id": "gpt-4o",
                "context_length": 128000
            }
        ]
    }"#;
    let models = parse_openai_models(body).unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "qwen2.5-coder-32b");
    assert_eq!(models[0].context_window, Some(65536));
    assert_eq!(models[0].owned_by, Some("vllm".into()));
    assert_eq!(models[1].id, "gpt-4o");
    assert_eq!(models[1].context_window, Some(128000));
}

#[test]
fn test_parse_gemini_models() {
    let body = r#"{
        "models": [
            {
                "name": "models/gemini-2.5-pro",
                "displayName": "Gemini 2.5 Pro",
                "description": "State-of-the-art multimodal model",
                "inputTokenLimit": 1048576
            }
        ]
    }"#;
    let models = parse_gemini_models(body).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "gemini-2.5-pro");
    assert_eq!(models[0].name, Some("Gemini 2.5 Pro".into()));
    assert_eq!(models[0].context_window, Some(1048576));
}

#[test]
fn test_parse_chatgpt_codex_models() {
    let body = r#"{
        "models": [
            {
                "slug": "gpt-5.6-sol",
                "title": "GPT-5.6 Sol",
                "description": "Flagship Codex model",
                "max_tokens": 200000,
                "visibility": "list"
            },
            {
                "slug": "gpt-5.6-terra",
                "title": null,
                "description": null,
                "max_tokens": null,
                "visibility": "list"
            }
        ]
    }"#;
    let models = parse_chatgpt_codex_models(body).unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "gpt-5.6-sol");
    assert_eq!(models[0].name, Some("GPT-5.6 Sol".into()));
    assert_eq!(models[0].context_window, Some(200000));
    assert_eq!(models[0].owned_by, Some("openai-chatgpt".into()));
    assert_eq!(models[1].id, "gpt-5.6-terra");
    assert_eq!(models[1].name, Some("gpt-5.6-terra".into()));
    assert_eq!(models[1].context_window, None);
}

#[test]
fn roots_and_empty_models() {
    assert_eq!(
        ollama_root(" http://localhost:11434/v1/ ").unwrap(),
        "http://localhost:11434"
    );
    assert!(ollama_root("file:///models").is_err());
    assert!(ollama_root("http://localhost:11434?key=x").is_err());
    assert_eq!(parse_ollama_tags(r#"{"models":[]}"#), Some(vec![]));
}

#[tokio::test]
async fn empty_native_discovery_does_not_fall_back() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0; 4096];
        let n = socket.read(&mut buf).unwrap();
        let body = r#"{"models":[]}"#;
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&buf[..n]).starts_with("GET /api/tags "));
    });
    assert!(probe_endpoint_models(&base, None, "ollama")
        .await
        .unwrap()
        .is_empty());
    server.join().unwrap();
}

#[test]
fn conversation_curation_covers_versions_alias_labels_and_tab_internal_ids() {
    for version in ["2.5", "3", "3.1", "3.5", "3.6"] {
        assert!(!conversation_model_is_visible(
            &format!("antigravity/gemini-{version}-flash"),
            None
        ));
        assert!(!conversation_model_is_visible(
            "antigravity/gemini-pro-agent",
            Some(&format!("Gemini {version} Pro (High)"))
        ));
    }
    for id in [
        "antigravity/tab_flash_lite_preview",
        "antigravity/tab_jump_flash_lite_preview",
        "Tab Internal",
    ] {
        assert!(!conversation_model_is_visible(id, None));
    }
    for id in [
        "antigravity/gemini-3.8-flash-tiered",
        "gemini-3.7-flash",
        "gemini-3.10-flash",
        "claude-3-5-sonnet",
        "gpt-6.1-sol",
    ] {
        assert!(conversation_model_is_visible(id, None));
    }
}

#[test]
fn copilot_subscription_policy_overrides_only_the_legacy_picker_flag() {
    let body = r#"{"data":[
        {"id":"enabled","name":"Enabled chat","model_picker_enabled":false,"policy":{"state":"enabled"},"capabilities":{"type":"chat","limits":{"max_context_window_tokens":128000}},"supported_endpoints":["/chat/completions"]},
        {"id":"disabled","model_picker_enabled":true,"policy":{"state":"disabled"},"capabilities":{"type":"chat"}},
        {"id":"unconfigured","model_picker_enabled":true,"policy":{"state":"unconfigured"},"capabilities":{"type":"chat"}},
        {"id":"internal","model_picker_enabled":false,"capabilities":{"type":"chat"}},
        {"id":"embedding","policy":{"state":"enabled"},"capabilities":{"type":"embeddings"}},
        {"id":"hidden","policy":{"state":"enabled"},"isInternal":true,"capabilities":{"type":"chat"}},
        {"id":"responses-only","policy":{"state":"enabled"},"capabilities":{"type":"chat"},"supported_endpoints":["/responses"]},
        {"id":"legacy","model_picker_enabled":true,"capabilities":{"type":"chat"}}
    ]}"#;
    let models = parse_provider_models(body, Some("/chat/completions"), true).unwrap();
    assert_eq!(
        models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        vec!["enabled", "legacy"]
    );
    assert_eq!(models[0].context_window, Some(128000));
    let generic = parse_openai_models_for_transport(body, Some("/chat/completions")).unwrap();
    assert!(!generic.iter().any(|model| model.id == "enabled"));
    assert!(
        parse_provider_models(r#"{"data":[]}"#, Some("/chat/completions"), true)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn codex_visibility_efforts_and_empty_results_are_authoritative() {
    let models = parse_chatgpt_codex_models(r#"{"models":[
        {"slug":"gpt-6.1-sol","display_name":"GPT-6.1 Sol","visibility":"list","supported_in_api":false,"default_reasoning_level":"low","supported_reasoning_levels":[{"effort":"low"},{"effort":"xhigh"},{"effort":"max"},{"effort":"ultra"}]},
        {"slug":"internal","visibility":"hide"}, {"slug":"disabled","visibility":"list","disabled":true},
        {"slug":"audio","visibility":"list","output_modalities":["audio"]}
    ]}"#).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "gpt-6.1-sol");
    assert_eq!(models[0].name.as_deref(), Some("GPT-6.1 Sol"));
    assert_eq!(models[0].efforts, vec!["low", "xhigh", "max", "ultra"]);
    assert_eq!(models[0].default_effort.as_deref(), Some("low"));
    assert!(parse_chatgpt_codex_models(r#"{"models":[]}"#)
        .unwrap()
        .is_empty());
    assert!(parse_chatgpt_codex_models(
        r#"{"models":[{"slug":"internal","visibility":"hide"}]}"#
    )
    .unwrap()
    .is_empty());
    assert!(parse_chatgpt_codex_models("{}").is_err());
}

#[test]
fn discovery_does_not_offer_embedding_or_disabled_models() {
    let models = parse_openai_models(r#"{"data":[{"id":"text-embedding-3-large"},{"id":"gpt-image-1"},{"id":"disabled","available":false},{"id":"gpt-6-sol","supported_reasoning_levels":["low","high"],"default_effort":"invalid"}]}"#).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "gpt-6-sol");
    assert_eq!(models[0].default_effort, None);
    assert_eq!(models[0].efforts, vec!["low", "high"]);
    assert!(parse_openai_models(r#"{"data":[]}"#).unwrap().is_empty());
}

#[test]
fn account_endpoint_auth_generation_and_protocol_scope_do_not_mix() {
    let key = verified_catalog_key("A", "https://example/v1/", true, 1, "openai");
    assert_eq!(
        key,
        format!("verified-probe:A:https://example/v1:oauth:1:codex-{CODEX_CLIENT_VERSION}")
    );
    assert_ne!(
        key,
        verified_catalog_key("B", "https://example/v1", true, 1, "openai")
    );
    assert_ne!(
        key,
        verified_catalog_key("A", "https://other/v1", true, 1, "openai")
    );
    assert_ne!(
        key,
        verified_catalog_key("A", "https://example/v1", false, 1, "openai")
    );
    assert_ne!(
        key,
        verified_catalog_key("A", "https://example/v1", true, 2, "openai")
    );
}

#[test]
fn discovery_excludes_models_whose_advertised_endpoint_is_not_implemented() {
    let body = r#"{"data":[{"id":"gpt-6-sol","supported_endpoints":["/responses"]},{"id":"chat-alias","supported_endpoints":["/chat/completions"]},{"id":"embedding","supported_endpoints":["/embeddings"]}]}"#;
    let openai = parse_openai_models(body).unwrap();
    assert_eq!(openai.len(), 2);
    let copilot = parse_openai_models_for_transport(body, Some("/chat/completions")).unwrap();
    assert_eq!(copilot.len(), 1);
    assert_eq!(copilot[0].id, "chat-alias");
}
