use super::super::*;
use rmcp::model::Tool;
use rmcp::object;
use serde_json::json;

    #[test]
    fn test_tools_to_google_spec_with_valid_tools() {
        let params = object!({
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "A parameter"
                }
            }
        });
        let tools = vec![Tool::new("tool1", "description1", params.clone())];
        let result = format_tools(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "tool1");
        assert_eq!(result[0]["description"], "description1");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert!(result[0].get("parameters").is_none());
        assert_eq!(result[0]["parametersJsonSchema"], json!(params));
    }

    #[test]
    fn test_tools_to_google_spec_with_empty_properties() {
        let tools = vec![Tool::new(
            "tool1".to_string(),
            "description1".to_string(),
            object!({
                "properties": {}
            }),
        )];
        let result = format_tools(&tools);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "tool1");
        assert_eq!(result[0]["description"], "description1");
        assert!(result[0].get("parametersJsonSchema").is_none());
    }

    #[test]

    fn test_tools_uses_parameters_json_schema() {
        let params = object!({
            "properties": {
                "field": {
                    "type": ["string", "null"],
                    "description": "A field"
                }
            }
        });
        let tools = vec![Tool::new("test_tool", "test description", params.clone())];
        let result = format_tools(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test_tool");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert_eq!(result[0]["parametersJsonSchema"], json!(params));
    }

    #[test]
    fn test_format_tools_uses_parameters_json_schema() {
        let tool = Tool::new(
            "test_tool",
            "Test tool with $ref",
            object!({
                "type": "object",
                "$defs": {
                    "MyType": { "type": "string", "description": "A custom type" }
                },
                "properties": {
                    "field": { "$ref": "#/$defs/MyType" }
                }
            }),
        );

        let result = format_tools(&[tool]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test_tool");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert!(result[0].get("parameters").is_none());

        let schema = &result[0]["parametersJsonSchema"];
        assert_eq!(schema["properties"]["field"]["$ref"], "#/$defs/MyType");
        assert!(schema.get("$defs").is_some());
    }
