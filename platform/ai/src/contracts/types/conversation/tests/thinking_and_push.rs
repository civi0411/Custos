use crate::conversation::message::{
    InferenceMetadata, Message, MessageContentBlock,
};
use crate::conversation::{fix_conversation, Conversation};
use rmcp::model::{CallToolRequestParams, ContentBlock, Role};
use rmcp::object;

#[test]
fn test_dedupes_duplicate_signed_thinking_around_tool_call() {
    // Reproduces the Anthropic 400 scenario: a standalone signed thinking
    // message immediately followed by an assistant message that repeats the
    // same signed thinking plus a tool_use. After merging consecutive
    // assistant messages these become two adjacent identical thinking blocks.
    let messages = vec![
        Message::user().with_text("Do the thing"),
        Message::assistant().with_thinking("Let me think about this", "sig-1"),
        Message::assistant()
            .with_thinking("Let me think about this", "sig-1")
            .with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("do_thing").with_arguments(object!({"x": 1}))),
            ),
        Message::user().with_tool_response(
            "tool_1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("done"),
            ])),
        ),
        Message::user().with_text("Now continue"),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    assert!(
        issues
            .iter()
            .any(|i| i == "Removed duplicate signed thinking block"),
        "expected dedupe issue, got: {:?}",
        issues
    );

    let fixed_messages = fixed.messages();
    let assistant = fixed_messages
        .iter()
        .find(|m| {
            m.role == Role::Assistant
                && m.content
                    .iter()
                    .any(|c| matches!(c, MessageContentBlock::ToolRequest(_)))
        })
        .expect("assistant tool-call message should exist");

    let thinking_count = assistant
        .content
        .iter()
        .filter(|c| {
            matches!(
                c,
                MessageContentBlock::Thinking(_) | MessageContentBlock::RedactedThinking(_)
            )
        })
        .count();
    assert_eq!(
        thinking_count, 1,
        "duplicate signed thinking should be collapsed to one block"
    );
}

#[test]
fn test_keeps_distinct_signed_thinking_blocks_in_assistant_message() {
    // Distinct signed thinking blocks (different signatures) must be preserved.
    let messages = vec![
        Message::user().with_text("Do the thing"),
        Message::assistant()
            .with_thinking("First thought", "sig-A")
            .with_thinking("Second thought", "sig-B")
            .with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("do_thing").with_arguments(object!({"x": 1}))),
            ),
        Message::user().with_tool_response(
            "tool_1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("done"),
            ])),
        ),
        Message::user().with_text("Now continue"),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    assert!(
        !issues
            .iter()
            .any(|i| i == "Removed duplicate signed thinking block"),
        "should not dedupe distinct thinking blocks, got: {:?}",
        issues
    );

    let fixed_messages = fixed.messages();
    let thinking_count = fixed_messages[1]
        .content
        .iter()
        .filter(|c| {
            matches!(
                c,
                MessageContentBlock::Thinking(_) | MessageContentBlock::RedactedThinking(_)
            )
        })
        .count();
    assert_eq!(thinking_count, 2, "distinct thinking blocks must be kept");
}

