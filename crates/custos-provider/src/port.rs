//! Provider Port Trait
//!
//! Contract defining how the Custos runtime interacts with AI model providers.

use crate::events::ProviderEvent;
use crate::request::{ModelResponse, ProviderRequest};
use async_trait::async_trait;
use custos_domain::DomainError;
use tokio::sync::mpsc;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Unique provider identifier (e.g., "fake", "claude", "codex", "antigravity").
    fn provider_id(&self) -> &str;

    /// Unary synchronous generation call.
    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError>;

    /// Streaming generation returning an asynchronous receiver of ProviderEvents.
    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(4);
        let resp = self.generate(req).await?;
        let req_id = req.request_id.clone();
        tokio::spawn(async move {
            let _ = tx
                .send(ProviderEvent::completed(
                    req_id,
                    Some(resp.content),
                    Some(crate::events::TokenUsage {
                        input_tokens: None,
                        output_tokens: Some(resp.tokens_used),
                    }),
                ))
                .await;
        });
        Ok(rx)
    }
}

/// Canonical Hexagonal Architecture alias for ModelProvider
pub use ModelProvider as ModelPort;

/// Agent Runtime Port (Deep Dissection of Agent Loop)
///
/// Dissects the legacy upstream execution loop into a strictly governed contract.
/// Concrete agent engines (including dissected Goose loops) implement this trait.
#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    /// Executes a single agent step or full worker turn under the governed WorkerRun contract
    async fn execute_turn(
        &self,
        worker_run: &custos_domain::WorkerRun,
        context_pack: &custos_domain::ContextPack,
    ) -> Result<Vec<custos_domain::ActionIntent>, DomainError>;
}

/// Capability Dispatch Port
///
/// Port for executing physical effects (MCP, shell, local filesystem)
/// strictly governed by ExecutionPermits issued by the Security Kernel.
#[async_trait]
pub trait CapabilityPort: Send + Sync {
    /// Identifier for this capability (e.g. "fs_read", "fs_write", "shell", "mcp:git")
    fn capability_name(&self) -> &str;

    /// Dispatches an action intent that has been authorized with an ExecutionPermit
    async fn dispatch(
        &self,
        action: &custos_domain::ActionIntent,
        permit: &custos_domain::ExecutionPermit,
    ) -> Result<custos_domain::ExecutionReceipt, DomainError>;
}

