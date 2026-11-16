use super::parser::{EmulatorAction, StreamingEmulatorParser};
#[cfg(feature = "mlx")]
use super::prompt::build_emulator_tool_description;

fn parse_chunks(chunks: &[&str], code_mode: bool) -> Vec<EmulatorAction> {
    let mut parser = StreamingEmulatorParser::new(code_mode);
    let mut actions = Vec::new();
    for chunk in chunks {
        actions.extend(parser.process_chunk(chunk));
    }
    actions.extend(parser.flush());
    actions
}

fn parse_all(input: &str, code_mode: bool) -> Vec<EmulatorAction> {
    parse_chunks(&[input], code_mode)
}

fn text(actions: &[EmulatorAction]) -> String {
    actions
        .iter()
        .filter_map(|action| match action {
            EmulatorAction::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn shell_commands(actions: &[EmulatorAction]) -> Vec<&str> {
    actions
        .iter()
        .filter_map(|action| match action {
            EmulatorAction::ShellCommand(command) => Some(command.as_str()),
            _ => None,
        })
        .collect()
}

fn execute_blocks(actions: &[EmulatorAction]) -> Vec<&str> {
    actions
        .iter()
        .filter_map(|action| match action {
            EmulatorAction::ExecuteCode(code) => Some(code.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn plain_text_is_preserved() {
    let input = "Hello, world!\n";
    let actions = parse_all(input, false);

    assert_eq!(text(&actions), input);
    assert!(shell_commands(&actions).is_empty());
    assert!(execute_blocks(&actions).is_empty());
}

#[test]
fn top_level_shell_commands_are_detected() {
    let actions = parse_chunks(&["Let me check:\n", "$ who", "ami\n$ pwd"], false);

    assert_eq!(shell_commands(&actions), ["whoami", "pwd"]);
    assert_eq!(text(&actions), "Let me check:\n");
}

#[test]
fn dollar_sign_mid_sentence_is_text() {
    let input = "It costs $50 per month";
    let actions = parse_all(input, false);

    assert_eq!(text(&actions), input);
    assert!(shell_commands(&actions).is_empty());
}

#[test]
fn markdown_contained_shell_commands_remain_text() {
    let cases = [
        "````markdown\n$ inert\n````\n",
        "> $ inert\n",
        "- $ inert\n",
        "<div>\n$ inert\n\n",
    ];

    for input in cases {
        let actions = parse_all(input, true);
        assert_eq!(text(&actions), input, "input: {input:?}");
        assert!(shell_commands(&actions).is_empty(), "input: {input:?}");
    }
}

#[test]
fn top_level_execute_block_after_text_is_detected() {
    let input = "Here's the code:\n```execute_typescript\nconsole.log('hi');\n```\n";
    let actions = parse_all(input, true);

    assert_eq!(text(&actions), "Here's the code:\n");
    assert_eq!(execute_blocks(&actions), ["console.log('hi');"]);
}

#[test]
fn execute_fence_can_be_split_across_chunks() {
    let input = "```execute_typescript\nlet x = 1;\n```\n";
    let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
    let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    let actions = parse_chunks(&chunk_refs, true);

    assert_eq!(execute_blocks(&actions), ["let x = 1;"]);
}

#[test]
fn execute_blocks_are_disabled_without_code_mode() {
    let input = "```execute_typescript\nlet x = 1;\n```\n";
    let actions = parse_all(input, false);

    assert_eq!(text(&actions), input);
    assert!(execute_blocks(&actions).is_empty());
}

#[test]
fn nested_execute_fences_remain_text() {
    let cases = [
        "````markdown\n```execute_typescript\ninert();\n```\n````\n",
        "~~~markdown\n```execute_typescript\ninert();\n```\n~~~\n",
        "> ```execute_typescript\n> inert();\n> ```\n",
        "- ```execute_typescript\n  inert();\n  ```\n",
        "<div>\n```execute_typescript\ninert();\n```\n\n",
    ];

    for input in cases {
        let actions = parse_all(input, true);
        assert_eq!(text(&actions), input, "input: {input:?}");
        assert!(execute_blocks(&actions).is_empty(), "input: {input:?}");
    }
}

#[test]
fn invalid_fence_info_does_not_hide_following_execute() {
    let input = "```execute_typescript```\n```execute_typescript\nsafe();\n```\n";
    let actions = parse_all(input, true);

    assert_eq!(execute_blocks(&actions), ["safe();"]);
    assert!(text(&actions).contains("```execute_typescript```"));
}

#[test]
fn longer_execute_fence_requires_matching_close() {
    let input = "````execute_typescript\nlet before = 1;\n```\nlet after = 2;\n````\n";
    let actions = parse_all(input, true);

    assert_eq!(
        execute_blocks(&actions),
        ["let before = 1;\n```\nlet after = 2;"]
    );
}

#[test]
fn closing_fence_waits_for_complete_line() {
    let mut parser = StreamingEmulatorParser::new(true);

    assert!(parser
        .process_chunk("```execute_typescript\nlet x = 1;\n```")
        .iter()
        .all(|action| !matches!(action, EmulatorAction::ExecuteCode(_))));
    assert!(parser
        .process_chunk("not-a-close")
        .iter()
        .all(|action| !matches!(action, EmulatorAction::ExecuteCode(_))));
    let actions = parser.process_chunk("\n```\n");

    assert_eq!(execute_blocks(&actions), ["let x = 1;\n```not-a-close"]);
}

#[test]
fn unicode_whitespace_does_not_close_execute_fence() {
    let input = "```execute_typescript\nlet before = 1;\n```\u{a0}\nlet after = 2;\n```\n";
    let actions = parse_all(input, true);

    assert_eq!(
        execute_blocks(&actions),
        ["let before = 1;\n```\u{a0}\nlet after = 2;"]
    );
}

#[test]
fn markdown_parse_work_is_bounded() {
    let mut input = String::from("````markdown\n");
    for _ in 0..1000 {
        input.push_str("$ inert\n");
    }
    input.push_str("````\n$ still inert\n");

    let actions = parse_all(&input, true);

    assert_eq!(text(&actions), input);
    assert!(shell_commands(&actions).is_empty());
    assert!(execute_blocks(&actions).is_empty());
}

#[test]
fn eof_flushes_closed_and_unclosed_execute_blocks() {
    for (input, expected) in [
        ("```execute_typescript\nsafe();\n```", "safe();"),
        ("```execute_typescript\nsafe();", "safe();"),
    ] {
        let actions = parse_all(input, true);
        assert_eq!(execute_blocks(&actions), [expected]);
    }
}

#[cfg(feature = "mlx")]
#[test]
fn tool_description_uses_execute_typescript_fence() {
    let description = build_emulator_tool_description(&[], true);

    assert!(description.contains("```execute_typescript"));
}

