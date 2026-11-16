use crate::conversation::message::Message;
use crate::cloud::codecs::openrouter as openrouter_format;
use serde_json::Value;
use std::collections::HashSet;
use std::ops::Range;

const GEMINI_SCHEMA_REF_KEY: &str = "$ref";
const GEMINI_SAFE_SCHEMA_REF_KEY_BASE: &str = "dollar_ref";

pub(crate) fn is_gemini_model(model_name: &str) -> bool {
    model_name.starts_with("google/gemini")
}

/// Spans of the literal `$ref` token inside opaque tool text.
///
/// Tool results are not required to parse as JSON. Google rejects the token in
/// single-quoted Python `repr` output, YAML, and unquoted text just as it does
/// in strict JSON, so matching only well-formed JSON key positions would miss
/// the reproduction in #11260. A trailing identifier character means the token
/// is part of a longer name such as `$refs` or `$refresh_token`, which must be
/// left alone.
pub(crate) fn scan_schema_ref_tokens(content: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut cursor = 0;

    while cursor < content.len() {
        if !content.is_char_boundary(cursor) {
            cursor += 1;
            continue;
        }

        match match_schema_ref_at(content, cursor) {
            Some(end)
                if !starts_with_escaped_backslash(content, cursor)
                    && !continues_identifier(content, end) =>
            {
                spans.push(cursor..end);
                cursor = end;
            }
            _ => {
                cursor += content
                    .get(cursor..)
                    .and_then(|rest| rest.chars().next())
                    .map_or(1, char::len_utf8);
            }
        }
    }

    spans
}

/// Decodes one character: either a literal one or a `\uXXXX` escape. JSON lets
/// any character of a key be escaped, so `$\u0072ef` and `\u0024\u0072\u0065\u0066`
/// both decode to `$ref` and would otherwise reach Gemini unrewritten.
pub(crate) fn decode_unit(content: &str, index: usize) -> Option<(char, usize)> {
    let rest = content.get(index..)?;

    if let Some(after_prefix) = rest.strip_prefix("\\u") {
        let character = after_prefix
            .get(..4)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32)?;
        return Some((character, index + "\\u".len() + 4));
    }

    let character = rest.chars().next()?;
    Some((character, index + character.len_utf8()))
}

/// Returns the end offset when the text at `start` decodes to `$ref`.
pub(crate) fn match_schema_ref_at(content: &str, start: usize) -> Option<usize> {
    let mut cursor = start;

    for expected in GEMINI_SCHEMA_REF_KEY.chars() {
        let (character, next) = decode_unit(content, cursor)?;
        if character != expected {
            return None;
        }
        cursor = next;
    }

    Some(cursor)
}

/// An odd number of backslashes before `\u0024ref` means the leading backslash
/// is itself escaped, so the text is the literal characters `\u0024ref` rather
/// than an encoded `$ref`. Rewriting it would emit invalid JSON.
pub(crate) fn starts_with_escaped_backslash(content: &str, start: usize) -> bool {
    if !content
        .get(start..)
        .is_some_and(|token| token.starts_with('\\'))
    {
        return false;
    }

    let preceding_backslashes = content
        .get(..start)
        .map(|prefix| prefix.chars().rev().take_while(|c| *c == '\\').count())
        .unwrap_or(0);

    preceding_backslashes % 2 == 1
}

/// A trailing identifier character means the token is part of a longer name
/// such as `$refs` or `$refresh_token`, which must be left alone.
pub(crate) fn continues_identifier(content: &str, end: usize) -> bool {
    decode_unit(content, end)
        .is_some_and(|(character, _)| character.is_ascii_alphanumeric() || character == '_')
}

/// Decodes `\uXXXX` escapes so a candidate key spelled as an escape sequence
/// still counts as occupied.
pub(crate) fn decoded_view(content: &str) -> String {
    let mut decoded = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(offset) = rest.find("\\u") {
        let (before, from_escape) = rest.split_at(offset);
        decoded.push_str(before);

        let after_prefix = from_escape.get("\\u".len()..).unwrap_or_default();
        match after_prefix
            .get(..4)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32)
        {
            Some(character) => {
                decoded.push(character);
                rest = after_prefix.get(4..).unwrap_or_default();
            }
            None => {
                decoded.push_str("\\u");
                rest = after_prefix;
            }
        }
    }
    decoded.push_str(rest);

    decoded
}

