use super::helpers::run_verify;
use crate::conversation::message::{Message, MessageContentBlock, MessageMetadata};
use crate::conversation::{fix_conversation, Conversation};
use rmcp::model::{CallToolRequestParams, Role};
use rmcp::object;

#[test]
fn test_valid_conversation() {
    use rmcp::model::ContentBlock;

    let all_messages = [
        Message::user().with_text("Can you help me search for something?"),
        Message::assistant()
            .with_text("I'll help you search.")
            .with_tool_request(
                "search_1",
                Ok(CallToolRequestParams::new("web_search")
                    .with_arguments(object!({"query": "rust programming"}))),
            ),
        Message::user().with_tool_response(
            "search_1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("Search results here"),
            ])),
        ),
        Message::assistant().with_text("Based on the search results, here's what I found..."),
    ];

    for i in 1..=all_messages.len() {
        let messages = Conversation::new_unvalidated(all_messages[..i].to_vec());
        if messages.last().unwrap().role == Role::User {
            let (fixed, issues) = fix_conversation(messages.clone());
            assert_eq!(
                fixed.len(),
                messages.len(),
                "Step {}: Length should match",
                i
            );
            assert!(
                issues.is_empty(),
                "Step {}: Should have no issues, but found: {:?}",
                i,
                issues
            );
            assert_eq!(
                fixed.messages(),
                messages.messages(),
                "Step {}: Messages should be unchanged",
                i
            );
        }
    }
}

#[test]
fn test_role_alternation_and_content_placement_issues() {
    use rmcp::model::ContentBlock;

    let messages = vec![
        Message::user().with_text("Hello"),
        Message::user().with_text("Another user message"),
        Message::assistant()
            .with_text("Response")
            .with_tool_response(
                "orphan_1",
                Ok(rmcp::model::CallToolResult::success(vec![
                    ContentBlock::text("result"),
                ])),
            ), // Wrong role
        Message::assistant().with_thinking("Let me think", "sig"),
        Message::user()
            .with_tool_request(
                "bad_req",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
            )
            .with_text("User with bad tool request"),
    ];

    let (fixed, issues) = run_verify(messages);

    assert_eq!(fixed.len(), 3);

    assert_has_issues_unordered!(
        fixed,
        issues,
        "Merged consecutive assistant messages",
        "Merged consecutive user messages",
        "Removed tool response 'orphan_1' from assistant message",
        "Removed tool request 'bad_req' from user message",
    );

    assert_eq!(fixed[0].role, Role::User);
    assert_eq!(fixed[1].role, Role::Assistant);
    assert_eq!(fixed[2].role, Role::User);

    assert_eq!(fixed[0].content.len(), 2);
}

#[test]
fn merge_does_not_cross_user_visibility() {
    let visible = Message::user().with_text("what is in main.rs?");
    let hidden = Message::user()
        .with_text("<turn-context>frozen</turn-context>")
        .with_visibility(false, true);

    let (fixed, _) = fix_conversation(Conversation::new_unvalidated(vec![visible, hidden]));
    assert_eq!(
        fixed.messages().len(),
        2,
        "the persistable form keeps the agent-only event separate"
    );
    assert!(fixed.messages()[0].is_user_visible());
    assert!(!fixed.messages()[1].is_user_visible());

    let merged =
        crate::conversation::merge_consecutive_messages_for_request(fixed.messages().clone());
    assert_eq!(
        merged.len(),
        1,
        "the request form merges to satisfy role alternation"
    );
    assert_eq!(merged[0].content.len(), 2);
}

#[test]
fn merge_does_not_erase_turn_context_marker() {
    let preserved_prompt = Message::user()
        .with_text("the preserved prompt")
        .with_metadata(MessageMetadata::agent_only());
    let carried_event = Message::user()
        .with_text("<turn-context>frozen</turn-context>")
        .with_metadata(MessageMetadata::agent_only().with_turn_context());

    let (fixed, _) = fix_conversation(Conversation::new_unvalidated(vec![
        preserved_prompt,
        carried_event,
    ]));
    assert_eq!(
        fixed.messages().len(),
        2,
        "the persistable form must not fold a turn-context event into the prompt"
    );
    assert!(!fixed.messages()[0].is_turn_context());
    assert!(fixed.messages()[1].is_turn_context());

    let merged =
        crate::conversation::merge_consecutive_messages_for_request(fixed.messages().clone());
    assert_eq!(
        merged.len(),
        1,
        "the request form still merges to satisfy role alternation"
    );
}

#[test]
fn test_orphaned_tools_and_empty_messages() {
    use rmcp::model::ContentBlock;

    // This conversation completely collapses. the first user message is invalid
    // then we remove the empty user message and the wrong tool response
    // then we collapse the assistant messages
    // which we then remove because you can't end a conversation with an assistant message
    let messages = vec![
        Message::assistant()
            .with_text("I'll search for you")
            .with_tool_request(
                "search_1",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
            ),
        Message::user(),
        Message::user().with_tool_response(
            "wrong_id",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("result"),
            ])),
        ),
        Message::assistant().with_tool_request(
            "search_2",
            Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
        ),
    ];

    let (fixed, issues) = run_verify(messages);

    assert_eq!(fixed.len(), 1);

    assert_has_issues_unordered!(
        fixed,
        issues,
        "Removed empty message",
        "Removed orphaned tool response 'wrong_id'",
        "Removed orphaned tool request 'search_1'",
        "Removed orphaned tool request 'search_2'",
        "Removed empty message",
        "Removed empty message",
        "Removed leading assistant message",
        "Added placeholder user message to empty conversation",
    );

    assert_eq!(fixed[0].role, Role::User);
    assert_eq!(fixed[0].as_concat_text(), "Hello");
}

