use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub type ToolCallData = HashMap<
    i32,
    (
        String,
        String,
        String,
        Option<serde_json::Map<String, Value>>,
    ),
>;

pub fn deserialize_null_default_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

pub fn describe_json_value(value: &Value) -> &'static str {
    match value {
        Value::Array(_) => "an array",
        Value::String(_) => "a string",
        Value::Number(_) => "a number",
        Value::Bool(_) => "a boolean",
        Value::Null => "null",
        Value::Object(_) => "an object",
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct DeltaToolCallFunction {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub(crate) arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DeltaToolCall {
    pub(crate) id: Option<String>,
    pub(crate) function: DeltaToolCallFunction,
    pub(crate) index: Option<i32>,
    pub(crate) r#type: Option<String>,
    #[serde(flatten)]
    pub(crate) extra: Option<serde_json::Map<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum DeltaContentBlock {
    String(String),
    Array(Vec<ContentBlockPart>),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ContentBlockPart {
    pub(crate) r#type: String,
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(rename = "thoughtSignature")]
    pub(crate) thought_signature: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct Delta {
    #[serde(default)]
    pub(crate) content: Option<DeltaContentBlock>,
    pub(crate) role: Option<String>,
    pub(crate) tool_calls: Option<Vec<DeltaToolCall>>,
    pub(crate) reasoning_details: Option<Vec<Value>>,
    pub(crate) reasoning: Option<String>,
    pub(crate) reasoning_content: Option<String>,
}

impl Delta {
    /// Prefer `reasoning_content` (DeepSeek/OpenRouter) over `reasoning`
    /// (vLLM); some servers (gpt-oss via vLLM) emit both. Skip empty values.
    pub(crate) fn reasoning_text(&self) -> Option<&str> {
        self.reasoning_content
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.reasoning.as_deref().filter(|s| !s.is_empty()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct StreamingChoice {
    #[serde(default)]
    pub(crate) delta: Delta,
    pub(crate) index: Option<i32>,
    #[serde(default, deserialize_with = "empty_finish_reason_as_none")]
    pub(crate) finish_reason: Option<String>,
}

pub(crate) fn empty_finish_reason_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.filter(|reason| !reason.is_empty()))
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct StreamingChunk {
    pub(crate) choices: Vec<StreamingChoice>,
    created: Option<i64>,
    pub(crate) id: Option<String>,
    pub(crate) usage: Option<Value>,
    pub(crate) model: Option<String>,
}

pub(crate) fn extract_content_and_signature(
    delta_content: Option<&DeltaContentBlock>,
) -> (Option<String>, Option<String>) {
    match delta_content {
        Some(DeltaContentBlock::String(s)) => (Some(s.clone()), None),
        Some(DeltaContentBlock::Array(parts)) => {
            let text_parts: Vec<_> = parts.iter().filter(|p| p.r#type == "text").collect();

            let text = text_parts
                .iter()
                .filter_map(|p| p.text.as_deref())
                .collect::<String>();

            let signature = text_parts
                .iter()
                .find_map(|p| p.thought_signature.as_ref())
                .cloned();

            let text = if text.is_empty() { None } else { Some(text) };

            (text, signature)
        }
        None => (None, None),
    }
}

