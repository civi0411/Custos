use crate::conversation::message::{Message, MessageContentBlock};
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, Role};
use serde_json::{json, Value};

pub(crate) fn set_up_text_message(text: &str, role: Role) -> Message {
    Message::new(role, 0, vec![MessageContentBlock::text(text.to_string())])
}

pub(crate) fn set_up_tool_request_message(id: &str, tool_call: CallToolRequestParams) -> Message {
    Message::new(
        Role::User,
        0,
        vec![MessageContentBlock::tool_request(
            id.to_string(),
            Ok(tool_call),
        )],
    )
}

pub(crate) fn set_up_action_required_message(id: &str, tool_call: CallToolRequestParams) -> Message {
    Message::new(
        Role::User,
        0,
        vec![MessageContentBlock::action_required(
            id.to_string(),
            tool_call.name.to_string().clone(),
            tool_call.arguments.unwrap_or_default().clone(),
            Some("goose would like to call the above tool. Allow? (y/n):".to_string()),
        )],
    )
}

pub(crate) fn set_up_tool_response_message(id: &str, tool_response: Vec<ContentBlock>) -> Message {
    Message::new(
        Role::Assistant,
        0,
        vec![MessageContentBlock::tool_response(
            id.to_string(),
            Ok(CallToolResult::success(tool_response)),
        )],
    )
}

pub(crate) fn google_response(parts: Vec<Value>) -> Value {
    json!({"candidates": [{"content": {"role": "model", "parts": parts}}]})
}

pub(crate) fn tool_result(text: &str) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}
