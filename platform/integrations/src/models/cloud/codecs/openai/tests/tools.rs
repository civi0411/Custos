use super::super::*;
use rmcp::model::Tool;
use rmcp::object;
use serde_json::json;

#[test]
fn test_validate_tool_schemas() {
    // Test case 1: Empty parameters object
    // Input JSON with an incomplete parameters object
    let mut actual = vec![json!({
        "type": "function",
        "function": {
            "name": "test_func",
            "description": "test description",
            "parameters": {
                "type": "object"
            }
        }
    })];

    // Run the function to validate and update schemas
    validate_tool_schemas(&mut actual);

    // Expected JSON after validation
    let expected = vec![json!({
        "type": "function",
        "function": {
            "name": "test_func",
            "description": "test description",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }
    })];

    // Compare entire JSON structures instead of individual fields
    assert_eq!(actual, expected);

    // Test case 2: Missing type field
    let mut tools = vec![json!({
        "type": "function",
        "function": {
            "name": "test_func",
            "description": "test description",
            "parameters": {
                "properties": {}
            }
        }
    })];

    validate_tool_schemas(&mut tools);

    let params = tools[0]["function"]["parameters"].as_object().unwrap();
    assert_eq!(params["type"], "object");

    // Test case 3: Complete valid schema should remain unchanged
    let original_schema = json!({
        "type": "function",
        "function": {
            "name": "test_func",
            "description": "test description",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City and country"
                    }
                },
                "required": ["location"]
            }
        }
    });

    let mut tools = vec![original_schema.clone()];
    validate_tool_schemas(&mut tools);
    assert_eq!(tools[0], original_schema);

    // Test case 4: anyOf nullable is unwrapped, preserving sibling metadata
    let mut tools = vec![json!({
        "type": "function",
        "function": {
            "name": "shell",
            "description": "run shell",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout_secs": {
                        "description": "timeout in seconds",
                        "anyOf": [
                            { "type": "integer", "format": "uint64", "minimum": 0 },
                            { "type": "null" }
                        ]
                    }
                },
                "required": ["command"]
            }
        }
    })];
    validate_tool_schemas(&mut tools);
    let timeout_schema = &tools[0]["function"]["parameters"]["properties"]["timeout_secs"];
    assert_eq!(timeout_schema["type"], "integer");
    assert_eq!(timeout_schema["format"], "uint64");
    assert_eq!(timeout_schema["description"], "timeout in seconds");
    assert!(timeout_schema.get("anyOf").is_none());

    // Test case 4b: type array form (schemars 1.x style for nullable primitives)
    let mut tools = vec![json!({
        "type": "function",
        "function": {
            "name": "shell",
            "description": "run shell",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout_secs": {
                        "type": ["integer", "null"],
                        "format": "uint64",
                        "minimum": 0
                    }
                },
                "required": ["command"]
            }
        }
    })];
    validate_tool_schemas(&mut tools);
    let timeout_schema = &tools[0]["function"]["parameters"]["properties"]["timeout_secs"];
    assert_eq!(timeout_schema["type"], "integer");
    assert!(!timeout_schema["type"].is_array());
}

#[test]
fn test_validate_tool_schemas_sanitizes_defs() {
    let mut tools = vec![json!({
        "type": "function",
        "function": {
            "name": "cache",
            "description": "manage cache",
            "parameters": {
                "type": "object",
                "$defs": {
                    "CacheCommand": {
                        "oneOf": [
                            { "description": "List cached files", "type": "string", "const": "list" },
                            { "description": "Clear cached files", "type": "string", "const": "clear" }
                        ]
                    },
                    "TextStyle": {
                        "type": "object",
                        "properties": {
                            "size": { "type": ["integer", "null"], "format": "int32" }
                        }
                    }
                },
                "properties": {
                    "command": { "$ref": "#/$defs/CacheCommand" },
                    "style": { "$ref": "#/$defs/TextStyle" }
                },
                "required": ["command"]
            }
        }
    })];
    validate_tool_schemas(&mut tools);
    let defs = &tools[0]["function"]["parameters"]["$defs"];
    assert!(defs["CacheCommand"].get("oneOf").is_none());
    assert_eq!(defs["CacheCommand"]["anyOf"].as_array().unwrap().len(), 2);
    assert_eq!(defs["TextStyle"]["properties"]["size"]["type"], "integer");
}

#[test]
fn test_format_tools() -> anyhow::Result<()> {
    let tool = Tool::new(
        "test_tool",
        "A test tool",
        object!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Test parameter"
                }
            },
            "required": ["input"]
        }),
    );

    let spec = format_tools(&[tool])?;

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["type"], "function");
    assert_eq!(spec[0]["function"]["name"], "test_tool");
    assert_eq!(
        spec[0]["function"]["strict"],
        json!(false),
        "Some chat-completions upstreams default strict to true when the flag is absent, but MCP tool schemas are not strict-compatible; must explicitly set strict: false"
    );
    Ok(())
}

#[test]
fn test_format_tools_duplicate() -> anyhow::Result<()> {
    let tool1 = Tool::new(
        "test_tool",
        "Test tool",
        object!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Test parameter"
                }
            },
            "required": ["input"]
        }),
    );

    let tool2 = Tool::new(
        "test_tool",
        "Test tool",
        object!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Test parameter"
                }
            },
            "required": ["input"]
        }),
    );

    let result = format_tools(&[tool1, tool2]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Duplicate tool name"));

    Ok(())
}

#[test]
fn test_format_tools_empty() -> anyhow::Result<()> {
    let spec = format_tools(&[])?;
    assert!(spec.is_empty());
    Ok(())
}

