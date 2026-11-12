//! Governed Agent Runtime Implementation (Deep Dissection of Agent Loop)
//!
//! Bridges model inference and agent loop mechanics into the Sovereign Task Kernel
//! via `custos_provider::AgentRuntimePort`.
//!
//! Rather than executing tools in an unconstrained wild loop, candidate tool calls
//! are intercepted and transformed into `ActionIntent`s that require cryptographic
//! `ExecutionPermit`s from the Security Kernel (`custos-core`).

use custos_provider::ModelPort;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod tool;
pub mod writer;
mod loop_turn;

#[cfg(test)]
mod tests;

pub use custos_core::contracts::harness::AgentRuntimePort;
pub use tool::*;
pub use writer::*;

/// Maximum iterations in a multi-turn agent loop to prevent infinite loops.
pub const DEFAULT_MAX_ITERATIONS: usize = 10;

/// Maximum bytes for tool output before truncation (64 KiB) to guard against unbounded memory consumption.
pub const DEFAULT_MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;

pub struct GovernedAgentRuntime {
    provider: Arc<dyn ModelPort>,
    system_prompt: String,
    available_tools: Vec<serde_json::Value>,
    tool_executor: Option<Arc<dyn GovernedToolExecutor>>,
    max_iterations: usize,
    max_tool_output_bytes: usize,
    cancelled_runs: Arc<RwLock<HashSet<String>>>,
    steer_messages: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl GovernedAgentRuntime {
    pub fn new(
        provider: Arc<dyn ModelPort>,
        system_prompt: impl Into<String>,
        available_tools: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            provider,
            system_prompt: system_prompt.into(),
            available_tools,
            tool_executor: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            max_tool_output_bytes: DEFAULT_MAX_TOOL_OUTPUT_BYTES,
            cancelled_runs: Arc::new(RwLock::new(HashSet::new())),
            steer_messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_tool_executor(mut self, executor: Arc<dyn GovernedToolExecutor>) -> Self {
        self.tool_executor = Some(executor);
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_tool_output_cap(mut self, cap: usize) -> Self {
        self.max_tool_output_bytes = cap;
        self
    }

    pub fn register_tool(&mut self, tool: serde_json::Value) {
        self.available_tools.push(tool);
    }

    pub fn with_tool(mut self, tool: serde_json::Value) -> Self {
        self.available_tools.push(tool);
        self
    }

    pub fn is_tool_enabled(&self) -> bool {
        !self.available_tools.is_empty()
    }
}
