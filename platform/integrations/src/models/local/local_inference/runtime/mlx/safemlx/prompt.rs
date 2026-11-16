use crate::local_inference::native_tool_parsing::message_from_native_tool_text;
use crate::local_inference::provider_utils::filter_extensions_from_system_prompt;
use crate::local_inference::tool_emulation::{
    build_emulator_tool_description, load_tiny_model_prompt,
};
use crate::local_inference::extract_text_content;
use custos_provider::conversation::message::Message;
use custos_provider::errors::ProviderError;
use custos_provider::codecs::openai;
use custos_provider::images::ImageFormat;
use safemlx_lm::models::LoadedModel;
use safemlx_lm_utils::tokenizer::{Chat, Conversation, Role};
use serde_json::json;

use super::generation::mlx_error;

#[derive(Clone, Copy)]
pub(crate) enum ToolMode {
    None,
    Native,
    Emulated { code_mode_enabled: bool },
}

pub(crate) fn build_prompt(
    model: &mut LoadedModel,
    model_name: &str,
    system: &str,
    messages: &[Message],
    tools: &[rmcp::model::Tool],
    tool_mode: ToolMode,
) -> Result<String, ProviderError> {
    match tool_mode {
        ToolMode::Native => {
            let conversations = openai_messages(system, messages);
            let tool_specs = openai::format_tools(tools)
                .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
            if let Some(prompt) = model
                .apply_chat_template_json([conversations], Some(&tool_specs), true)
                .map_err(mlx_error)?
            {
                return Ok(prompt);
            }

            Ok(render_prompt(system, messages))
        }
        ToolMode::Emulated { code_mode_enabled } => {
            let system_prompt = format!(
                "{}{}",
                load_tiny_model_prompt(),
                build_emulator_tool_description(tools, code_mode_enabled)
            );
            if is_gemma4(model) {
                let conversations = gemma4_messages_with_system(&system_prompt, messages);
                if let Some(prompt) = model
                    .apply_chat_template_json([conversations], None, true)
                    .map_err(mlx_error)?
                {
                    return Ok(prompt);
                }
            }

            let conversations = chat_conversations(&system_prompt, messages);
            if let Some(prompt) = model
                .apply_chat_template([Chat::Owned(conversations)], None, true)
                .map_err(mlx_error)?
            {
                return Ok(prompt);
            }

            Ok(render_prompt(&system_prompt, messages))
        }
        ToolMode::None => {
            if is_gemma4(model) {
                let conversations = gemma4_messages(model_name, system, messages);
                if let Some(prompt) = model
                    .apply_chat_template_json([conversations], None, true)
                    .map_err(mlx_error)?
                {
                    return Ok(prompt);
                }
            }

            let conversations = chat_conversations(system, messages);
            if let Some(prompt) = model
                .apply_chat_template([Chat::Owned(conversations)], None, true)
                .map_err(mlx_error)?
            {
                return Ok(prompt);
            }

            Ok(render_prompt(system, messages))
        }
    }
}

pub(crate) fn is_gemma4(model: &LoadedModel) -> bool {
    matches!(model.model_type(), "gemma4" | "gemma4_text")
}

pub(crate) fn gemma4_messages(
    model_name: &str,
    system: &str,
    messages: &[Message],
) -> Vec<serde_json::Value> {
    let system = gemma4_system_prompt(model_name, system);
    gemma4_messages_with_optional_system(system.as_deref(), messages)
}

pub(crate) fn gemma4_messages_with_system(system: &str, messages: &[Message]) -> Vec<serde_json::Value> {
    gemma4_messages_with_optional_system(Some(system), messages)
}

pub(crate) fn gemma4_messages_with_optional_system(
    system: Option<&str>,
    messages: &[Message],
) -> Vec<serde_json::Value> {
    let mut values = Vec::new();
    if let Some(system) = system.map(str::trim).filter(|system| !system.is_empty()) {
        values.push(json!({
            "role": "system",
            "content": system,
        }));
    }

    for message in messages.iter().filter(|message| message.is_agent_visible()) {
        let text = extract_text_content(message);
        if text.trim().is_empty() {
            continue;
        }

        match message.role {
            rmcp::model::Role::User => values.push(json!({
                "role": "user",
                "content": [{"type": "text", "text": text.trim(), "content": text.trim()}],
            })),
            rmcp::model::Role::Assistant => values.push(json!({
                "role": "assistant",
                "content": text.trim(),
            })),
        }
    }

    values
}

pub(crate) fn gemma4_system_prompt(model_name: &str, system: &str) -> Option<String> {
    if should_use_tiny_system_prompt(model_name) {
        return Some(load_tiny_model_prompt());
    }

    let filtered = filter_extensions_from_system_prompt(system);
    let system = filtered.trim();
    if system.is_empty() {
        None
    } else {
        Some(system.to_string())
    }
}

pub(crate) fn should_use_tiny_system_prompt(model_name: &str) -> bool {
    estimate_model_size_billions(model_name).is_some_and(|size| size <= 4.0)
}

pub(crate) fn estimate_model_size_billions(model_name: &str) -> Option<f32> {
    let normalized = model_name.to_ascii_lowercase().replace('-', "_");
    for part in normalized.split('_') {
        if let Some(value) = part.strip_suffix('b') {
            if let Ok(size) = value.parse::<f32>() {
                return Some(size);
            }
        }
        if let Some(value) = part
            .strip_prefix('e')
            .and_then(|value| value.strip_suffix('b'))
        {
            if let Ok(size) = value.parse::<f32>() {
                return Some(size);
            }
        }
    }
    None
}

pub(crate) fn openai_messages(system: &str, messages: &[Message]) -> Vec<serde_json::Value> {
    let mut values = vec![serde_json::json!({
        "role": "system",
        "content": system,
    })];
    values.extend(openai::format_messages(messages, &ImageFormat::OpenAi));
    values
}

pub(crate) fn chat_conversations(system: &str, messages: &[Message]) -> Vec<Conversation<Role, String>> {
    let mut conversations = Vec::new();
    if !system.trim().is_empty() {
        conversations.push(Conversation {
            role: Role::System,
            content: system.trim().to_string(),
        });
    }
    for message in messages.iter().filter(|message| message.is_agent_visible()) {
        let role = match message.role {
            rmcp::model::Role::User => Role::User,
            rmcp::model::Role::Assistant => Role::Assistant,
        };
        let text = extract_text_content(message);
        if !text.trim().is_empty() {
            conversations.push(Conversation {
                role,
                content: text.trim().to_string(),
            });
        }
    }
    conversations
}

pub(crate) fn render_prompt(system: &str, messages: &[Message]) -> String {
    let mut prompt = String::new();
    if !system.trim().is_empty() {
        prompt.push_str("System: ");
        prompt.push_str(system.trim());
        prompt.push('\n');
    }
    for message in messages.iter().filter(|message| message.is_agent_visible()) {
        let role = match message.role {
            rmcp::model::Role::User => "User",
            rmcp::model::Role::Assistant => "Assistant",
        };
        let text = extract_text_content(message);
        if !text.trim().is_empty() {
            prompt.push_str(role);
            prompt.push_str(": ");
            prompt.push_str(text.trim());
            prompt.push('\n');
        }
    }
    prompt.push_str("Assistant: ");
    prompt
}
