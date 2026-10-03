//! Secret Redaction for Context Compilation
//!
//! Masks sensitive credentials (API keys, tokens, high-entropy secrets)
//! to prevent accidental leakage to model inference endpoints (Custos.md §6.4 Step 7).

/// Redacts secrets and high-entropy sensitive tokens from text content.
pub struct SecretRedactor;

impl SecretRedactor {
    /// Redacts known credential patterns and high-entropy strings from the given text.
    pub fn redact(text: &str) -> String {
        let mut result = text.to_string();

        // 1. Common API key prefixes
        result = Self::redact_prefixed(&result, "sk-", 20);
        result = Self::redact_prefixed(&result, "ghp_", 20);
        result = Self::redact_prefixed(&result, "github_pat_", 20);
        result = Self::redact_prefixed(&result, "xoxb-", 20);
        result = Self::redact_prefixed(&result, "xoxp-", 20);

        // 2. High entropy token detection on long contiguous words
        let mut sanitized_words = Vec::new();
        for word in result.split_whitespace() {
            if word.len() >= 24 && Self::is_high_entropy_token(word) {
                sanitized_words.push("[REDACTED_HIGH_ENTROPY]".to_string());
            } else {
                sanitized_words.push(word.to_string());
            }
        }

        // Preserve line structure if word-level replacement happened on high entropy
        let word_replaced = sanitized_words.join(" ");
        if word_replaced.contains("[REDACTED_HIGH_ENTROPY]") {
            // Apply word-level masking back into result
            result = Self::mask_high_entropy_tokens(&result);
        }

        result
    }

    fn redact_prefixed(text: &str, prefix: &str, min_len: usize) -> String {
        let mut output = String::new();
        let mut cursor = 0;

        while let Some(start_idx) = text[cursor..].find(prefix) {
            let actual_start = cursor + start_idx;
            output.push_str(&text[cursor..actual_start]);

            // Scan end of alphanumeric token
            let remainder = &text[actual_start..];
            let token_end = remainder
                .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(remainder.len());

            let token = &remainder[..token_end];
            if token.len() >= min_len {
                output.push_str("[REDACTED_API_KEY]");
            } else {
                output.push_str(token);
            }
            cursor = actual_start + token_end;
        }

        output.push_str(&text[cursor..]);
        output
    }

    fn mask_high_entropy_tokens(text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut in_token = false;
        let mut token_start = 0;

        for (idx, ch) in text.char_indices() {
            let is_token_char = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
            if is_token_char && !in_token {
                in_token = true;
                token_start = idx;
            } else if !is_token_char && in_token {
                in_token = false;
                let token = &text[token_start..idx];
                if token.len() >= 24 && Self::is_high_entropy_token(token) {
                    output.push_str("[REDACTED_HIGH_ENTROPY]");
                } else {
                    output.push_str(token);
                }
                output.push(ch);
            } else if !in_token {
                output.push(ch);
            }
        }

        if in_token {
            let token = &text[token_start..];
            if token.len() >= 24 && Self::is_high_entropy_token(token) {
                output.push_str("[REDACTED_HIGH_ENTROPY]");
            } else {
                output.push_str(token);
            }
        }

        output
    }

    /// Computes Shannon entropy of an ASCII token string
    pub fn shannon_entropy(token: &str) -> f32 {
        if token.is_empty() {
            return 0.0;
        }
        let mut counts = [0u32; 256];
        for &byte in token.as_bytes() {
            counts[byte as usize] += 1;
        }

        let len = token.len() as f32;
        let mut entropy = 0.0f32;
        for &count in &counts {
            if count > 0 {
                let p = count as f32 / len;
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    pub fn is_high_entropy_token(token: &str) -> bool {
        let has_digit = token.chars().any(|c| c.is_ascii_digit());
        let has_alpha = token.chars().any(|c| c.is_ascii_alphabetic());
        let entropy = Self::shannon_entropy(token);

        // Hex tokens (0-9, a-f) have theoretical max entropy 4.0 (log2(16))
        let is_hex = token.chars().all(|c| c.is_ascii_hexdigit());
        if is_hex && token.len() >= 32 && entropy > 3.3 {
            return true;
        }

        // Base64 and high-entropy alphanumeric credentials
        has_digit && has_alpha && entropy > 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_sk_keys() {
        let input = "export OPENAI_API_KEY=sk-ant-api03-abcdef1234567890abcdef1234567890\nkeep going";
        let redacted = SecretRedactor::redact(input);
        assert!(!redacted.contains("sk-ant-api03"));
        assert!(redacted.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn test_redact_github_token() {
        let input = "Authorization: token ghp_111122223333444455556666777788889999";
        let redacted = SecretRedactor::redact(input);
        assert!(!redacted.contains("ghp_11112222"));
        assert!(redacted.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn test_redact_high_entropy_random_hex() {
        let input = "token: 8f9a2b4c6e1d3f5a7c9e2b4d6f8a0c2e4b6d";
        let redacted = SecretRedactor::redact(input);
        assert!(redacted.contains("[REDACTED_HIGH_ENTROPY]"));
    }

    #[test]
    fn test_normal_text_not_redacted() {
        let input = "fn calculate_standard_deviation_for_metrics() -> f32 { 0.0 }";
        let redacted = SecretRedactor::redact(input);
        assert_eq!(input, redacted);
    }
}