#[test]
fn test_real_world_consecutive_assistant_messages() {
    use rmcp::model::ContentBlock;

    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("run ls in the current directory and then run a word count on the smallest file"),

        Message::assistant()
            .with_text("I'll help you run `ls` in the current directory and then perform a word count on the smallest file. Let me start by listing the directory contents.")
            .with_tool_request("toolu_bdrk_018adWbP4X26CfoJU5hkhu3i", Ok(CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "ls -la"})))),

        Message::assistant()
            .with_text("Now I'll identify the smallest file by size. Looking at the output, I can see that both `slack.yaml` and `subrecipes.yaml` have a size of 0 bytes, making them the smallest files. I'll run a word count on one of them:")
            .with_tool_request("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "wc slack.yaml"})))),

        Message::user()
            .with_tool_response("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(rmcp::model::CallToolResult::success(vec![ContentBlock::text("0 0 0 slack.yaml")]))),

        Message::assistant()
            .with_text("I ran `ls -la` in the current directory and found several files. Looking at the file sizes, I can see that both `slack.yaml` and `subrecipes.yaml` are 0 bytes (the smallest files). I ran a word count on `slack.yaml` which shows: **0 lines**, **0 words**, **0 characters**"),
        Message::user().with_text("thanks!"),
    ]);

    let (fixed, issues) = fix_conversation(conversation);

    assert_eq!(fixed.len(), 5);
    assert_has_issues_unordered!(
        fixed,
        issues,
        "Removed orphaned tool request 'toolu_bdrk_018adWbP4X26CfoJU5hkhu3i'",
        "Merged consecutive assistant messages"
    )
}

#[test]
fn test_tool_response_effective_role() {
    use rmcp::model::ContentBlock;

    let messages = vec![
        Message::user().with_text("Search for something"),
        Message::assistant()
            .with_text("I'll search for you")
            .with_tool_request(
                "search_1",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
            ),
        Message::user().with_tool_response(
            "search_1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("search results"),
            ])),
        ),
        Message::user().with_text("Thanks!"),
    ];

    let (_fixed, issues) = run_verify(messages);
    assert!(issues.is_empty());
}

#[test]
fn test_merge_text_content_items() {
    use rmcp::model::TextContent;

    let mut message = Message::assistant().with_text("Hello");

    message
        .content
        .push(MessageContentBlock::Text(TextContent::new(" world")));
    message
        .content
        .push(MessageContentBlock::Text(TextContent::new("!")));

    let messages = vec![
        Message::user().with_text("hello"),
        message,
        Message::user().with_text("thanks"),
    ];

    let (fixed, issues) = run_verify(messages);

    assert_eq!(fixed.len(), 3);
    assert_has_issues_unordered!(fixed, issues, "Merged text content");

    let fixed_msg = &fixed[1];
    assert_eq!(fixed_msg.content.len(), 1);

    if let MessageContentBlock::Text(text_content) = &fixed_msg.content[0] {
        assert_eq!(text_content.text, "Hello world!");
    } else {
        panic!("Expected text content");
    }
}

#[test]
fn test_merge_text_content_items_with_mixed_content() {
    use rmcp::model::TextContent;

    let mut image_message = Message::assistant().with_text("Look at");

    image_message
        .content
        .push(MessageContentBlock::Text(TextContent::new(" this image:")));

    image_message = image_message.with_image("", "");

    let messages = vec![
        Message::user().with_text("hello"),
        image_message,
        Message::user().with_text("thanks"),
    ];

    let (fixed, issues) = run_verify(messages);

    assert_eq!(fixed.len(), 3);
    assert_has_issues_unordered!(fixed, issues, "Merged text content");
    let fixed_msg = &fixed[1];

    assert_eq!(fixed_msg.content.len(), 2);
    if let MessageContentBlock::Text(text_content) = &fixed_msg.content[0] {
        assert_eq!(text_content.text, "Look at this image:");
    } else {
        panic!("Expected first item to be text content");
    }

    if let MessageContentBlock::Image(_) = &fixed_msg.content[1] {
        // Good
    } else {
        panic!("Expected second item to be an image");
    }
}

#[test]
fn test_streamed_text_with_different_audiences_is_not_merged() {
    use rmcp::model::{Annotations, Role, TextContent};

    let text = |value: &str, audience| {
        MessageContentBlock::Text(
            TextContent::new(value)
                .with_annotations(Annotations::default().with_audience(vec![audience])),
        )
    };

    for (first, second) in [(Role::User, Role::Assistant), (Role::Assistant, Role::User)] {
        let mut conversation = Conversation::empty();
        conversation.push(
            Message::assistant()
                .with_id("stream-1")
                .with_content(text("first", first.clone())),
        );
        conversation.push(
            Message::assistant()
                .with_id("stream-1")
                .with_content(text("second", second)),
        );

        let message = conversation.last().unwrap();
        assert_eq!(message.content.len(), 2);
        assert_eq!(
            message
                .user_visible_content()
                .content
                .iter()
                .filter_map(|content| match content {
                    MessageContentBlock::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            if first == Role::User {
                vec!["first"]
            } else {
                vec!["second"]
            }
        );
        assert_eq!(
            message
                .agent_visible_content()
                .content
                .iter()
                .filter_map(|content| match content {
                    MessageContentBlock::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            if first == Role::Assistant {
                vec!["first"]
            } else {
                vec!["second"]
            }
        );
    }
}
