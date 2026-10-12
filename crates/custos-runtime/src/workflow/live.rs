//! Bounded ephemeral run projections. Durable outcomes remain in RunPort.
use custos_core::contracts::harness::RunObserver;
use custos_domain::{
    DomainError, LiveRunEvent, LiveRunReplay, LiveRunSnapshot, Run, StartRunCommand,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

const MAX_EVENTS: usize = 1000;
const MAX_BYTES: usize = 2 * 1024 * 1024;
const RETENTION: Duration = Duration::from_secs(300);

struct Projection {
    snapshot: LiveRunSnapshot,
    events: VecDeque<(LiveRunEvent, usize)>,
    bytes: usize,
    terminal_at: Option<Instant>,
    cancel: CancellationToken,
}

#[derive(Clone, Default)]
pub struct LiveRuns(Arc<Mutex<HashMap<String, Projection>>>);

impl LiveRuns {
    pub fn register(
        &self,
        run: &Run,
        cmd: &StartRunCommand,
    ) -> Result<CancellationToken, DomainError> {
        let mut runs = self
            .0
            .lock()
            .map_err(|_| DomainError::Validation("Live projection lock unavailable".into()))?;
        runs.retain(|_, p| p.terminal_at.is_none_or(|time| time.elapsed() < RETENTION));
        if runs.len() >= 256 {
            return Err(DomainError::Validation(
                "Too many retained live runs".into(),
            ));
        }
        let cancel = CancellationToken::new();
        runs.insert(
            run.id.clone(),
            Projection {
                snapshot: LiveRunSnapshot {
                    cursor: 0,
                    run_id: run.id.clone(),
                    task_id: run.task_id.clone(),
                    session_id: cmd.session_id.clone().unwrap_or_default(),
                    turn_id: cmd
                        .turn_id
                        .clone()
                        .or_else(|| {
                            run.metadata
                                .get("turn_id")
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| run.id.clone()),
                    status: "queued".into(),
                    text: String::new(),
                    activities: vec![],
                    usage: None,
                    error: None,
                },
                events: VecDeque::new(),
                bytes: 0,
                terminal_at: None,
                cancel: cancel.clone(),
            },
        );
        Ok(cancel)
    }

    pub fn emit(&self, run_id: &str, kind: &str, mut payload: serde_json::Value) {
        let Ok(mut runs) = self.0.lock() else {
            return;
        };
        let Some(p) = runs.get_mut(run_id) else {
            return;
        };
        if p.terminal_at.is_some() {
            return;
        }
        if kind == "status"
            && p.snapshot.cursor > 0
            && payload.get("status").and_then(|v| v.as_str()) == Some(p.snapshot.status.as_str())
        {
            return;
        }

        if matches!(kind, "tool_input" | "tool_output" | "reasoning_summary") {
            payload = sanitize_activity(payload);
            if let Ok(serialized) = serde_json::to_string(&payload) {
                if serialized.len() > 16 * 1024 {
                    let mut end = (8 * 1024 - 256).min(serialized.len());
                    while !serialized.is_char_boundary(end) {
                        end = end.saturating_sub(1);
                    }
                    payload = serde_json::json!({"preview":format!("{} [truncated]",&serialized[..end]),"truncated":true});
                }
            }
            p.snapshot
                .activities
                .push(serde_json::json!({"kind":kind,"payload":payload}));
            if p.snapshot.activities.len() > 100 {
                p.snapshot.activities.remove(0);
            }
        }
        if kind == "text_delta" {
            if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                if p.snapshot.text.len().saturating_add(text.len()) <= MAX_BYTES {
                    p.snapshot.text.push_str(text);
                } else {
                    p.cancel.cancel();
                    return;
                }
            }
        }
        if kind == "status" {
            if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
                p.snapshot.status = status.to_string();
            }
        }
        if kind == "usage" {
            p.snapshot.usage = Some(payload.clone());
        }
        if kind == "result" {
            p.snapshot.status = payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("failed")
                .into();
            if let Some(text) = payload
                .get("text")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                let mut end = text.len().min(MAX_BYTES);
                while !text.is_char_boundary(end) {
                    end = end.saturating_sub(1);
                }
                p.snapshot.text = text[..end].to_string();
            }
            p.snapshot.error = payload
                .get("error")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            p.terminal_at = Some(Instant::now());
        }
        p.snapshot.cursor += 1;
        let event = LiveRunEvent {
            id: p.snapshot.cursor,
            session_id: p.snapshot.session_id.clone(),
            turn_id: p.snapshot.turn_id.clone(),
            task_id: p.snapshot.task_id.clone(),
            run_id: run_id.into(),
            kind: kind.into(),
            payload,
        };
        let size = serde_json::to_vec(&event)
            .map(|v| v.len())
            .unwrap_or(MAX_BYTES + 1);
        if size > MAX_BYTES {
            p.events.clear();
            p.bytes = 0;
            return;
        }
        p.bytes += size;
        p.events.push_back((event, size));
        while p.events.len() > MAX_EVENTS || p.bytes > MAX_BYTES {
            if let Some((_, size)) = p.events.pop_front() {
                p.bytes = p.bytes.saturating_sub(size);
            }
        }
    }

    pub fn replay(
        &self,
        session: &str,
        run: &str,
        cursor: u64,
    ) -> Result<LiveRunReplay, DomainError> {
        let mut runs = self
            .0
            .lock()
            .map_err(|_| DomainError::Validation("Live projection lock unavailable".into()))?;
        runs.retain(|_, p| p.terminal_at.is_none_or(|time| time.elapsed() < RETENTION));
        let p = runs.get(run).ok_or_else(|| DomainError::NotFound {
            kind: "LiveRun".into(),
            id: run.into(),
        })?;
        if p.snapshot.session_id != session {
            return Err(DomainError::Validation(
                "Run does not belong to this session".into(),
            ));
        }
        let first = p
            .events
            .front()
            .map(|(e, _)| e.id)
            .unwrap_or(p.snapshot.cursor + 1);
        let gap = cursor > p.snapshot.cursor || cursor.saturating_add(1) < first;
        Ok(LiveRunReplay {
            events: if gap {
                vec![]
            } else {
                p.events
                    .iter()
                    .filter(|(e, _)| e.id > cursor)
                    .map(|(e, _)| e.clone())
                    .collect()
            },
            snapshot: p.snapshot.clone(),
            gap,
        })
    }

    pub fn cancel(&self, run: &str) -> bool {
        let Ok(runs) = self.0.lock() else {
            return false;
        };
        if let Some(p) = runs.get(run).filter(|p| p.terminal_at.is_none()) {
            p.cancel.cancel();
            true
        } else {
            false
        }
    }

    pub fn observer(&self, run: &str, token: CancellationToken) -> Arc<dyn RunObserver> {
        Arc::new(LiveObserver {
            runs: self.clone(),
            run: run.into(),
            token,
        })
    }
}

