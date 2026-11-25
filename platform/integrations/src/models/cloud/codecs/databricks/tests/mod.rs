pub(crate) const OPENAI_TOOL_USE_RESPONSE: &str = r#"{
    "choices": [{
        "role": "assistant",
        "message": {
            "tool_calls": [{
                "id": "1",
                "function": {
                    "name": "example_fn",
                    "arguments": "{\"param\": \"value\"}"
                }
            }]
        }
    }],
    "usage": {
        "input_tokens": 10,
        "output_tokens": 25,
        "total_tokens": 35
    }
}"#;

mod documents;
mod messages;
mod request;
mod response;
mod tools_and_ordering;
