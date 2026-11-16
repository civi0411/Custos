use crate::conversation::message::Message;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::{fix_conversation, Conversation};
use rmcp::model::CallToolRequestParams;
use rmcp::object;

#[test]
fn test_user_visible_messages_projects_content_and_drops_hidden_rows() {
    use rmcp::model::{Annotations, Role, TextContent};

    let assistant_only = |value: &str| {
        MessageContentBlock::Text(
            TextContent::new(value)
                .with_annotations(Annotations::default().with_audience(vec![Role::Assistant])),
        )
    };
    let conversation = Conversation::new_unvalidated([
        Message::assistant()
            .with_content(assistant_only("content hidden by audience"))
            .agent_only(),
        Message::assistant()
            .with_content(assistant_only("private"))
            .with_text("public"),
    ]);

    let projected = conversation.user_visible_messages();

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].as_concat_text(), "public");
}

#[test]
fn test_agent_visible_messages_projects_content_and_drops_hidden_rows() {
    use rmcp::model::{Annotations, Role, TextContent};

    let user_only = |value: &str| {
        MessageContentBlock::Text(
            TextContent::new(value)
                .with_annotations(Annotations::default().with_audience(vec![Role::User])),
        )
    };
    let conversation = Conversation::new_unvalidated([
        Message::assistant()
            .with_content(user_only("content hidden from agent"))
            .user_only(),
        Message::assistant()
            .with_content(user_only("private from agent"))
            .with_text("shared with agent"),
    ]);

    let projected = conversation.agent_visible_messages();

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].as_concat_text(), "shared with agent");
}

