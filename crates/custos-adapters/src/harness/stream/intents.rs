//! Extraction of structured ActionIntents from raw stream logs or events.

use custos_domain::{new_id, ActionIntent, Assurance, RiskLevel};
use serde_json::Value;

/// Unified normalization of raw process output or stream lines into structured `ActionIntent`s.
/// Handles Claude stream-json, Codex item/tool events, Goose tools, OpenCode parts, and line-by-line CLI text heuristics.
pub fn extract_observed_intents(raw_output: &str, _harness: &str, task_id: &str) -> Vec<ActionIntent> {
    let mut intents = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 1. Structured JSON parsing
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                // A. Direct Claude tool_use event: {"type": "tool_use", "name": "...", "input": {...}}
                // or content_block_start: {"event": {"type": "content_block_start", "content_block": {"type": "tool_use", ...}}}
                let direct_tool_use = if value.get("type").and_then(Value::as_str) == Some("tool_use") && value.get("part").is_none() {
                    Some(&value)
                } else if value.pointer("/event/type").and_then(Value::as_str) == Some("content_block_start") {
                    let cb = &value["event"]["content_block"];
                    if cb.get("type").and_then(Value::as_str) == Some("tool_use") {
                        Some(cb)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(block) = direct_tool_use {
                    let tool_name = block
                        .get("name")
                        .or_else(|| block.get("tool"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let input = block
                        .get("input")
                        .cloned()
                        .unwrap_or(Value::Null);

                    let (action_name, target, risk) = match tool_name {
                        "bash" | "execute_command" => {
                            let cmd = input.get("command").and_then(Value::as_str).unwrap_or("");
                            ("shell_exec", cmd.to_string(), RiskLevel::High)
                        }
                        "write_file" | "create_file" => {
                            let path = input
                                .get("path")
                                .or_else(|| input.get("file_path"))
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            ("file_write", path.to_string(), RiskLevel::Medium)
                        }
                        "edit_file" | "patch" => {
                            let path = input
                                .get("path")
                                .or_else(|| input.get("file_path"))
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            ("file_edit", path.to_string(), RiskLevel::Medium)
                        }
                        "view_file" | "read_file" => {
                            let path = input
                                .get("path")
                                .or_else(|| input.get("file_path"))
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            ("file_read", path.to_string(), RiskLevel::Low)
                        }
                        other => (other, "native_tool".to_string(), RiskLevel::Medium),
                    };

                    let intent = ActionIntent::new(
                        new_id("act_native"),
                        action_name.into(),
                        target,
                        serde_json::json!({
                            "input": input,
                            "provider_governed": true,
                            "assurance": "provider-governed"
                        }),
                        risk,
                    )
                    .with_task_id(task_id)
                    .with_assurance(Assurance::ProviderGoverned);

                    intents.push(intent);
                    continue;
                }

                // B. Codex item completed or item started
                if let Some(item) = value.get("item") {
                    let item_kind = item.get("type").and_then(Value::as_str).unwrap_or("");
                    if matches!(
                        item_kind,
                        "command_execution" | "mcp_tool_call" | "file_change" | "web_search"
                    ) {
                        let identity = item.get("id").and_then(Value::as_str).unwrap_or("");
                        if identity.is_empty() || seen.insert(identity.to_string()) {
                            let target = item
                                .get("command")
                                .or_else(|| item.get("changes"))
                                .and_then(Value::as_str)
                                .unwrap_or("native_workspace");
                            let intent = ActionIntent::new(
                                new_id("act_native"),
                                item_kind.to_string(),
                                target.to_string(),
                                serde_json::json!({
                                    "item": item,
                                    "provider_governed": true,
                                    "assurance": "provider-governed"
                                }),
                                RiskLevel::High,
                            )
                            .with_task_id(task_id)
                            .with_assurance(Assurance::ProviderGoverned);

                            intents.push(intent);
                            continue;
                        }
                    }
                }

                // C. Codex function / tool_call
                if let Some(tool_call) = value.get("function").or_else(|| value.get("tool_call")) {
                    let name = tool_call
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("codex_exec");
                    let args = tool_call
                        .get("arguments")
                        .cloned()
                        .unwrap_or(Value::Null);

                    let (action_name, target, risk) = match name {
                        "apply_patch" | "write_file" => {
                            let path = args.get("path").and_then(Value::as_str).unwrap_or("unknown");
                            ("write_file", path.to_string(), RiskLevel::High)
                        }
                        "exec_shell" | "bash" => {
                            let cmd = args.get("cmd").and_then(Value::as_str).unwrap_or("");
                            ("shell_exec", cmd.to_string(), RiskLevel::High)
                        }
                        _ => (name, "workspace".to_string(), RiskLevel::Medium),
                    };

                    let intent = ActionIntent::new(
                        new_id("intent"),
                        action_name.to_string(),
                        target,
                        args,
                        risk,
                    )
                    .with_task_id(task_id)
                    .with_assurance(Assurance::ProviderGoverned);

                    intents.push(intent);
                    continue;
                }

                // D. Goose tool: {"tool": "...", "parameters": {...}}
                let goose_tool = value.get("tool").or_else(|| value.get("name")).and_then(Value::as_str);
                if let Some(tool_name) = goose_tool {
                    if !tool_name.is_empty() {
                        let params = value.get("parameters").or_else(|| value.get("arguments")).cloned().unwrap_or(Value::Null);
                        let (action_name, target, risk) = match tool_name {
                            "developer__text_editor" | "text_editor" => {
                                let path = params.get("path").and_then(Value::as_str).unwrap_or("unknown");
                                ("write_file", path.to_string(), RiskLevel::High)
                            }
                            "developer__shell" | "shell" => {
                                let cmd = params.get("command").and_then(Value::as_str).unwrap_or("");
                                ("shell_exec", cmd.to_string(), RiskLevel::High)
                            }
                            _ => (tool_name, "workspace".to_string(), RiskLevel::Medium),
                        };

                        let intent = ActionIntent::new(
                            new_id("intent"),
                            action_name.to_string(),
                            target,
                            params,
                            risk,
                        )
                        .with_task_id(task_id)
                        .with_assurance(Assurance::ProviderGoverned);

                        intents.push(intent);
                        continue;
                    }
                }

                // E. OpenCode part tool_use
                if value.get("type").and_then(Value::as_str) == Some("tool_use") && value.get("part").is_some() {
                    let block = &value["part"];
                    let identity = block.get("id").or_else(|| block.get("callID")).and_then(Value::as_str).unwrap_or("");
                    if identity.is_empty() || seen.insert(identity.to_string()) {
                        let target = block.pointer("/input/file_path").or_else(|| block.pointer("/input/command")).and_then(Value::as_str).unwrap_or("native_workspace");
                        let intent = ActionIntent::new(
                            new_id("intent"),
                            block.get("tool").and_then(Value::as_str).unwrap_or("tool_use").to_string(),
                            target.to_string(),
                            block.clone(),
                            RiskLevel::High,
                        )
                        .with_task_id(task_id)
                        .with_assurance(Assurance::ProviderGoverned);

                        intents.push(intent);
                        continue;
                    }
                }
            }
        }

        // 2. Line-by-line CLI text heuristics fallback
        if trimmed.starts_with("[tool_call:bash]") || trimmed.starts_with("$ ") {
            let cmd = trimmed
                .trim_start_matches("[tool_call:bash]")
                .trim_start_matches("$ ")
                .trim();
            let intent = ActionIntent::new(
                new_id("act_native"),
                "shell_exec".into(),
                "native_shell".into(),
                serde_json::json!({
                    "command": cmd,
                    "provider_governed": true,
                    "assurance": "provider-governed"
                }),
                RiskLevel::High,
            )
            .with_task_id(task_id)
            .with_assurance(Assurance::ProviderGoverned);

            intents.push(intent);
        } else if trimmed.starts_with("[tool_call:write_file]") || trimmed.contains("Writing file:") {
            let file_path = trimmed
                .trim_start_matches("[tool_call:write_file]")
                .split("Writing file:")
                .last()
                .unwrap_or("")
                .trim();
            let intent = ActionIntent::new(
                new_id("act_native"),
                "file_write".into(),
                file_path.into(),
                serde_json::json!({
                    "path": file_path,
                    "provider_governed": true,
                    "assurance": "provider-governed"
                }),
                RiskLevel::Medium,
            )
            .with_task_id(task_id)
            .with_assurance(Assurance::ProviderGoverned);

            intents.push(intent);
        } else if trimmed.starts_with("[tool_call:edit_file]")
            || trimmed.contains("Editing file:")
            || trimmed.contains("Patching file:")
        {
            let file_path = trimmed
                .trim_start_matches("[tool_call:edit_file]")
                .split("Editing file:")
                .last()
                .unwrap_or("")
                .split("Patching file:")
                .last()
                .unwrap_or("")
                .trim();
            let intent = ActionIntent::new(
                new_id("act_native"),
                "file_edit".into(),
                file_path.into(),
                serde_json::json!({
                    "path": file_path,
                    "provider_governed": true,
                    "assurance": "provider-governed"
                }),
                RiskLevel::Medium,
            )
            .with_task_id(task_id)
            .with_assurance(Assurance::ProviderGoverned);

            intents.push(intent);
        } else if trimmed.starts_with("[tool_call:view_file]") || trimmed.contains("Reading file:") {
            let file_path = trimmed
                .trim_start_matches("[tool_call:view_file]")
                .split("Reading file:")
                .last()
                .unwrap_or("")
                .trim();
            let intent = ActionIntent::new(
                new_id("act_native"),
                "file_read".into(),
                file_path.into(),
                serde_json::json!({
                    "path": file_path,
                    "provider_governed": true,
                    "assurance": "provider-governed"
                }),
                RiskLevel::Low,
            )
            .with_task_id(task_id)
            .with_assurance(Assurance::ProviderGoverned);

            intents.push(intent);
        }
    }

    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_observed_intents_comprehensive() {
        let mock_cli_log = r#"
Thinking process completed.
$ cargo test --package my-crate
Writing file: src/domain/mod.rs
Editing file: src/lib.rs
Reading file: README.md
{"type":"tool_use","name":"bash","input":{"command":"git status"}}
All tests green.
"#;
        let intents = extract_observed_intents(mock_cli_log, "claude-code", "task_001");
        assert_eq!(intents.len(), 5);
        assert_eq!(intents[0].name, "shell_exec");
        assert_eq!(intents[1].name, "file_write");
        assert_eq!(intents[2].name, "file_edit");
        assert_eq!(intents[3].name, "file_read");
        assert_eq!(intents[4].name, "shell_exec");
        for intent in &intents {
            assert_eq!(intent.task_id.as_deref(), Some("task_001"));
            assert_eq!(intent.assurance, Assurance::ProviderGoverned);
        }
    }
}
