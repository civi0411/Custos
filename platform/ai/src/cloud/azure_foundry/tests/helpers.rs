use crate::cloud::transport::api_client::AuthMethod;
use crate::cloud::azure_foundry::provider::AzureFoundryProvider;
use crate::model::ModelConfig;
use serde_json::json;
use wiremock::MockServer;

pub(crate) fn project_endpoint(server: &MockServer) -> String {
    format!("{}/api/projects/test", server.uri())
}

pub(crate) fn raw_model_config(model_name: &str) -> ModelConfig {
    let mut config = ModelConfig::new(model_name);
    config.model_name = model_name.to_string();
    config
}

pub(crate) fn project_provider(server: &MockServer) -> AzureFoundryProvider {
    AzureFoundryProvider::create(
        project_endpoint(server),
        None,
        None,
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        None,
        None,
    )
    .unwrap()
}

pub(crate) fn chat_stream() -> String {
    [
        json!({"id":"c1","object":"chat.completion.chunk","model":"test","choices":[{"delta":{"role":"assistant","content":"Hello"},"index":0}]}),
        json!({"id":"c1","object":"chat.completion.chunk","choices":[{"delta":{},"finish_reason":"stop","index":0}]}),
        json!({"id":"c1","object":"chat.completion.chunk","choices":[],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}),
    ]
    .into_iter()
    .map(|chunk| format!("data: {chunk}\n\n"))
    .chain(std::iter::once("data: [DONE]\n\n".to_string()))
    .collect()
}

pub(crate) fn responses_stream() -> String {
    let created = r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":0,"status":"in_progress","model":"gpt-5","output":[]}}"#;
    let delta = r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"m1","output_index":0,"content_index":0,"delta":"Hello"}"#;
    let completed = r#"data: {"type":"response.completed","sequence_number":3,"response":{"id":"resp_1","object":"response","created_at":0,"status":"completed","model":"gpt-5","output":[],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}"#;
    format!("{created}\n\n{delta}\n\n{completed}\n\ndata: [DONE]\n\n")
}

pub(crate) fn anthropic_stream() -> String {
    let start = r#"data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}"#;
    let block_start = r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
    let delta = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
    let block_stop = r#"data: {"type":"content_block_stop","index":0}"#;
    let message_delta = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}"#;
    let stop = r#"data: {"type":"message_stop"}"#;
    format!(
        "{start}\n\n{block_start}\n\n{delta}\n\n{block_stop}\n\n{message_delta}\n\n{stop}\n\n"
    )
}
