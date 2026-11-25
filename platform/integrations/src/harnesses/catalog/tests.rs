use super::*;

#[test]
fn test_catalog_specs_completeness() {
    assert_eq!(
        SPECS.len(),
        45,
        "Catalog must contain exactly 45 standard harnesses"
    );
    assert!(find_spec("claude-code").is_some());
    assert!(find_spec("codex").is_some());
    assert!(find_spec("goose").is_some());
    assert!(find_spec("opencode").is_some());
    assert!(find_spec("antigravity").is_some());
    assert!(find_spec("cursor").is_some());
}

#[test]
fn test_catalog_args_parity() {
    let instr = "fix the bug";
    assert_eq!(
        find_spec("claude-code").unwrap().headless.build_args(instr),
        vec![
            "--print",
            "fix the bug",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages"
        ]
    );
    assert_eq!(
        find_spec("codex").unwrap().headless.build_args(instr),
        vec!["exec", "--json", "fix the bug"]
    );
    assert_eq!(
        find_spec("opencode").unwrap().headless.build_args(instr),
        vec!["run", "--format", "json", "fix the bug"]
    );
    assert_eq!(
        find_spec("openclaude").unwrap().headless.build_args(instr),
        vec!["-p", "fix the bug"]
    );
    assert_eq!(
        find_spec("goose").unwrap().headless.build_args(instr),
        vec!["run", "--text", "fix the bug"]
    );
    assert_eq!(
        find_spec("opencode2").unwrap().headless.build_args(instr),
        vec!["run", "fix the bug"]
    );
    assert_eq!(
        find_spec("gemini").unwrap().headless.build_args(instr),
        vec!["-p", "fix the bug"]
    );
    assert_eq!(
        find_spec("antigravity").unwrap().headless.build_args(instr),
        vec!["-p", "fix the bug"]
    );
    assert_eq!(
        find_spec("kimi").unwrap().headless.build_args(instr),
        vec!["-p", "fix the bug"]
    );
    assert_eq!(
        find_spec("aider").unwrap().headless.build_args(instr),
        vec!["--message", "fix the bug", "--yes-always"]
    );
    assert_eq!(
        find_spec("cursor").unwrap().headless.build_args(instr),
        vec!["--", "fix the bug"]
    );
    assert_eq!(
        find_spec("grok").unwrap().headless.build_args(instr),
        vec!["--", "fix the bug"]
    );
    assert_eq!(
        find_spec("copilot").unwrap().headless.build_args(instr),
        vec!["fix the bug"]
    );
}
