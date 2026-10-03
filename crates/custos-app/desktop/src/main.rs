//! Custos Desktop GUI Entrypoint (Thin Presentation Client)
//!
//! Communicates strictly via `custos-sdk` over Unix Domain Sockets (`daemon.sock`).
//! Never connects directly to SQLite WAL persistence or runs internal FSM state logic.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing Custos Desktop Presentation Client...");
    println!("Custos Desktop GUI (Connected via custos-sdk)");
    Ok(())
}
