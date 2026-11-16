use crate::harness::spec::StreamProtocol;

use custos_core::contracts::harness::{HarnessExecutionResult, RunObserver, ToolMediationLevel};
use custos_domain::DomainError;
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

pub async fn run_observed_process_with_protocol(
    mut command: Command,
    observer: Arc<dyn RunObserver>,
    protocol: StreamProtocol,
    timeout: Duration,
) -> Result<HarnessExecutionResult, DomainError> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|e| DomainError::Validation(format!("Failed to spawn native harness: {e}")))?;
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
            chunk = rx.recv() => {
                let Some((stderr, bytes)) = chunk else { break; };
                let target = if stderr { &mut errors } else { &mut output };
                if target.len().saturating_add(bytes.len()) > 2 * 1024 * 1024 {
                    cancelled = true;
                    break;
                }
                target.extend_from_slice(&bytes);
                if !stderr {
                    pending.extend_from_slice(&bytes);
                    while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                        let line: Vec<_> = pending.drain(..=end).collect();
                        if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                            super::normalize(&value, protocol, &observer, &mut parts, &mut answer);
                        } else if protocol == StreamProtocol::PlainText {
                            let text = String::from_utf8_lossy(&line);
                            if !text.is_empty() {
                                observer.emit("text_delta", serde_json::json!({"text": text}));
                            }
                        }
                    }
                }
            },
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                if observer.is_cancelled() || tokio::time::Instant::now() >= deadline {
                    cancelled = true;
                    break;
                }
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
        #[cfg(unix)]
        if let Some(pid) = child.id() {
            // Kill entire process group to prevent orphan zombie CLI processes
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        let _ = child.kill().await;
    }
    let status = loop {
        tokio::select! {
            result = child.wait() => break result.map_err(|e| DomainError::Validation(format!("Native wait failed: {e}")))?,
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                if observer.is_cancelled() || tokio::time::Instant::now() >= deadline {
                    cancelled = true;
                    #[cfg(windows)]
                    if let Some(pid) = child.id() {
                        let _ = Command::new("taskkill")
                            .args(["/PID", &pid.to_string(), "/T", "/F"])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .await;
                    }
                    #[cfg(unix)]
                    if let Some(pid) = child.id() {
                        unsafe {
                            libc::kill(-(pid as i32), libc::SIGKILL);
                        }
                    }
                    let _ = child.kill().await;
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
            super::normalize(&value, protocol, &observer, &mut parts, &mut answer);
        } else if protocol == StreamProtocol::PlainText {
            let text = String::from_utf8_lossy(&pending);
            if !text.is_empty() {
                observer.emit("text_delta", serde_json::json!({"text": text}));
            }
        }
    }
    Ok(HarnessExecutionResult {
        success: !cancelled && status.success(),
        exit_code: status.code(),
        stdout: if answer.is_empty() {
            String::from_utf8_lossy(&output).into_owned()
        } else {
            answer
        },
        stderr: String::from_utf8_lossy(&errors).into_owned(),
        observed_effects: super::intents::extract_observed_intents(
            &String::from_utf8_lossy(&output),
            "native",
            "native_turn",
        ),
        mediation_level: ToolMediationLevel::ProviderGoverned,
    })
}

pub async fn run_observed_process(
    command: Command,
    observer: Arc<dyn RunObserver>,
    harness: &str,
    timeout: Duration,
) -> Result<HarnessExecutionResult, DomainError> {
    let protocol = match harness {
        "claude-code" => StreamProtocol::ClaudeJson,
        "codex" => StreamProtocol::CodexJson,
        "opencode" => StreamProtocol::OpenCodeJson,
        _ => StreamProtocol::PlainText,
    };
    run_observed_process_with_protocol(command, observer, protocol, timeout).await
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

    #[cfg(unix)]
    #[tokio::test]
    async fn test_unix_cancel_stops_process_and_no_events_after_ack() {
        let observer = Arc::new(Observer::default());
        let mut command = Command::new("sh");
        command.args(["-c", "while true; do echo '{\"type\":\"item.updated\",\"item\":{\"type\":\"agent_message\",\"id\":\"a\",\"text\":\"tick\"}}'; sleep 0.05; done"]);

        let cancel = observer.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            cancel
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            run_observed_process(command, observer.clone(), "codex", Duration::from_secs(10)),
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!result.success);
        let count_at_cancel = observer.events.lock().unwrap().len();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let count_after_wait = observer.events.lock().unwrap().len();
        assert_eq!(
            count_at_cancel, count_after_wait,
            "No events must be emitted after cancellation"
        );
    }
}
