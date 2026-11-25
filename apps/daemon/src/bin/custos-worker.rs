//! Custos optional worker binary — receives one operation via stdin, returns bounded output via stdout.
//! Zero access to Host SQLite, zero access to Vault. Bounded standard I/O execution.

use custos_protocol::worker::{WorkerObservation, WorkerOperation};
use std::io::{self, Read, Write};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut stdin_buf = Vec::new();
    io::stdin().read_to_end(&mut stdin_buf)?;

    if stdin_buf.is_empty() {
        let obs = WorkerObservation::failure("empty", "No operation received on stdin", 0);
        let out = serde_json::to_vec(&obs)?;
        io::stdout().write_all(&out)?;
        return Ok(());
    }

    let op: WorkerOperation = match serde_json::from_slice(&stdin_buf) {
        Ok(op) => op,
        Err(e) => {
            let obs = WorkerObservation::failure("malformed", format!("JSON parse error: {e}"), 0);
            let out = serde_json::to_vec(&obs)?;
            io::stdout().write_all(&out)?;
            return Ok(());
        }
    };

    // Execute requested domain operation without DB/vault access
    let obs = match op.operation_kind.as_str() {
        "ping" | "healthcheck" => {
            let wall_ms = start.elapsed().as_millis() as u64;
            WorkerObservation::success(
                &op.operation_id,
                serde_json::json!({
                    "status": "healthy",
                    "component_id": op.component_id,
                    "version": op.version,
                }),
                wall_ms,
                0,
            )
        }
        "echo" => {
            let wall_ms = start.elapsed().as_millis() as u64;
            WorkerObservation::success(&op.operation_id, op.payload.clone(), wall_ms, 0)
        }
        "research.embed_text" => {
            let text = op
                .payload
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let wall_ms = start.elapsed().as_millis() as u64;
            let embedding = vec![0.1f32, 0.2, 0.3, text.len() as f32 / 100.0];
            WorkerObservation::success(
                &op.operation_id,
                serde_json::json!({ "embedding": embedding, "length": text.len() }),
                wall_ms,
                0,
            )
        }
        "quant.verify_sharpe" => {
            let sharpe = op
                .payload
                .get("sharpe")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let passed = sharpe > 1.0;
            let wall_ms = start.elapsed().as_millis() as u64;
            WorkerObservation::success(
                &op.operation_id,
                serde_json::json!({ "passed": passed, "sharpe": sharpe }),
                wall_ms,
                0,
            )
        }
        other => {
            let wall_ms = start.elapsed().as_millis() as u64;
            WorkerObservation::failure(
                &op.operation_id,
                format!("Unsupported worker operation kind: {other}"),
                wall_ms,
            )
        }
    };

    let out = serde_json::to_vec(&obs)?;
    io::stdout().write_all(&out)?;
    io::stdout().flush()?;
    Ok(())
}