#[test]
fn test_keeps_duplicate_unsigned_thinking_blocks() {
    // Unsigned thinking (reasoning summaries from non-Anthropic providers)
    // can legitimately repeat and must not be dropped, since only signed
    // blocks trigger the Anthropic exact-replay 400.
    let messages = vec![
        Message::user().with_text("Do the thing"),
        Message::assistant()
            .with_thinking("same reasoning", "")
            .with_thinking("same reasoning", ""),
        Message::user().with_text("Now continue"),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    assert!(
        !issues
            .iter()
            .any(|i| i == "Removed duplicate signed thinking block"),
        "unsigned thinking must not be deduped, got: {:?}",
        issues
    );

    let fixed_messages = fixed.messages();
    let thinking_count = fixed_messages
        .iter()
        .flat_map(|m| m.content.iter())
        .filter(|c| matches!(c, MessageContentBlock::Thinking(_)))
        .count();
    assert_eq!(
        thinking_count, 2,
        "duplicate unsigned thinking blocks must be kept"
    );
}

#[test]
fn test_dedupes_signed_thinking_across_split_tool_messages() {
    // The agent splits one provider turn with multiple tool calls into one
    // assistant message per call (interleaved with tool results), each
    // carrying the same signed thinking. merge_consecutive_messages cannot
    // merge them because tool results sit between, so the dedupe must span
    // messages and keep the signed block only on the first.
    let messages = vec![
        Message::user().with_text("Use both tools"),
        Message::assistant()
            .with_thinking("multi-tool reasoning", "sig-1")
            .with_tool_request(
                "call_1",
                Ok(CallToolRequestParams::new("tool_a").with_arguments(object!({"p": 1}))),
            ),
        Message::user().with_tool_response(
            "call_1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("ok"),
            ])),
        ),
        Message::assistant()
            .with_thinking("multi-tool reasoning", "sig-1")
            .with_tool_request(
                "call_2",
                Ok(CallToolRequestParams::new("tool_b").with_arguments(object!({"p": 2}))),
            ),
        Message::user().with_tool_response(
            "call_2",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("ok"),
            ])),
        ),
        Message::user().with_text("Now continue"),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    assert!(
        issues
            .iter()
            .any(|i| i == "Removed duplicate signed thinking block"),
        "expected cross-message dedupe issue, got: {:?}",
        issues
    );

    let fixed_messages = fixed.messages();
    let total_thinking = fixed_messages
        .iter()
        .flat_map(|m| m.content.iter())
        .filter(|c| {
            matches!(
                c,
                MessageContentBlock::Thinking(_) | MessageContentBlock::RedactedThinking(_)
            )
        })
        .count();
    assert_eq!(
        total_thinking, 1,
        "the repeated signed block must survive only once across the turn"
    );

    let total_tool_requests = fixed_messages
        .iter()
        .flat_map(|m| m.content.iter())
        .filter(|c| matches!(c, MessageContentBlock::ToolRequest(_)))
        .count();
    assert_eq!(total_tool_requests, 2, "both tool calls must be preserved");

    // The first split message keeps the thinking; the second loses it.
    assert!(
        fixed_messages[1]
            .content
            .iter()
            .any(|c| matches!(c, MessageContentBlock::Thinking(_))),
        "first split message keeps signed thinking"
    );
}

#[test]
fn test_push_coalesces_thinking_deltas() {
    let mut conv = Conversation::empty();
    for fragment in ["I ", "should ", "think ", "about ", "this."] {
        conv.push(
            Message::assistant()
                .with_thinking(fragment, "")
                .with_id("turn-1"),
        );
    }

    assert_eq!(conv.messages().len(), 1);
    let content = &conv.messages()[0].content;
    assert_eq!(content.len(), 1);
    match &content[0] {
        MessageContentBlock::Thinking(t) => {
            assert_eq!(t.thinking, "I should think about this.");
            assert_eq!(t.signature, "");
        }
        other => panic!("expected single Thinking block, got {:?}", other),
    }
}

#[test]
fn test_push_merges_empty_output_token_limit_update_by_id() {
    let mut conv = Conversation::empty();
    conv.push(Message::assistant().with_text("first").with_id("turn-1"));

    let inference = InferenceMetadata {
        provider: "test-provider".to_string(),
        requested_model: "test-model".to_string(),
        resolved_model: None,
        provider_session_id: None,
    };
    let mut limited = Message::assistant()
        .with_id("turn-1")
        .with_inference(inference.clone());
    limited.metadata.output_token_limit_reached = true;
    conv.push(limited);

    assert_eq!(conv.messages().len(), 1);
    assert!(conv.messages()[0].metadata.output_token_limit_reached);
    assert_eq!(conv.messages()[0].metadata.inference, Some(inference));
}

#[test]
fn test_push_retains_unmatched_output_token_limit_update_for_user_only() {
    let mut conv = Conversation::empty();
    let mut limited = Message::assistant().with_id("turn-1");
    limited.metadata.output_token_limit_reached = true;

    conv.push(limited);

    assert_eq!(conv.messages().len(), 1);
    let persisted = &conv.messages()[0];
    assert!(persisted.content.is_empty());
    assert!(persisted.metadata.user_visible);
    assert!(!persisted.metadata.agent_visible);
    assert!(persisted.metadata.output_token_limit_reached);
    assert!(conv.agent_visible_messages().is_empty());
}

