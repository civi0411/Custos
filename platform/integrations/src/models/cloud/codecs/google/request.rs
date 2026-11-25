use super::messages::format_messages;
use super::tools::format_tools;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use anyhow::Result;
use rmcp::model::Tool;
use serde::Serialize;
use serde_json::Value;

pub(crate) const DEFAULT_THINKING_BUDGET: i32 = 8192;

#[derive(Serialize)]
struct TextPart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct SystemInstruction<'a> {
    parts: [TextPart<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolsWrapper {
    function_declarations: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thinking_level: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thinking_budget: Option<i32>,
    pub(crate) include_thoughts: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleRequest<'a> {
    system_instruction: SystemInstruction<'a>,
    contents: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolsWrapper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

pub(crate) fn get_thinking_config(
    model_config: &ModelConfig,
    thinking_budget: Option<i32>,
) -> Option<ThinkingConfig> {
    if model_config.reasoning == Some(false)
        || model_config.thinking_effort() == Some(ThinkingEffort::Off)
    {
        let model_name = model_config.model_name.to_lowercase();
        if model_name.starts_with("gemini-3.5") || model_name.starts_with("gemini-3.6") {
            return Some(ThinkingConfig {
                thinking_level: Some(ThinkingLevel::Minimal),
                thinking_budget: None,
                include_thoughts: false,
            });
        }
        // Gemini 2.5 Flash defaults to dynamic thinking; only an explicit budget
        // of 0 turns it off. Other families can't be disabled, so leave them unset.
        if model_config
            .model_name
            .to_lowercase()
            .starts_with("gemini-2.5-flash")
        {
            return Some(ThinkingConfig {
                thinking_level: None,
                thinking_budget: Some(0),
                include_thoughts: false,
            });
        }
        return None;
    }
    let model_name = model_config.model_name.to_lowercase();
    let is_gemini_3 = model_name.starts_with("gemini-3");
    let is_gemini_25 = model_name.starts_with("gemini-2.5");
    if !is_gemini_3 && !is_gemini_25 {
        return None;
    }

    if is_gemini_3 {
        let effort = model_config
            .thinking_effort()
            .unwrap_or(ThinkingEffort::Off);
        if effort == ThinkingEffort::Off {
            return None;
        }
        let thinking_level = match effort {
            ThinkingEffort::Off | ThinkingEffort::Low => ThinkingLevel::Low,
            ThinkingEffort::Medium if model_name.starts_with("gemini-3-pro") => ThinkingLevel::Low,
            ThinkingEffort::Medium => ThinkingLevel::Medium,
            ThinkingEffort::High | ThinkingEffort::Max => ThinkingLevel::High,
        };

        Some(ThinkingConfig {
            thinking_level: Some(thinking_level),
            thinking_budget: None,
            include_thoughts: true,
        })
    } else {
        let thinking_budget = match model_config
            .request_param::<i32>("thinking_budget")
            .or(thinking_budget)
        {
            Some(budget) if budget >= 0 => budget,
            Some(budget) => {
                tracing::warn!(
                    "Invalid thinking budget '{}' for model '{}'. Must be >= 0. Using '{}'.",
                    budget,
                    model_config.model_name,
                    DEFAULT_THINKING_BUDGET,
                );
                DEFAULT_THINKING_BUDGET
            }
            None => DEFAULT_THINKING_BUDGET,
        };
        Some(ThinkingConfig {
            thinking_level: None,
            thinking_budget: Some(thinking_budget),
            include_thoughts: true,
        })
    }
}

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    create_request_impl(model_config, system, messages, tools, None)
}

pub fn create_request_with_thinking_budget(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    thinking_budget: Option<i32>,
) -> Result<Value> {
    create_request_impl(model_config, system, messages, tools, thinking_budget)
}

fn create_request_impl(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    thinking_budget: Option<i32>,
) -> Result<Value> {
    let tools_wrapper = if tools.is_empty() {
        None
    } else {
        Some(ToolsWrapper {
            function_declarations: format_tools(tools),
        })
    };

    let thinking_config = get_thinking_config(model_config, thinking_budget);
    let temperature = (!model_config
        .model_name
        .to_lowercase()
        .starts_with("gemini-3"))
    .then(|| model_config.temperature.map(|t| t as f64))
    .flatten();

    let generation_config = Some(GenerationConfig {
        temperature,
        max_output_tokens: Some(model_config.max_output_tokens()),
        thinking_config,
    });

    let request = GoogleRequest {
        system_instruction: SystemInstruction {
            parts: [TextPart { text: system }],
        },
        contents: format_messages(
            messages,
            model_config
                .model_name
                .to_lowercase()
                .starts_with("gemini-3"),
        ),
        tools: tools_wrapper,
        generation_config,
    };

    Ok(serde_json::to_value(request)?)
}
