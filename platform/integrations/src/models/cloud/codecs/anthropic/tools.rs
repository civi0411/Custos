use rmcp::model::{JsonObject, Tool};
use rmcp::object as json_object;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use super::options::*;

fn anthropic_flavored_input_schema(input_schema: Arc<JsonObject>) -> Arc<JsonObject> {
    if input_schema.is_empty() {
        return Arc::new(json_object!({
            "type": "object",
        }));
    }
    input_schema
}

/// Convert internal Tool format to Anthropic's API tool specification
pub fn format_tools(tools: &[Tool], options: &AnthropicFormatOptions) -> Vec<Value> {
    let mut unique_tools = HashSet::new();
    let mut tool_specs = Vec::new();

    for tool in tools {
        if unique_tools.insert(tool.name.clone()) {
            tool_specs.push(json!({
                NAME_FIELD: tool.name,
                "description": tool.description,
                "input_schema": anthropic_flavored_input_schema(tool.input_schema.clone())
            }));
        }
    }

    if options.prompt_cache_disabled {
        return tool_specs;
    }

    // Add "cache_control" to the last tool spec, if any. This means that all tool definitions,
    // will be cached as a single prefix.
    if let Some(last_tool) = tool_specs.last_mut() {
        last_tool
            .as_object_mut()
            .unwrap()
            .insert(CACHE_CONTROL_FIELD.to_string(), options.cache_control());
    }

    tool_specs
}

/// Convert system message to Anthropic's API system specification
pub fn format_system(system: &str, options: &AnthropicFormatOptions) -> Value {
    if options.prompt_cache_disabled {
        return json!([{
            TYPE_FIELD: TEXT_TYPE,
            TEXT_TYPE: system
        }]);
    }
    json!([{
        TYPE_FIELD: TEXT_TYPE,
        TEXT_TYPE: system,
        CACHE_CONTROL_FIELD: options.cache_control()
    }])
}
