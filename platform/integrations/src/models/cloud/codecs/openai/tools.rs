use anyhow::anyhow;
use rmcp::model::Tool;
use serde_json::{json, Value};

pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
                "strict": false,
            }
        }));
    }

    Ok(result)
}


pub fn validate_tool_schemas(tools: &mut [Value]) {
    for tool in tools.iter_mut() {
        if let Some(function) = tool.get_mut("function") {
            if let Some(parameters) = function.get_mut("parameters") {
                if parameters.is_object() {
                    ensure_valid_json_schema(parameters);
                }
            }
        }
    }
}

/// Ensures that the given JSON value follows the expected JSON Schema structure.
fn ensure_valid_json_schema(schema: &mut Value) {
    if let Some(params_obj) = schema.as_object_mut() {
        if !params_obj.contains_key("type") {
            params_obj.insert("type".to_string(), json!("object"));
        }
    }
    sanitize_schema_node(schema);
}

fn sanitize_schema_node(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        // Moonshot's walle validator rejects `oneOf` behind a `$ref` as
        // "infinite recursion" because its termination check only traverses
        // `anyOf`. The two are interchangeable for tool-argument schemas, so
        // emit the more widely supported form.
        if !obj.contains_key("anyOf") {
            if let Some(one_of) = obj.remove("oneOf") {
                obj.insert("anyOf".to_string(), one_of);
            }
        }
    }

    normalize_nullable(node);

    let Some(obj) = node.as_object_mut() else {
        return;
    };

    if obj.get("type").and_then(|t| t.as_str()) == Some("object") {
        obj.entry("properties").or_insert_with(|| json!({}));
        obj.entry("required").or_insert_with(|| json!([]));
    }

    for key in ["properties", "$defs", "definitions"] {
        if let Some(children) = obj.get_mut(key).and_then(Value::as_object_mut) {
            for child in children.values_mut() {
                sanitize_schema_node(child);
            }
        }
    }
    for key in ["anyOf", "allOf", "prefixItems"] {
        if let Some(children) = obj.get_mut(key).and_then(Value::as_array_mut) {
            for child in children.iter_mut() {
                sanitize_schema_node(child);
            }
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(child) = obj.get_mut(key) {
            if child.is_object() {
                sanitize_schema_node(child);
            }
        }
    }
}

/// Normalizes nullable type representations that some providers (e.g. Vertex Gemini via Bifrost)
/// don't support:
/// - `"type": ["integer", "null"]` → `"type": "integer"` (drops the null variant)
/// - `"anyOf": [T, {"type": "null"}]` → T (unwraps to the non-null schema)
///
/// Optional-ness is already conveyed by the field being absent from `required`.
fn normalize_nullable(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };

    // Handle type: ["T", "null"] array form (schemars 1.x style for nullable primitives)
    if let Some(type_val) = obj.get("type").cloned() {
        if let Some(types) = type_val.as_array() {
            let non_null: Vec<&Value> = types
                .iter()
                .filter(|t| t.as_str() != Some("null"))
                .collect();
            if non_null.len() == 1 {
                let scalar = non_null[0].clone();
                obj.insert("type".to_string(), scalar);
                return;
            }
        }
    }

    // Handle anyOf: [T, {type: "null"}] form — merge the non-null variant's fields
    // into the current object (preserving sibling keys like "description" or "default")
    // rather than replacing the whole schema.
    if let Some(any_of) = obj.remove("anyOf") {
        if let Some(variants) = any_of.as_array() {
            if variants.len() == 2 {
                let is_null = |v: &Value| v.get("type").and_then(|t| t.as_str()) == Some("null");
                let non_null = if is_null(&variants[0]) {
                    Some(&variants[1])
                } else if is_null(&variants[1]) {
                    Some(&variants[0])
                } else {
                    None
                };
                if let Some(replacement) = non_null {
                    if let Some(replacement_obj) = replacement.as_object() {
                        for (k, v) in replacement_obj {
                            obj.entry(k.clone()).or_insert(v.clone());
                        }
                        return;
                    }
                }
            }
        }
        // Put it back if we couldn't simplify
        obj.insert("anyOf".to_string(), any_of);
    }
}

