//! Structured native process observations; unknown logs never imply thinking or tools.
use custos_core::contracts::harness::{HarnessExecutionResult, RunObserver, ToolMediationLevel};
use custos_domain::{new_id, ActionIntent, Assurance, DomainError, RiskLevel};
use serde_json::Value;
use std::{collections::HashMap, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::mpsc,
};

async fn read_pipe<R: AsyncRead + Unpin>(
    mut pipe: R,
    tx: mpsc::Sender<(bool, Vec<u8>)>,
    stderr: bool,
) -> std::io::Result<()> {
    loop {
        let mut buf = vec![0; 8192];
        let count = pipe.read(&mut buf).await?;
        if count == 0 {
            break;
        }
        buf.truncate(count);
        if tx.send((stderr, buf)).await.is_err() {
            break;
        }
    }
    Ok(())
}

pub async fn run_observed_process(
    mut command: Command,
    observer: Arc<dyn RunObserver>,
    harness: &str,
    timeout: Duration,
) -> Result<HarnessExecutionResult, DomainError> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| DomainError::Validation(format!("Native process launch failed: {e}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DomainError::Validation("Missing native stdout".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DomainError::Validation("Missing native stderr".into()))?;
    let (tx, mut rx) = mpsc::channel(32);
    let out_reader = tokio::spawn(read_pipe(stdout, tx.clone(), false));
    let err_reader = tokio::spawn(read_pipe(stderr, tx, true));
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let mut pending = Vec::new();
    let mut parts = HashMap::new();
    let mut answer = String::new();
    let deadline = tokio::time::Instant::now() + timeout;
    let mut cancelled = false;
    loop {
        if observer.is_cancelled() || tokio::time::Instant::now() >= deadline {
            cancelled = true;
            break;
        }
        tokio::select! {
            chunk=rx.recv()=> {
                let Some((stderr,bytes))=chunk else {break;};
                let target=if stderr {&mut errors} else {&mut output};
                if target.len().saturating_add(bytes.len())>2*1024*1024 { cancelled=true;break; }
                target.extend_from_slice(&bytes);
                if !stderr {
                    pending.extend_from_slice(&bytes);
                    while let Some(end)=pending.iter().position(|b|*b==b'\n') {
                        let line:Vec<_>=pending.drain(..=end).collect();
                        if let Ok(value)=serde_json::from_slice::<Value>(&line) { normalize(&value,harness,&observer,&mut parts,&mut answer); }
                    }
                }
            },
            _=tokio::time::sleep(Duration::from_millis(50))=> {
                if observer.is_cancelled() || tokio::time::Instant::now()>=deadline { cancelled=true;break; }
            },
        }
    }
    if cancelled {
        #[cfg(windows)]
        if let Some(pid) = child.id() {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
        let _ = child.kill().await;
    }
    let status = loop {
        tokio::select! {
            result=child.wait()=>break result.map_err(|e|DomainError::Validation(format!("Native wait failed: {e}")))?,
            _=tokio::time::sleep(Duration::from_millis(50))=> {
                if observer.is_cancelled() || tokio::time::Instant::now()>=deadline {
                    cancelled=true;
                    #[cfg(windows)]
                    if let Some(pid)=child.id() { let _=Command::new("taskkill").args(["/PID",&pid.to_string(),"/T","/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status().await; }
                    let _=child.kill().await;
                }
            }
        }
    };
    if cancelled {
        out_reader.abort();
        err_reader.abort();
    } else {
        out_reader
            .await
            .map_err(|_| DomainError::Validation("Native stdout reader failed".into()))?
            .map_err(|_| DomainError::Validation("Native stdout capture failed".into()))?;
        err_reader
            .await
            .map_err(|_| DomainError::Validation("Native stderr reader failed".into()))?
            .map_err(|_| DomainError::Validation("Native stderr capture failed".into()))?;
    }
    if cancelled {
        errors.extend_from_slice(if observer.is_cancelled() {
            b"\nRun cancelled"
        } else {
            b"\nNative timeout or output limit exceeded"
        });
    }
    if !pending.is_empty() {
        if let Ok(value) = serde_json::from_slice(&pending) {
            normalize(&value, harness, &observer, &mut parts, &mut answer);
        }
    }
    Ok(HarnessExecutionResult {
        success: !cancelled && status.success(),
        exit_code: status.code(),
        stdout: answer,
        stderr: String::from_utf8_lossy(&errors).into_owned(),
        observed_effects: observed_intents(&output, harness),
        mediation_level: ToolMediationLevel::ProviderGoverned,
    })
}

fn normalize(
    value: &Value,
    harness: &str,
    observer: &Arc<dyn RunObserver>,
    parts: &mut HashMap<String, String>,
    answer: &mut String,
) {
    let kind = value["type"].as_str().unwrap_or("");
    let mut text: Option<String> = None;
    if harness == "codex" {
        let item = &value["item"];
        let item_kind = item["type"].as_str().unwrap_or("");
        if matches!(kind, "item.started" | "item.updated" | "item.completed") {
            if item_kind == "agent_message" {
                let id = item["id"].as_str().unwrap_or("answer");
                let full = item["text"].as_str().unwrap_or("");
                let new_message = !parts.contains_key(id) && !answer.is_empty();
                let previous = parts.entry(id.into()).or_default();
                if let Some(delta) = full.strip_prefix(previous.as_str()) {
                    text = Some(if new_message {
                        format!("\n\n{delta}")
                    } else {
                        delta.into()
                    });
                }
                *previous = full.into();
            } else if item_kind == "reasoning" {
                observer.emit("status", serde_json::json!({"status":"thinking"}));
            } else if matches!(
                item_kind,
                "command_execution" | "mcp_tool_call" | "file_change" | "web_search"
            ) {
                observer.emit("status", serde_json::json!({"status":"tool-running"}));
                observer.emit(if kind=="item.completed" {"tool_output"} else {"tool_input"},serde_json::json!({"name":item_kind,"call_id":item["id"],"command":item["command"],"output":item["aggregated_output"],"changes":item["changes"]}));
            }
        }
        if kind == "turn.completed" {
            emit_usage(observer, value["usage"].clone());
        }
    } else if harness == "claude-code" {
        let event = value.get("event").unwrap_or(value);
        if event["type"].as_str() == Some("content_block_delta") {
            if event.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") {
                text = event
                    .pointer("/delta/text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            if event.pointer("/delta/type").and_then(Value::as_str) == Some("input_json_delta") {
                observer.emit("tool_input",serde_json::json!({"index":event["index"],"partial_json":event.pointer("/delta/partial_json")}));
            }
            if event.pointer("/delta/type").and_then(Value::as_str) == Some("thinking_delta") {
                observer.emit("status", serde_json::json!({"status":"thinking"}));
            }
        }
        if event["type"].as_str() == Some("content_block_start")
            && event.pointer("/content_block/type").and_then(Value::as_str) == Some("tool_use")
        {
            observer.emit("status", serde_json::json!({"status":"tool-running"}));
            observer.emit("tool_input", event["content_block"].clone());
        }
        if kind == "assistant" {
            if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
                for block in blocks {
                    if block["type"].as_str() == Some("tool_use") {
                        observer.emit("status", serde_json::json!({"status":"tool-running"}));
                        observer.emit("tool_input", block.clone());
                    }
                }
            }
        }
        if kind == "user" {
            if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
                for block in blocks {
                    if block["type"].as_str() == Some("tool_result") {
                        observer.emit("tool_output", block.clone());
                    }
                }
            }
        }
        if kind == "result" {
            emit_usage(observer, value["usage"].clone());
            if answer.is_empty() {
                text = value["result"].as_str().map(str::to_string);
            }
        }
    } else if harness == "opencode" {
        if kind == "text" {
            text = value
                .pointer("/part/text")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if kind == "reasoning" {
            observer.emit("status", serde_json::json!({"status":"thinking"}));
        }
        if kind == "tool_use" {
            observer.emit("status", serde_json::json!({"status":"tool-running"}));
            observer.emit("tool_input",serde_json::json!({"name":value.pointer("/part/tool"),"input":value.pointer("/part/state/input")}));
            if value.pointer("/part/state/output").is_some() {
                observer.emit(
                    "tool_output",
                    serde_json::json!({"output":value.pointer("/part/state/output")}),
                );
            }
        }
        if kind == "step_finish" {
            observer.emit(
                "usage",
                value
                    .pointer("/part/tokens")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
    }
    if let Some(text) = text.filter(|text| !text.is_empty()) {
        if answer.len() + text.len() <= 2 * 1024 * 1024 {
            answer.push_str(&text);
            observer.emit("status", serde_json::json!({"status":"running"}));
            observer.emit("text_delta", serde_json::json!({"text":text}));
        }
    }
}

fn emit_usage(observer: &Arc<dyn RunObserver>, usage: Value) {
    if !usage.is_object() {
        return;
    }
    let input = usage["input_tokens"]
        .as_u64()
        .or_else(|| usage["input"].as_u64());
    let output = usage["output_tokens"]
        .as_u64()
        .or_else(|| usage["output"].as_u64());
    let total = usage["total_tokens"]
        .as_u64()
        .or_else(|| usage["total"].as_u64())
        .or_else(|| input.zip(output).map(|(a, b)| a.saturating_add(b)));
    let mut public =
        serde_json::json!({"input_tokens":input,"output_tokens":output,"total_tokens":total});
    for key in [
        "cached_input_tokens",
        "cache_read_input_tokens",
        "cache_write_input_tokens",
        "cache_creation_input_tokens",
        "reasoning_output_tokens",
    ] {
        if let Some(count) = usage[key].as_u64() {
            public[key] = serde_json::json!(count);
        }
    }
    observer.emit("usage", public);
}

fn observed_intents(output: &[u8], harness: &str) -> Vec<ActionIntent> {
    let mut intents = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in output.split(|b| *b == b'\n') {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let block = match harness {
            "codex" if value["type"] == "item.completed" || value["type"] == "item.started" => {
                &value["item"]
            }
            "claude-code"
                if value.pointer("/event/type").and_then(Value::as_str)
                    == Some("content_block_start") =>
            {
                &value["event"]["content_block"]
            }
            "opencode" if value["type"] == "tool_use" => &value["part"],
            _ => continue,
        };
        let kind = block["type"].as_str().unwrap_or("");
        let name = block["name"]
            .as_str()
            .or_else(|| block["tool"].as_str())
            .unwrap_or(kind);
        if !matches!(
            kind,
            "command_execution"
                | "mcp_tool_call"
                | "file_change"
                | "web_search"
                | "tool_use"
                | "tool"
        ) && harness != "opencode"
        {
            continue;
        }
        let identity = block["id"]
            .as_str()
            .or_else(|| block["callID"].as_str())
            .map(str::to_string)
            .unwrap_or_else(|| block.to_string());
        if !seen.insert(identity) {
            continue;
        }
        let target = block["command"]
            .as_str()
            .or_else(|| block.pointer("/input/file_path").and_then(Value::as_str))
            .unwrap_or("native_workspace");
        intents.push(
            ActionIntent::new(
                new_id("intent"),
                name.to_string(),
                target.to_string(),
                block.clone(),
                RiskLevel::High,
            )
            .with_assurance(Assurance::ProviderGoverned),
        );
    }
    intents
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Observer {
        events: std::sync::Mutex<Vec<(String, Value)>>,
        cancel: std::sync::atomic::AtomicBool,
    }
    impl RunObserver for Observer {
        fn emit(&self, kind: &str, value: Value) {
            self.events.lock().unwrap().push((kind.into(), value));
        }
        fn is_cancelled(&self) -> bool {
            self.cancel.load(std::sync::atomic::Ordering::Relaxed)
        }
    }
    #[test]
    fn native_messages_tools_and_unknown_logs() {
        let captured = Arc::new(Observer::default());
        let observer: Arc<dyn RunObserver> = captured.clone();
        let mut parts = HashMap::new();
        let mut answer = String::new();
        for value in [
            serde_json::json!({"type":"item.updated","item":{"type":"agent_message","id":"a","text":"Hello"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"a","text":"Hello world"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"command_execution","id":"tool","command":"echo test","aggregated_output":"test"}}),
        ] {
            normalize(&value, "codex", &observer, &mut parts, &mut answer);
        }
        assert_eq!(answer, "Hello world");
        assert!(captured
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "tool_output"));
        let count = captured.events.lock().unwrap().len();
        normalize(
            &serde_json::json!({"type":"log","text":"thinking"}),
            "codex",
            &observer,
            &mut parts,
            &mut answer,
        );
        assert_eq!(captured.events.lock().unwrap().len(), count);
        let intents=observed_intents(br#"{"type":"item.completed","item":{"type":"command_execution","id":"tool","command":"echo test"}}"#,"codex");
        assert_eq!(intents.len(), 1);
        emit_usage(
            &observer,
            serde_json::json!({"input_tokens":2,"output_tokens":1,"access_token":"private-value","reasoning":"private chain"}),
        );
        let events = captured.events.lock().unwrap();
        let usage = &events.last().unwrap().1;
        assert_eq!(usage["total_tokens"], 3);
        assert!(usage.get("access_token").is_none());
        assert!(usage.get("reasoning").is_none());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn process_drains_both_pipes_and_stop_kills_process() {
        let observer = Arc::new(Observer::default());
        let mut command = Command::new("powershell");
        command.args(["-NoProfile","-Command",r#"[Console]::Out.WriteLine('{"type":"item.completed","item":{"type":"agent_message","id":"a","text":"fixture"}}'); [Console]::Error.WriteLine('stderr fixture')"#]);
        let result =
            run_observed_process(command, observer.clone(), "codex", Duration::from_secs(60))
                .await
                .unwrap();
        assert_eq!(result.stdout, "fixture");
        assert!(result.stderr.contains("stderr fixture"));
        let mut failing = Command::new("powershell");
        failing.args(["-NoProfile","-Command",r#"[Console]::Out.WriteLine('{"type":"item.completed","item":{"type":"agent_message","id":"a","text":"partial fixture"}}'); exit 7"#]);
        let failed =
            run_observed_process(failing, observer.clone(), "codex", Duration::from_secs(10))
                .await
                .unwrap();
        assert!(!failed.success);
        assert_eq!(failed.exit_code, Some(7));
        assert_eq!(failed.stdout, "partial fixture");
        let mut timed = Command::new("powershell");
        timed.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
        let timed = tokio::time::timeout(
            Duration::from_secs(5),
            run_observed_process(timed, observer.clone(), "codex", Duration::from_millis(100)),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(!timed.success);
        assert!(timed.stderr.contains("timeout"));
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            "while($true) { [Console]::Out.WriteLine('{}'); Start-Sleep -Milliseconds 10 }",
        ]);
        let cancel = observer.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            run_observed_process(command, observer, "codex", Duration::from_secs(60)),
        )
        .await
        .unwrap();
        let result = result.unwrap();
        assert!(!result.success);
        assert!(result.stderr.contains("cancelled"));
    }
}