#[test]
fn test_agent_visible_non_visible_message_ordering_with_fixes() {
    // Test that non-visible messages maintain their position relative to visible messages
    // even when visible messages are fixed (merged, removed, etc.)

    // Create messages with mixed visibility where visible ones need fixing
    let mut msg1_user = Message::user().with_text("First user message");
    msg1_user.metadata.agent_visible = true;

    let mut msg2_non_visible = Message::user().with_text("Non-visible note 1");
    msg2_non_visible.metadata.agent_visible = false;

    // These two consecutive user messages should be merged (triggering a fix)
    let mut msg3_user = Message::user().with_text("Second user message");
    msg3_user.metadata.agent_visible = true;

    let mut msg4_user = Message::user().with_text("Third user message");
    msg4_user.metadata.agent_visible = true;

    let mut msg5_non_visible = Message::user().with_text("Non-visible note 2");
    msg5_non_visible.metadata.agent_visible = false;

    let mut msg6_assistant = Message::assistant().with_text("Assistant response");
    msg6_assistant.metadata.agent_visible = true;

    let mut msg7_non_visible = Message::user().with_text("Non-visible note 3");
    msg7_non_visible.metadata.agent_visible = false;

    let mut msg8_user = Message::user().with_text("Final user message");
    msg8_user.metadata.agent_visible = true;

    let messages = vec![
        msg1_user.clone(),
        msg2_non_visible.clone(),
        msg3_user.clone(),
        msg4_user.clone(),
        msg5_non_visible.clone(),
        msg6_assistant.clone(),
        msg7_non_visible.clone(),
        msg8_user.clone(),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages.clone()));

    // Should have merged consecutive user messages
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|i| i.contains("Merged consecutive")));

    let fixed_messages = fixed.messages();

    // Verify non-visible messages are still present
    let non_visible_texts: Vec<String> = fixed_messages
        .iter()
        .filter(|m| !m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(non_visible_texts.len(), 3);
    assert_eq!(non_visible_texts[0], "Non-visible note 1");
    assert_eq!(non_visible_texts[1], "Non-visible note 2");
    assert_eq!(non_visible_texts[2], "Non-visible note 3");

    // Verify visible messages were processed
    let visible_texts: Vec<String> = fixed_messages
        .iter()
        .filter(|m| m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    // Should have 3 visible messages: first user, merged user messages, assistant, final user
    // But after merging consecutive users and fixing lead/trail, we get fewer
    assert!(!visible_texts.is_empty());

    // The key assertion: non-visible messages should be preserved and not reordered
    // relative to each other
    let mut found_note1 = false;
    let mut found_note2 = false;

    for msg in fixed_messages {
        let text = msg.as_concat_text();
        if text == "Non-visible note 1" {
            assert!(!found_note2 && !found_note1);
            found_note1 = true;
        } else if text == "Non-visible note 2" {
            assert!(found_note1 && !found_note2);
            found_note2 = true;
        } else if text == "Non-visible note 3" {
            assert!(found_note1 && found_note2);
        }
    }
}

#[test]
fn test_shadow_map_with_multiple_consecutive_merges() {
    // Test the shadow map handles multiple consecutive visible messages that all merge
    let mut msg1 = Message::user().with_text("User 1");
    msg1.metadata.agent_visible = true;

    let mut msg2_non_vis = Message::user().with_text("Non-visible A");
    msg2_non_vis.metadata.agent_visible = false;

    let mut msg3 = Message::user().with_text("User 2");
    msg3.metadata.agent_visible = true;

    let mut msg4 = Message::user().with_text("User 3");
    msg4.metadata.agent_visible = true;

    let mut msg5 = Message::user().with_text("User 4");
    msg5.metadata.agent_visible = true;

    let mut msg6_non_vis = Message::user().with_text("Non-visible B");
    msg6_non_vis.metadata.agent_visible = false;

    let messages = vec![
        msg1,
        msg2_non_vis.clone(),
        msg3,
        msg4,
        msg5,
        msg6_non_vis.clone(),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    // Should have merged the consecutive user messages
    assert!(issues.iter().any(|i| i.contains("Merged consecutive")));

    let fixed_messages = fixed.messages();

    // Non-visible messages should still be present and in order
    let non_visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| !m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(non_visible.len(), 2);
    assert_eq!(non_visible[0], "Non-visible A");
    assert_eq!(non_visible[1], "Non-visible B");

    // The merged message should contain all the user texts
    let visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(visible.len(), 1);
    assert!(visible[0].contains("User 1"));
    assert!(visible[0].contains("User 2"));
    assert!(visible[0].contains("User 3"));
    assert!(visible[0].contains("User 4"));
}

#[test]
fn test_shadow_map_with_leading_trailing_removal() {
    // Test that shadow map handles removal of leading/trailing assistant messages
    let mut msg1_assistant = Message::assistant().with_text("Leading assistant");
    msg1_assistant.metadata.agent_visible = true;

    let mut msg2_non_vis = Message::user().with_text("Non-visible note");
    msg2_non_vis.metadata.agent_visible = false;

    let mut msg3_user = Message::user().with_text("User message");
    msg3_user.metadata.agent_visible = true;

    let mut msg4_assistant = Message::assistant().with_text("Assistant response");
    msg4_assistant.metadata.agent_visible = true;

    let mut msg5_assistant = Message::assistant().with_text("Trailing assistant");
    msg5_assistant.metadata.agent_visible = true;

    let messages = vec![
        msg1_assistant,
        msg2_non_vis.clone(),
        msg3_user,
        msg4_assistant,
        msg5_assistant,
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    // Should have merged consecutive assistants, removed leading, and removed trailing
    assert!(issues
        .iter()
        .any(|i| i.contains("Merged consecutive assistant")));
    assert!(issues
        .iter()
        .any(|i| i.contains("Removed leading assistant")));
    assert!(issues
        .iter()
        .any(|i| i.contains("Removed trailing assistant")));

    let fixed_messages = fixed.messages();

    // Non-visible message should still be present
    let non_visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| !m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(non_visible.len(), 1);
    assert_eq!(non_visible[0], "Non-visible note");

    // The two consecutive assistant messages get merged, then the merged message
    // is removed as trailing, leaving only the user message
    let visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0], "User message");
}

#[test]
fn test_shadow_map_all_visible_messages_removed() {
    // Edge case: all visible messages are removed, only non-visible remain
    let mut msg1_assistant = Message::assistant().with_text("Only assistant");
    msg1_assistant.metadata.agent_visible = true;

    let mut msg2_non_vis = Message::user().with_text("Non-visible note 1");
    msg2_non_vis.metadata.agent_visible = false;

    let mut msg3_non_vis = Message::user().with_text("Non-visible note 2");
    msg3_non_vis.metadata.agent_visible = false;

    let messages = vec![msg1_assistant, msg2_non_vis, msg3_non_vis];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    // Should have removed the assistant and added placeholder
    assert!(issues
        .iter()
        .any(|i| i.contains("Removed leading assistant")));
    assert!(issues.iter().any(|i| i.contains("Added placeholder")));

    let fixed_messages = fixed.messages();

    // Non-visible messages should still be present
    let non_visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| !m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(non_visible.len(), 2);
    assert_eq!(non_visible[0], "Non-visible note 1");
    assert_eq!(non_visible[1], "Non-visible note 2");

    // Should have placeholder user message
    let visible: Vec<String> = fixed_messages
        .iter()
        .filter(|m| m.metadata.agent_visible)
        .map(|m| m.as_concat_text())
        .collect();

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0], "Hello");
}

