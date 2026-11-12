use anyhow::anyhow;
use rmcp::model::Tool;
use serde_json::{json, Value};

pub fn format_tools(tools: &[Tool], _model_name: &str) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        let has_properties = tool
            .input_schema
            .get("properties")
            .and_then(|v| v.as_object())
            .is_some_and(|p| !p.is_empty());

        // Databricks serving endpoints (including Gemini-backed ones) use the
        // OpenAI-compatible chat format, so tools always use "parameters" — not
        // the Google-native "parametersJsonSchema" field.
        let mut def = json!({
            "name": tool.name,
            "description": tool.description,
        });
        if has_properties {
            def["parameters"] = json!(tool.input_schema);
        }
        let function_def = def;

        result.push(json!({
            "type": "function",
            "function": function_def,
        }));
    }

    Ok(result)
}