pub(crate) fn replace_spans(content: &str, spans: &[Range<usize>], replacement: &str) -> String {
    let mut rewritten = String::with_capacity(content.len());
    let mut copied_through = 0;

    for span in spans {
        if let Some(preceding) = content.get(copied_through..span.start) {
            rewritten.push_str(preceding);
        }
        rewritten.push_str(replacement);
        copied_through = span.end;
    }
    if let Some(trailing) = content.get(copied_through..) {
        rewritten.push_str(trailing);
    }

    rewritten
}

/// The replacement must not already occur anywhere in the tool text, otherwise
/// the rewrite would be ambiguous to the model reading the compatibility note.
///
/// Tool output is externally controlled, so occupied candidates are collected in
/// a single pass rather than rescanning every result for each suffix.
pub(crate) fn collision_free_gemini_schema_ref_key(contents: &[&str]) -> String {
    let mut occupied = HashSet::new();
    for content in contents {
        collect_safe_key_candidates(content, &mut occupied);
        collect_safe_key_candidates(&decoded_view(content), &mut occupied);
    }

    (1..)
        .map(|suffix| {
            if suffix == 1 {
                GEMINI_SAFE_SCHEMA_REF_KEY_BASE.to_string()
            } else {
                format!("{GEMINI_SAFE_SCHEMA_REF_KEY_BASE}_{suffix}")
            }
        })
        .find(|candidate| !occupied.contains(candidate))
        .expect("an unbounded suffix sequence always yields an unused candidate")
}

/// Records every `dollar_ref`/`dollar_ref_N` occurrence in one scan.
pub(crate) fn collect_safe_key_candidates(content: &str, occupied: &mut HashSet<String>) {
    let mut cursor = 0;

    while let Some(offset) = content
        .get(cursor..)
        .and_then(|tail| tail.find(GEMINI_SAFE_SCHEMA_REF_KEY_BASE))
    {
        let start = cursor + offset;
        let end = start + GEMINI_SAFE_SCHEMA_REF_KEY_BASE.len();
        let suffix_len = content
            .get(end..)
            .map(|rest| {
                let digits = rest
                    .strip_prefix('_')
                    .map(|after| after.chars().take_while(char::is_ascii_digit).count())
                    .unwrap_or(0);
                if digits == 0 {
                    0
                } else {
                    1 + digits
                }
            })
            .unwrap_or(0);

        if let Some(token) = content.get(start..end + suffix_len) {
            occupied.insert(token.to_string());
        }
        cursor = end;
    }
}

pub(crate) fn gemini_schema_ref_note(safe_key: &str) -> String {
    format!(
        "[OpenRouter/Gemini compatibility: interpret `{safe_key}` as the JSON Schema key formed by `$` followed by `ref`.]\n"
    )
}

pub(crate) fn apply_gemini_compatibility(model_name: &str, payload: &mut Value, messages: &[Message]) {
    if is_gemini_model(model_name) {
        escape_gemini_schema_ref_keys_in_tool_responses(payload);
        openrouter_format::add_reasoning_details_to_request(payload, messages);
    }
}

/// OpenRouter translates OpenAI `role: tool` messages into Gemini
/// `function_response` parts. Gemini rejects a response containing a literal
/// JSON Schema `$ref` key, treating its value as a function-response part name
/// instead of arbitrary tool text, and Goose replays persisted history, so one
/// such tool result breaks every later turn in the session.
///
/// Rewrite the token and prepend a note so the model can reconstruct the
/// original text. All bytes outside matching token spans remain unchanged.
pub(crate) fn escape_gemini_schema_ref_keys_in_tool_responses(payload: &mut Value) -> usize {
    let Some(messages) = payload.get_mut("messages").and_then(Value::as_array_mut) else {
        return 0;
    };

    let mut tool_contents = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let Some(content_text) = message.get("content").and_then(Value::as_str) else {
            continue;
        };
        tool_contents.push((message_index, content_text.to_string()));
    }

    let scanned: Vec<&str> = tool_contents
        .iter()
        .map(|(_, content)| content.as_str())
        .collect();
    let safe_key = collision_free_gemini_schema_ref_key(&scanned);
    let note = gemini_schema_ref_note(&safe_key);

    let mut escaped = 0;
    for (message_index, content_text) in &tool_contents {
        let spans = scan_schema_ref_tokens(content_text);
        if spans.is_empty() {
            continue;
        }

        let sanitized = replace_spans(content_text, &spans, &safe_key);
        messages[*message_index]["content"] = Value::String(format!("{note}{sanitized}"));
        escaped += spans.len();
    }

    escaped
}