#[test]
fn test_empty_tool_result_gets_placeholder() {
    // Test that tool results with empty content get a placeholder added
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
            Ok(rmcp::model::CallToolResult::success(vec![])), // Empty content - this should get a placeholder
        ),
        Message::user().with_text("Thanks!"),
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    // Should have added a placeholder
    assert!(issues
        .iter()
        .any(|i| i.contains("Added placeholder to empty tool result")));

    // Find the tool response and verify it has content now
    let tool_response_msg = fixed
        .messages()
        .iter()
        .find(|m| {
            m.content.iter().any(|c| {
                matches!(
                    c,
                    crate::conversation::message::MessageContentBlock::ToolResponse(_)
                )
            })
        })
        .expect("Should have a tool response message");

    if let crate::conversation::message::MessageContentBlock::ToolResponse(resp) =
        &tool_response_msg.content[0]
    {
        if let Ok(result) = &resp.tool_result {
            assert!(!result.content.is_empty(), "Content should not be empty");
            // Verify the placeholder text
            let text = result.content[0]
                .as_text()
                .expect("Should be text content")
                .text
                .clone();
            assert_eq!(text, "(empty result)");
        } else {
            panic!("Tool result should be Ok");
        }
    } else {
        panic!("First content should be ToolResponse");
    }
}

#[test]
fn test_shadow_map_preserves_interleaving_pattern() {
    // Test that complex interleaving patterns are preserved
    let mut msg1_user = Message::user().with_text("User 1");
    msg1_user.metadata.agent_visible = true;

    let mut msg2_non_vis = Message::user().with_text("Non-vis A");
    msg2_non_vis.metadata.agent_visible = false;

    let mut msg3_assistant = Message::assistant().with_text("Assistant 1");
    msg3_assistant.metadata.agent_visible = true;

    let mut msg4_non_vis = Message::user().with_text("Non-vis B");
    msg4_non_vis.metadata.agent_visible = false;

    let mut msg5_user = Message::user().with_text("User 2");
    msg5_user.metadata.agent_visible = true;

    let mut msg6_non_vis = Message::user().with_text("Non-vis C");
    msg6_non_vis.metadata.agent_visible = false;

    let messages = vec![
        msg1_user,
        msg2_non_vis,
        msg3_assistant,
        msg4_non_vis,
        msg5_user,
        msg6_non_vis,
    ];

    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

    // Should have no issues for this valid conversation
    assert!(issues.is_empty());

    let fixed_messages = fixed.messages();

    // Verify the interleaving pattern is preserved
    assert_eq!(fixed_messages.len(), 6);

    assert_eq!(fixed_messages[0].as_concat_text(), "User 1");
    assert!(fixed_messages[0].metadata.agent_visible);

    assert_eq!(fixed_messages[1].as_concat_text(), "Non-vis A");
    assert!(!fixed_messages[1].metadata.agent_visible);

    assert_eq!(fixed_messages[2].as_concat_text(), "Assistant 1");
    assert!(fixed_messages[2].metadata.agent_visible);

    assert_eq!(fixed_messages[3].as_concat_text(), "Non-vis B");
    assert!(!fixed_messages[3].metadata.agent_visible);

    assert_eq!(fixed_messages[4].as_concat_text(), "User 2");
    assert!(fixed_messages[4].metadata.agent_visible);

    assert_eq!(fixed_messages[5].as_concat_text(), "Non-vis C");
    assert!(!fixed_messages[5].metadata.agent_visible);
}
