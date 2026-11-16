use super::*;
use crate::base::MessageStream;
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;

struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    fn get_name(&self) -> &str {
        "mock-provider"
    }

    async fn complete(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[rmcp::model::Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let msg = Message::assistant().with_text("mocked response");
        let usage = ProviderUsage::new(
            "mock-model".to_string(),
            Usage::new(Some(10), Some(20), None),
        );
        Ok((msg, usage))
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError> {
        let s = async_stream::stream! {
            yield Ok((Some(Message::assistant().with_text("streamed")), None));
        };
        Ok(Box::pin(s))
    }

    async fn get_context_limit(&self, _model: &str, override_limit: Option<usize>) -> usize {
        override_limit.unwrap_or(4096)
    }
}

struct MockModelPort;

#[async_trait]
impl ModelProvider for MockModelPort {
    fn provider_id(&self) -> &str {
        "mock-port"
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        Ok(
            ModelResponse::text(format!("echo: {}", req.prompt), &req.model, 30).with_usage(
                Some(10),
                Some(20),
                Some(30),
            ),
        )
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(4);
        let req_id = req.request_id.clone();
        tokio::spawn(async move {
            let _ = tx
                .send(ProviderEvent::chunk(req_id.clone(), "chunk1"))
                .await;
            let _ = tx
                .send(ProviderEvent::completed(
                    req_id,
                    Some("chunk1".to_string()),
                    None,
                ))
                .await;
        });
        Ok(rx)
    }
}

#[tokio::test]
async fn test_provider_to_model_port_roundtrip() {
    let provider = Arc::new(MockProvider);
    let port = ProviderToModelPort::new(provider);

    assert_eq!(port.provider_id(), "mock-provider");

    let req = ProviderRequest::new("req_1", "task_1", 1, "hello", "test-model");
    let resp = port.generate(&req).await.unwrap();
    assert_eq!(resp.content, "mocked response");
    assert_eq!(resp.input_tokens, Some(10));
    assert_eq!(resp.output_tokens, Some(20));
}

#[tokio::test]
async fn test_model_port_to_provider_roundtrip() {
    let port = Arc::new(MockModelPort);
    let provider = ModelPortToProvider::new(port);

    assert_eq!(provider.get_name(), "mock-port");

    let config = ModelConfig::new("test-model");
    let messages = vec![Message::user().with_text("ping")];
    let (msg, usage) = provider
        .complete(&config, "system prompt", &messages, &[])
        .await
        .unwrap();

    let text = msg.content[0].as_text().unwrap();
    assert!(text.contains("echo: System: system prompt"));
    assert!(text.contains("User: ping"));
    assert_eq!(usage.usage.input_tokens, Some(10));
    assert_eq!(usage.usage.output_tokens, Some(20));
}

