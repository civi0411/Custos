//! Custos Trusted Daemon Entry Point

mod runtime;

use custos_observability::init_tracing;
use runtime::CustosRuntime;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    info!("Starting Custos Daemon (custosd) v4.0...");

    let runtime = CustosRuntime::bootstrap_in_memory()?;
    let task = runtime
        .task_service
        .create_task("Bootstrap daemon verification task".into())
        .await?;
    info!(
        "Created initial verification task: id={}, status={}",
        task.id, task.status
    );

    info!("Custos Daemon initialized successfully. Ready for IPC connections.");
    Ok(())
}