pub struct LiveObserver {
    runs: LiveRuns,
    run: String,
    token: CancellationToken,
}
impl RunObserver for LiveObserver {
    fn emit(&self, kind: &str, payload: serde_json::Value) {
        self.runs.emit(&self.run, kind, payload);
    }
    fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

fn sanitize_activity(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let lower = key.to_lowercase();
                    let secret = [
                        "token",
                        "secret",
                        "password",
                        "authorization",
                        "api_key",
                        "apikey",
                        "cookie",
                        "partial_json",
                    ]
                    .iter()
                    .any(|s| lower.contains(s));
                    (
                        key,
                        if secret {
                            serde_json::json!("[redacted]")
                        } else {
                            sanitize_activity(value)
                        },
                    )
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            let truncated = values.len() > 50;
            let items: Vec<_> = values.into_iter().take(50).map(sanitize_activity).collect();
            if truncated {
                serde_json::json!({"items":items,"truncated":true})
            } else {
                serde_json::Value::Array(items)
            }
        }
        serde_json::Value::String(text) => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if value.is_object() || value.is_array() {
                    let safe = sanitize_activity(value);
                    return serde_json::Value::String(
                        serde_json::to_string(&safe).unwrap_or_else(|_| "[redacted]".into()),
                    );
                }
            }
            let lower = text.to_lowercase();
            let sensitive = lower.contains("bearer ")
                || lower.contains("authorization:")
                || lower.contains("access_token=")
                || lower.contains("password=")
                || lower.contains("token=")
                || lower.contains("api-key:")
                || text.split_whitespace().any(|word| {
                    let word = word.trim_matches(['\'', '"', '[', ']', '(', ')']);
                    word.starts_with("AIza")
                        || word.starts_with("ya29.")
                        || word.starts_with("github_pat_")
                        || word.starts_with("gho_")
                        || word.starts_with("sk-")
                        || word.starts_with("ghp_")
                        || word.starts_with("eyJ")
                        || word.eq_ignore_ascii_case("bearer")
                        || word.to_lowercase().contains("api_key=")
                });
            if sensitive {
                serde_json::json!("[redacted sensitive activity]")
            } else if text.len() > 4096 {
                serde_json::json!(format!(
                    "{} [truncated]",
                    text.chars().take(1024).collect::<String>()
                ))
            } else {
                serde_json::json!(text)
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (LiveRuns, Run, CancellationToken) {
        let live = LiveRuns::default();
        let run = Run::new("task".into(), 1);
        let mut cmd = StartRunCommand::new("task", "chat");
        cmd.session_id = Some("session".into());
        cmd.turn_id = Some("turn".into());
        let token = live.register(&run, &cmd).unwrap();
        (live, run, token)
    }
    #[test]
    fn replay_scope_cursor_and_terminal_are_consistent() {
        let (live, run, _) = setup();
        live.emit(&run.id, "text_delta", serde_json::json!({"text":"hello"}));
        live.emit(&run.id, "text_delta", serde_json::json!({"text":" world"}));
        let replay = live.replay("session", &run.id, 1).unwrap();
        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.events[0].id, 2);
        assert_eq!(replay.events[0].turn_id, "turn");
        assert_eq!(replay.snapshot.text, "hello world");
        assert!(live.replay("other", &run.id, 0).is_err());
        live.emit(
            &run.id,
            "result",
            serde_json::json!({"status":"completed","text":"canonical"}),
        );
        live.emit(&run.id, "text_delta", serde_json::json!({"text":"ignored"}));
        assert_eq!(
            live.replay("session", &run.id, 0).unwrap().snapshot.text,
            "canonical"
        );
        assert!(!live.cancel(&run.id));
    }
    #[test]
    fn eviction_requires_snapshot_and_cancel_preserves_partial() {
        let (live, run, token) = setup();
        for _ in 0..1001 {
            live.emit(&run.id, "text_delta", serde_json::json!({"text":"x"}));
        }
        let replay = live.replay("session", &run.id, 0).unwrap();
        assert!(replay.gap);
        assert!(replay.events.is_empty());
        assert_eq!(replay.snapshot.text.len(), 1001);
        assert_eq!(
            live.replay("session", &run.id, 1).unwrap().events.len(),
            1000
        );
        assert!(live.cancel(&run.id));
        assert!(token.is_cancelled());
    }
    #[test]
    fn activity_is_redacted_bounded_and_terminal_expires() {
        let (live, run, _) = setup();
        live.emit(&run.id,"tool_input",serde_json::json!({"authorization":"secret","command":"Bearer abc","output":"x".repeat(5000)}));
        let snapshot = live.replay("session", &run.id, 0).unwrap().snapshot;
        assert_eq!(
            snapshot.activities[0]["payload"]["authorization"],
            "[redacted]"
        );
        assert!(snapshot.activities[0]["payload"]["output"]
            .as_str()
            .unwrap()
            .contains("[truncated]"));
        live.emit(&run.id,"tool_input",serde_json::json!({"partial_json":"private-fragment","text":"{\"api_key\":\"private-json-secret\"}"}));
        let replay = live.replay("session", &run.id, 0).unwrap();
        let serialized = serde_json::to_string(&replay).unwrap();
        assert!(!serialized.contains("private-fragment"));
        assert!(!serialized.contains("private-json-secret"));
        live.emit(&run.id, "result", serde_json::json!({"status":"failed"}));
        live.0.lock().unwrap().get_mut(&run.id).unwrap().terminal_at =
            Some(Instant::now() - RETENTION);
        assert!(live.replay("session", &run.id, 0).is_err());
    }
}