#[test]
fn test_push_thinking_adopts_signature_on_closing_delta() {
    let mut conv = Conversation::empty();
    // Streamed shape for one signed block: text deltas accumulate while
    // unsigned; the closing delta carries the signature.
    conv.push(
        Message::assistant()
            .with_thinking("a", "")
            .with_id("turn-1"),
    );
    conv.push(
        Message::assistant()
            .with_thinking("b", "sig1")
            .with_id("turn-1"),
    );

    let content = &conv.messages()[0].content;
    assert_eq!(content.len(), 1);
    match &content[0] {
        MessageContentBlock::Thinking(t) => {
            assert_eq!(t.thinking, "ab");
            assert_eq!(t.signature, "sig1");
        }
        other => panic!("expected Thinking, got {:?}", other),
    }
}

#[test]
fn test_push_unsigned_thinking_after_signed_starts_new_block() {
    let mut conv = Conversation::empty();
    conv.push(
        Message::assistant()
            .with_thinking("first body", "sig1")
            .with_id("turn-1"),
    );
    // A second thinking block begins; in signature-at-end streams the
    // first text arrives before the block's signature, so the new
    // unsigned delta must NOT be appended to the already-signed block —
    // otherwise the closing signature later would replay text under
    // the wrong signature.
    conv.push(
        Message::assistant()
            .with_thinking("second body start", "")
            .with_id("turn-1"),
    );

    let content = &conv.messages()[0].content;
    assert_eq!(
        content.len(),
        2,
        "unsigned delta must not merge into a signed block: {:?}",
        content
    );
    match (&content[0], &content[1]) {
        (MessageContentBlock::Thinking(a), MessageContentBlock::Thinking(b)) => {
            assert_eq!(a.thinking, "first body");
            assert_eq!(a.signature, "sig1");
            assert_eq!(b.thinking, "second body start");
            assert_eq!(b.signature, "");
        }
        other => panic!("unexpected content shape: {:?}", other),
    }
}

#[test]
fn test_push_keeps_distinct_signed_thinking_blocks_separate() {
    let mut conv = Conversation::empty();
    conv.push(
        Message::assistant()
            .with_thinking("block A", "sig-A")
            .with_id("turn-1"),
    );
    conv.push(
        Message::assistant()
            .with_thinking("block B", "sig-B")
            .with_id("turn-1"),
    );

    let content = &conv.messages()[0].content;
    assert_eq!(
        content.len(),
        2,
        "two distinct signed blocks must not coalesce: {:?}",
        content
    );
    match (&content[0], &content[1]) {
        (MessageContentBlock::Thinking(a), MessageContentBlock::Thinking(b)) => {
            assert_eq!(a.thinking, "block A");
            assert_eq!(a.signature, "sig-A");
            assert_eq!(b.thinking, "block B");
            assert_eq!(b.signature, "sig-B");
        }
        other => panic!("unexpected content shape: {:?}", other),
    }
}

#[test]
fn test_push_does_not_coalesce_multi_block_thinking_message() {
    let mut conv = Conversation::empty();
    conv.push(
        Message::assistant()
            .with_thinking("first", "")
            .with_id("turn-1"),
    );

    // Multi-block message must NOT coalesce into the existing thinking
    // block — the merge arm requires `message.content.len() == 1`.
    let mut multi = Message::assistant().with_thinking("second", "");
    multi = multi.with_text("and now text").with_id("turn-1");
    conv.push(multi);

    let content = &conv.messages()[0].content;
    assert_eq!(content.len(), 3);
    match (&content[0], &content[1], &content[2]) {
        (
            MessageContentBlock::Thinking(a),
            MessageContentBlock::Thinking(b),
            MessageContentBlock::Text(c),
        ) => {
            assert_eq!(a.thinking, "first");
            assert_eq!(b.thinking, "second");
            assert_eq!(c.text, "and now text");
        }
        other => panic!("unexpected content shape: {:?}", other),
    }
}
