//! Agent Reply Processing & Argument Coercion
//!
//! Handles extraction of model response parts, JSON schema type coercion for tool arguments,
//! and construction of structured tool requests.

use custos_provider::conversation::message::ToolRequest;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};

/// Coerce a string value based on the target JSON schema type definition
pub fn coerce_value(s: &str, schema: &Value) -> Value {
    let type_str = schema.get("type");

    match type_str {
        Some(Value::String(t)) => match t.as_str() {
            "number" | "integer" => try_coerce_number(s),
            "boolean" => try_coerce_boolean(s),
            _ => Value::String(s.to_string()),
        },
        Some(Value::Array(types)) => {
            for t in types {
                if let Value::String(type_name) = t {
                    match type_name.as_str() {
                        "number" | "integer" if s.parse::<f64>().is_ok() => {
                            return try_coerce_number(s)
                        }
                        "boolean" if matches!(s.to_lowercase().as_str(), "true" | "false") => {
                            return try_coerce_boolean(s)
                        }
                        _ => continue,
                    }
                }
            }
            Value::String(s.to_string())
        }
        _ => Value::String(s.to_string()),
    }
}

pub fn try_coerce_number(s: &str) -> Value {
    if let Ok(n) = s.parse::<f64>() {
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            json!(n as i64)
        } else {
            json!(n)
        }
    } else {
        Value::String(s.to_string())
    }
}

pub fn try_coerce_boolean(s: &str) -> Value {
    match s.trim().to_lowercase().as_str() {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(s.to_string()),
    }
}

/// Recursively coerce object fields against property schemas
pub fn coerce_object(
    raw_obj: &serde_json::Map<String, Value>,
    schema: &Value,
) -> serde_json::Map<String, Value> {
    let properties = schema.get("properties").and_then(|p| p.as_object());
    let mut coerced = serde_json::Map::new();

    for (k, v) in raw_obj {
        if let Some(prop_schema) = properties.and_then(|p| p.get(k)) {
            match v {
                Value::String(s) => {
                    coerced.insert(k.clone(), coerce_value(s, prop_schema));
                }
                Value::Object(nested) => {
                    coerced.insert(k.clone(), Value::Object(coerce_object(nested, prop_schema)));
                }
                _ => {
                    coerced.insert(k.clone(), v.clone());
                }
            }
        } else {
            coerced.insert(k.clone(), v.clone());
        }
    }

    coerced
}

/// Build a structured ToolRequest message from tool call parameters
pub fn build_tool_request(
    id: impl Into<String>,
    tool_name: impl Into<String>,
    arguments: Value,
) -> ToolRequest {
    let mut params = CallToolRequestParams::new(tool_name.into());
    if let Value::Object(map) = arguments {
        params = params.with_arguments(map);
    }

    ToolRequest {
        id: id.into(),
        tool_call: Ok(params),
        metadata: None,
        tool_meta: None,
    }
}

/// Accumulate text fragments from streaming tokens
pub fn accumulate_text<'a>(chunks: impl IntoIterator<Item = &'a str>) -> String {
    let mut buffer = String::new();
    for chunk in chunks {
        buffer.push_str(chunk);
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coerce_number() {
        let schema = json!({"type": "integer"});
        assert_eq!(coerce_value("42", &schema), json!(42));
        assert_eq!(coerce_value("-100", &schema), json!(-100));

        let float_schema = json!({"type": "number"});
        assert_eq!(coerce_value("2.5", &float_schema), json!(2.5));
    }

    #[test]
    fn test_coerce_boolean() {
        let schema = json!({"type": "boolean"});
        assert_eq!(coerce_value("True", &schema), json!(true));
        assert_eq!(coerce_value("FALSE", &schema), json!(false));
        assert_eq!(coerce_value("maybe", &schema), json!("maybe"));
    }

    #[test]
    fn test_coerce_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"},
                "active": {"type": "boolean"},
                "label": {"type": "string"}
            }
        });

        let mut raw = serde_json::Map::new();
        raw.insert("count".into(), json!("15"));
        raw.insert("active".into(), json!("true"));
        raw.insert("label".into(), json!("hello"));

        let coerced = coerce_object(&raw, &schema);
        assert_eq!(coerced.get("count"), Some(&json!(15)));
        assert_eq!(coerced.get("active"), Some(&json!(true)));
        assert_eq!(coerced.get("label"), Some(&json!("hello")));
    }

    #[test]
    fn test_build_tool_request() {
        let req = build_tool_request("call_1", "read_file", json!({"path": "src/lib.rs"}));
        assert_eq!(req.id, "call_1");
        assert!(req.tool_call.is_ok());
        let call = req.tool_call.unwrap();
        assert_eq!(call.name, "read_file");
        assert!(call.arguments.is_some());
    }
}
