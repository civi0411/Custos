//! Harness Registry for Native Coding and Governed Agent Runtimes
//!
//! Provides discovery, availability probe, and lifecycle dispatch across:
//! - Claude Code CLI (`claude-code`)
//! - OpenAI Codex (`codex`)
//! - Block Goose (`goose`)
//! - Custos Governed Agent Runtime (`governed-agent-runtime`)

use custos_core::contracts::harness::{
    AgentRuntimePort, HarnessDescriptor, HarnessExecutionResult,
};
use custos_domain::DomainError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct HarnessEntry {
    pub name: String,
    pub description: String,
    pub binary_path: String,
    pub harness: Arc<dyn AgentRuntimePort>,
}

#[derive(Clone, Default)]
pub struct HarnessRegistry {
    harnesses: HashMap<String, Arc<HarnessEntry>>,
    default_harness_id: Option<String>,
}

impl HarnessRegistry {
    pub fn new() -> Self {
        Self {
            harnesses: HashMap::new(),
            default_harness_id: None,
        }
    }

    /// Creates a registry pre-populated with standard native harnesses configured for a workspace root.
    pub fn default_with_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let root = workspace_root.into();
        let mut registry = Self::new();

        let claude = Arc::new(super::claude_code::ClaudeCodeHarnessAdapter::new(root.clone()));
        registry.register(
            claude,
            "Claude Code CLI",
            "Anthropic Claude Code sub-process native coding harness.",
            "claude",
        );

        let codex = Arc::new(super::codex::CodexHarnessAdapter::new(root.clone()));
        registry.register(
            codex,
            "OpenAI Codex",
            "OpenAI Codex / GPT execution loop native coding harness.",
            "codex",
        );

        let goose = Arc::new(super::goose::GooseHarnessAdapter::new(root));
        registry.register(
            goose,
            "Block Goose",
            "Block Goose autonomous developer CLI native coding harness.",
            "goose",
        );

        registry.set_default("claude-code");
        registry
    }

    pub fn register(
        &mut self,
        harness: Arc<dyn AgentRuntimePort>,
        name: impl Into<String>,
        description: impl Into<String>,
        binary_path: impl Into<String>,
    ) {
        let id = harness.harness_id().to_string();
        let entry = Arc::new(HarnessEntry {
            name: name.into(),
            description: description.into(),
            binary_path: binary_path.into(),
            harness,
        });
        if self.default_harness_id.is_none() {
            self.default_harness_id = Some(id.clone());
        }
        self.harnesses.insert(id, entry);
    }

    pub fn set_default(&mut self, id: &str) {
        if self.harnesses.contains_key(id) {
            self.default_harness_id = Some(id.to_string());
        }
    }

    pub fn default_harness_id(&self) -> Option<&str> {
        self.default_harness_id.as_deref()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentRuntimePort>> {
        self.harnesses.get(id).map(|e| e.harness.clone())
    }

    pub fn get_descriptor(&self, id: &str) -> Option<HarnessDescriptor> {
        self.harnesses.get(id).map(|e| {
            let profile = e.harness.profile();
            let is_available = if e.binary_path == "internal" || e.binary_path.is_empty() {
                true
            } else {
                let bin = Path::new(&e.binary_path);
                bin.exists() || which::which(bin).is_ok()
            };
            HarnessDescriptor {
                id: profile.harness_id.clone(),
                name: e.name.clone(),
                description: e.description.clone(),
                binary_path: e.binary_path.clone(),
                is_available,
                profile,
            }
        })
    }

    pub fn list(&self) -> Vec<HarnessDescriptor> {
        let mut list: Vec<HarnessDescriptor> = self
            .harnesses
            .keys()
            .filter_map(|id| self.get_descriptor(id))
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub async fn run_native(
        &self,
        id: &str,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        let harness = self.get(id).ok_or_else(|| {
            DomainError::Validation(format!("Harness '{id}' is not registered in HarnessRegistry"))
        })?;
        harness.run_native(instruction, cwd).await
    }

    pub async fn cancel(&self, id: &str, run_id: &str) -> Result<(), DomainError> {
        let harness = self.get(id).ok_or_else(|| {
            DomainError::Validation(format!("Harness '{id}' is not registered in HarnessRegistry"))
        })?;
        harness.cancel_run(run_id).await
    }

    pub async fn steer(&self, id: &str, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        let harness = self.get(id).ok_or_else(|| {
            DomainError::Validation(format!("Harness '{id}' is not registered in HarnessRegistry"))
        })?;
        harness.steer_run(run_id, guidance).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_registry_resolution() {
        let registry = HarnessRegistry::default_with_workspace("/tmp");

        let harnesses = registry.list();
        assert_eq!(harnesses.len(), 3);

        let ids: Vec<&str> = harnesses.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"goose"));

        assert_eq!(registry.default_harness_id(), Some("claude-code"));

        let codex = registry.get("codex").expect("Codex must be found");
        assert_eq!(codex.harness_id(), "codex");

        let desc = registry.get_descriptor("goose").expect("Goose descriptor");
        assert_eq!(desc.id, "goose");
        assert_eq!(desc.name, "Block Goose");
    }
}
