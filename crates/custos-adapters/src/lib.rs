//! # Custos Adapters Crate (`custos-adapters`)
//!
//! Concrete I/O and external platform adapter implementations in Layer 2 of the
//! Custos Architecture.
//!
//! This crate owns all concrete boundary interactions:
//! - Model connections & vendor wire transports (cloud and local)
//! - Operating system sandboxing and process containment (Seatbelt, Bubblewrap)
//! - Native agent harnesses and terminal PTY sessions
//! - External protocols: MCP, A2A, Jupyter/Python kernel
//! - Domain storage bridges: Worktrees, Market data streams, Packages, Remote Compute
//!
//! ## Architectural Boundaries (Rule 1)
//! - Implements contracts defined in `custos-core` and `custos-provider`.
//! - Does **NOT** decide Task completion or mint authority (owned by `custos-core`).
//! - Does **NOT** own top-level product routing or daemon startup (owned by `custos-daemon`).
//! - All untrusted inputs from the network, filesystem, or child processes MUST be
//!   validated at adapter boundaries before propagating inward.

// ============================================================================
// I. Inference & Model Intelligence Adapters
// ============================================================================

/// Remote cloud model providers (OpenAI, Anthropic, Gemini, Bedrock, Ollama, DeepSeek, etc.)
pub mod providers;

/// Native in-process model inference (llama.cpp, MLX, GGUF inspector, Metal probe)
pub mod local_inference;

/// Bounded model weights and artifact transfer from HuggingFace Hub / HTTPS
pub mod download_manager;

/// Model descriptor definitions and bridge types
pub mod model {
    pub use crate::local_inference::model::*;
    pub use custos_provider::types::model::*;
}

// Re-exports for inference engines
pub use local_inference::backend;
pub use local_inference::provider_utils::*;
pub use local_inference::tool_emulation;
pub use local_inference::tool_parsing;

// ============================================================================
// II. Host Environment, Sandboxing & Containment Adapters
// ============================================================================

/// Operating system boundary isolation (macOS Seatbelt, Linux Bubblewrap, Sovereign Developer mode)
pub mod sandbox;

/// Native POSIX / Windows Pseudo-Terminal (PTY) driver for live interactive shells
pub mod terminal;

/// Local folder and Git linked worktree lifecycle management
pub mod workspace;

/// External agent CLI harnesses (Aider, Claude CLI, SWE-bench runner, mock harness)
pub mod harness;

/// Canonical Git and GitHub identity alias used by daemon inspection
pub use workspace::github as repository_git;

// ============================================================================
// III. Engine & Communication Protocol Bridges
// ============================================================================

/// Model Context Protocol (MCP) clients, federation hub, security gates, and builtin tools
pub mod mcp;

/// Interactive Jupyter / Python kernel execution driver with memory bounds
pub mod notebook;

/// Standard Agent-to-Agent (A2A) JSON-RPC protocol and AgentCard discovery
pub mod a2a;

/// Peer-to-peer workspace roaming and sync over Iroh (optional feature)
#[cfg(feature = "roaming")]
pub mod roaming;

// ============================================================================
// IV. Domain Resource & Physical Storage Bridges
// ============================================================================

/// Reference-counted real-time and polling market data streams for Quant Workbench
pub mod market;

/// Remote cloud compute and GPU execution targets (Colab, Kaggle, Rented GPUs)
pub mod compute;

/// On-demand package binary distribution, SHA-256 verification, and staging promotion
pub mod packages;

/// Generic external tool I/O (source fetching, checkpoint CAS)
pub mod tools;

// Internal crate re-exports required by vendored provider client implementations
#[allow(unused_imports)]
pub(crate) use local_inference::*;
pub(crate) use providers::*;



