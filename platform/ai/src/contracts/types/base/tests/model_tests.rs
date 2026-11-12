use crate::base::{
    MessageStream, ModelInfo, Provider, ThinkingPreservationFormat,
};
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::model::ModelConfig;
use async_trait::async_trait;
use rmcp::model::Tool;

struct ModelInventoryProvider {
    models: Vec<String>,
}

#[async_trait]
impl Provider for ModelInventoryProvider {
    fn get_name(&self) -> &str {
        "xai"
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        unimplemented!()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(self.models.clone())
    }
}

#[tokio::test]
async fn recommended_models_preserve_unknown_future_models() {
    let provider = ModelInventoryProvider {
        models: vec![
            "grok-4.5".to_string(),
            "grok-future-unlisted".to_string(),
            "grok-4.20-multi-agent".to_string(),
        ],
    };

    let models = provider.fetch_recommended_models(false).await.unwrap();

    assert!(models.contains(&"grok-4.5".to_string()));
    assert!(models.contains(&"grok-future-unlisted".to_string()));
    assert!(!models.contains(&"grok-4.20-multi-agent".to_string()));
}

#[test]
fn test_model_info_creation() {
    let info = ModelInfo {
        name: "test-model".to_string(),
        resolved_model: None,
        context_limit: Some(1000),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning: false,
        thinking_preservation_format: None,
        request_params: None,
    };
    assert_eq!(info.context_limit, Some(1000));

    let info2 = ModelInfo {
        name: "test-model".to_string(),
        resolved_model: None,
        context_limit: Some(1000),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning: false,
        thinking_preservation_format: None,
        request_params: None,
    };
    assert_eq!(info, info2);

    let info3 = ModelInfo {
        name: "test-model".to_string(),
        resolved_model: None,
        context_limit: Some(2000),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning: false,
        thinking_preservation_format: None,
        request_params: None,
    };
    assert_ne!(info, info3);
}

#[test]
fn test_model_info_deserializes_thinking_preservation_and_request_params() {
    let info: ModelInfo = serde_json::from_str(
        r#"{
            "name": "zai-glm-4.7",
            "context_limit": 131072,
            "thinking_preservation_format": "content_xml",
            "request_params": {"reasoning_format": "parsed"}
        }"#,
    )
    .unwrap();

    assert_eq!(
        info.thinking_preservation_format,
        Some(ThinkingPreservationFormat::ContentXml)
    );
    assert_eq!(
        info.request_params.unwrap().get("reasoning_format"),
        Some(&serde_json::json!("parsed"))
    );

    let bare: ModelInfo =
        serde_json::from_str(r#"{"name": "gpt-4o", "context_limit": 128000}"#).unwrap();
    assert_eq!(bare.thinking_preservation_format, None);
    assert_eq!(bare.request_params, None);
}

#[test]
fn test_model_info_with_cost() {
    let info = ModelInfo::with_cost("gpt-4o", 128000, 0.0000025, 0.00001);
    assert_eq!(info.name, "gpt-4o");
    assert_eq!(info.context_limit, Some(128000));
    assert_eq!(info.input_token_cost, Some(0.0000025));
    assert_eq!(info.output_token_cost, Some(0.00001));
    assert_eq!(info.currency, Some("$".to_string()));
}
