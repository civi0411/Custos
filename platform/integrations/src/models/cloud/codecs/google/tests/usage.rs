use super::super::*;
use serde_json::json;

    #[test]
    fn test_get_usage() {
        let data = json!({
            "usageMetadata": {
                "promptTokenCount": 1,
                "candidatesTokenCount": 2,
                "totalTokenCount": 3
            }
        });
        let usage = get_usage(&data).unwrap();
        assert_eq!(usage.input_tokens, Some(1));
        assert_eq!(usage.output_tokens, Some(2));
        assert_eq!(usage.total_tokens, Some(3));
        assert_eq!(usage.cache_read_input_tokens, None);
        assert_eq!(usage.cache_write_input_tokens, None);
    }

    #[test]
    fn test_get_usage_with_cached_content() {
        let data = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 20,
                "totalTokenCount": 120,
                "cachedContentTokenCount": 80
            }
        });
        let usage = get_usage(&data).unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(120));
        assert_eq!(usage.cache_read_input_tokens, Some(80));
        assert_eq!(usage.cache_write_input_tokens, None);
    }

    #[test]
    fn test_get_usage_includes_thinking_tokens() {
        // Gemini thinking models (2.5/3, the goose defaults) report reasoning
        // tokens separately in `thoughtsTokenCount`, and per the API spec
        // `totalTokenCount` = prompt + thoughts + candidates. `output_tokens`
        // must include thoughts so the record reconciles (input + output ==
        // total) and cost, which Google bills at the output rate, is correct --
        // matching the OpenAI (completion_tokens includes reasoning) and
        // Anthropic (output_tokens includes thinking) adapters.
        let data = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "thoughtsTokenCount": 200,
                "totalTokenCount": 350
            }
        });
        let usage = get_usage(&data).unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(250));
        assert_eq!(usage.total_tokens, Some(350));
        assert_eq!(
            usage.input_tokens.unwrap() + usage.output_tokens.unwrap(),
            usage.total_tokens.unwrap(),
        );
    }
