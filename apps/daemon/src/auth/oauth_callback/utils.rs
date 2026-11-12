pub(crate) fn oauth_provider_endpoint(service_type: &str) -> Option<String> {
    match service_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => Some("https://api.openai.com/v1".to_string()),
        "anthropic" | "claude" => Some("https://api.anthropic.com/v1".to_string()),
        "gemini" | "google" => Some("https://cloudcode-pa.googleapis.com".to_string()),
        "antigravity" => Some("https://cloudcode-pa.googleapis.com".into()),
        "deepseek" => Some("https://api.deepseek.com/v1".to_string()),
        _ => None,
    }
}

pub(crate) fn oauth_provider_label(provider_id: &str, service_type: &str) -> String {
    match service_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => "OpenAI".to_string(),
        "anthropic" | "claude" => "Anthropic".to_string(),
        "gemini" | "google" => "Google Gemini".to_string(),
        "deepseek" => "DeepSeek".to_string(),
        _ => provider_id.to_string(),
    }
}

pub(crate) fn mask_oauth_token(token: &str) -> String {
    if token.trim().is_empty() {
        return "none".to_string();
    }
    "oauth".to_string()
}

pub(crate) fn extract_query_param(path_and_query: &str, param_name: &str) -> Option<String> {
    let query = path_and_query.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.split('=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == param_name {
                let decoded = urlencoding_decode(v);
                return Some(decoded);
            }
        }
    }
    None
}

pub(crate) fn urlencoding_decode(val: &str) -> String {
    let mut result = String::new();
    let bytes = val.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&val[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

