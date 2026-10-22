use std::str::FromStr;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

#[derive(Clone, Debug)]
pub enum McpCommand {
    AutoVisualiser,
    Memory,
}

impl FromStr for McpCommand {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(' ', "").as_str() {
            "autovisualiser" | "visualiser" => Ok(McpCommand::AutoVisualiser),
            "memory" => Ok(McpCommand::Memory),
            _ => Err(format!("Invalid command: {}", s)),
        }
    }
}

impl McpCommand {
    pub fn name(&self) -> &str {
        match self {
            McpCommand::AutoVisualiser => "autovisualiser",
            McpCommand::Memory => "memory",
        }
    }
}

pub async fn serve<S>(server: S) -> Result<()>
where
    S: rmcp::ServerHandler,
{
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    Ok(())
}
