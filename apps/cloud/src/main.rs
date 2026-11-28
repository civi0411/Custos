use custos_cloud::policy::{CloudConfig, CloudRole};
use custos_cloud::webhook::WebhookWakeSignal;
use custos_cloud::{CloudRuntime, NoopPushAdapter};
use custos_protocol::relay::{SpliceState, ZeroPiiPushCommand};
use serde::Deserialize;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum CloudCommand {
    Admit { session_id: String },
    Advance { session_id: String, state: SpliceState },
    Push { command: ZeroPiiPushCommand },
    Wake { signal: WebhookWakeSignal },
    DrainWakes { host_id: String },
    Counters,
}

fn parse_role_from_args() -> CloudRole {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--role") {
        if let Some(raw) = args.get(pos + 1) {
            return CloudRole::parse(raw).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(2);
            });
        }
    }
    std::env::var("CUSTOS_CLOUD_ROLE")
        .ok()
        .and_then(|r| CloudRole::parse(&r).ok())
        .unwrap_or(CloudRole::All)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = std::env::var("CUSTOS_CLOUD_BIND")
        .or_else(|_| std::env::var("CUSTOS_RELAY_BIND"))
        .unwrap_or_else(|_| "127.0.0.1:7878".into());
    let role = parse_role_from_args();
    let listener = TcpListener::bind(&bind_addr).await?;
    let runtime = Arc::new(CloudRuntime::new(
        CloudConfig {
            role,
            bind_addr: bind_addr.clone(),
        },
        Arc::new(NoopPushAdapter),
    ));
    println!("custos-cloud ({role:?}) listening on {bind_addr}");

    while let Ok((socket, _peer)) = listener.accept().await {
        let rt = runtime.clone();
        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let reply = match serde_json::from_str::<CloudCommand>(&line) {
                    Ok(CloudCommand::Admit { session_id }) => match rt.admit_session(session_id) {
                        Ok(state) => serde_json::json!({ "ok": true, "state": state }),
                        Err(err) => serde_json::json!({ "ok": false, "error": err }),
                    },
                    Ok(CloudCommand::Advance { session_id, state }) => {
                        match rt.advance_session(&session_id, state) {
                            Ok(next) => serde_json::json!({ "ok": true, "state": next }),
                            Err(err) => serde_json::json!({ "ok": false, "error": err }),
                        }
                    }
                    Ok(CloudCommand::Push { command }) => match rt.notify_zero_pii(command).await {
                        Ok(()) => serde_json::json!({ "ok": true }),
                        Err(err) => serde_json::json!({ "ok": false, "error": err }),
                    },
                    Ok(CloudCommand::Wake { signal }) => match rt.enqueue_wake(signal) {
                        Ok(enqueued) => serde_json::json!({ "ok": true, "enqueued": enqueued }),
                        Err(err) => serde_json::json!({ "ok": false, "error": err }),
                    },
                    Ok(CloudCommand::DrainWakes { host_id }) => {
                        let wakes = rt.drain_host_wakes(&host_id);
                        serde_json::json!({ "ok": true, "wakes": wakes })
                    }
                    Ok(CloudCommand::Counters) => {
                        let (sent, failed, suppressed) = rt.push_counters().snapshot();
                        serde_json::json!({ "ok": true, "sent": sent, "failed": failed, "suppressed": suppressed })
                    }
                    Err(err) => serde_json::json!({ "ok": false, "error": err.to_string() }),
                };
                let mut out = reply.to_string();
                out.push('\n');
                if writer.write_all(out.as_bytes()).await.is_err() {
                    break;
                }
            }
        });
    }
    Ok(())
}

