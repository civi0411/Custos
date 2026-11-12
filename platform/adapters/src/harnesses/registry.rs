//! Harness Registry for Native Coding and Governed Agent Runtimes
//!
//! Provides discovery, availability probe, and lifecycle dispatch across:
//! - Claude Code CLI (`claude-code`)
//! - OpenAI Codex (`codex`)
//! - Block Goose (`goose`)
//! - Custos Governed Agent Runtime (`governed-agent-runtime`)

use custos_core::contracts::harness::{
    AgentRuntimePort, HarnessDescriptor, HarnessExecutionResult, HarnessMaturity,
};
use custos_domain::DomainError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct HarnessEntry {
    pub name: String,
    pub description: String,
    pub binary_path: String,
    pub launch_cmd: String,
    pub maturity: HarnessMaturity,
    pub harness: Arc<dyn AgentRuntimePort>,
}

pub use super::binary::{probe_binary_available, resolve_binary_path};

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
    /// Wraps all first-party and community AI coding agents parity-matched with Orca's catalog.
    pub fn default_with_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let root = workspace_root.into();
        let mut registry = Self::new();

        for spec in super::catalog::SPECS.iter() {
            let adapter = Arc::new(super::adapter::CliHarness::from_spec(spec, root.clone()));
            registry.register_with_maturity(
                adapter,
                spec.name.as_str(),
                spec.desc.as_str(),
                spec.bins.as_str(),
                spec.launch_cmd.as_str(),
                spec.maturity,
            );
        }

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
        let bin = binary_path.into();
        self.register_with_launch_cmd(harness, name, description, bin.clone(), bin);
    }

    pub fn register_with_launch_cmd(
        &mut self,
        harness: Arc<dyn AgentRuntimePort>,
        name: impl Into<String>,
        description: impl Into<String>,
        binary_path: impl Into<String>,
        launch_cmd: impl Into<String>,
    ) {
        self.register_with_maturity(
            harness,
            name,
            description,
            binary_path,
            launch_cmd,
            HarnessMaturity::DiscoveredCli,
        );
    }

    pub fn register_with_maturity(
        &mut self,
        harness: Arc<dyn AgentRuntimePort>,
        name: impl Into<String>,
        description: impl Into<String>,
        binary_path: impl Into<String>,
        launch_cmd: impl Into<String>,
        maturity: HarnessMaturity,
    ) {
        let id = harness.harness_id().to_string();
        let entry = Arc::new(HarnessEntry {
            name: name.into(),
            description: description.into(),
            binary_path: binary_path.into(),
            launch_cmd: launch_cmd.into(),
            maturity,
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

    /// Returns the composed runtimes keyed by their stable harness identifiers.
    /// The runtime receives only the abstract port; registry metadata and binary
    /// probing remain adapter concerns.
    pub fn runtimes(&self) -> HashMap<String, Arc<dyn AgentRuntimePort>> {
        self.harnesses
            .iter()
            .map(|(id, entry)| (id.clone(), entry.harness.clone()))
            .collect()
    }

    pub fn get_descriptor(&self, id: &str) -> Option<HarnessDescriptor> {
        self.harnesses.get(id).map(|e| {
            let profile = e.harness.profile();
            let is_available = probe_binary_available(&e.binary_path);
            HarnessDescriptor {
                id: profile.harness_id.clone(),
                name: e.name.clone(),
                description: e.description.clone(),
                binary_path: e.binary_path.clone(),
                is_available,
                launch_cmd: Some(e.launch_cmd.clone()),
                maturity: e.maturity,
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
            DomainError::Validation(format!(
                "Harness '{id}' is not registered in HarnessRegistry"
            ))
        })?;
        harness.run_native(instruction, cwd).await
    }

    pub async fn cancel(&self, id: &str, run_id: &str) -> Result<(), DomainError> {
        let harness = self.get(id).ok_or_else(|| {
            DomainError::Validation(format!(
                "Harness '{id}' is not registered in HarnessRegistry"
            ))
        })?;
        harness.cancel_run(run_id).await
    }

    pub async fn steer(&self, id: &str, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        let harness = self.get(id).ok_or_else(|| {
            DomainError::Validation(format!(
                "Harness '{id}' is not registered in HarnessRegistry"
            ))
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
        assert_eq!(harnesses.len(), 45);

        let ids: Vec<&str> = harnesses.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"claude-agent-teams"));
        assert!(ids.contains(&"openclaude"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"grok"));
        assert!(ids.contains(&"copilot"));
        assert!(ids.contains(&"goose"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"opencode2"));
        assert!(ids.contains(&"antigravity"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"aider"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"cline"));
        assert!(ids.contains(&"kimi"));
        assert!(ids.contains(&"trae"));
        assert!(ids.contains(&"pi"));
        assert!(ids.contains(&"omp"));
        assert!(ids.contains(&"dsh"));
        assert!(ids.contains(&"devin"));

        assert_eq!(registry.default_harness_id(), Some("claude-code"));

        let opencode = registry.get("opencode").expect("OpenCode must be found");
        assert_eq!(opencode.harness_id(), "opencode");

        let runtimes = registry.runtimes();
        assert_eq!(runtimes.len(), 45);
        assert!(runtimes.contains_key("codex"));

        let desc = registry
            .get_descriptor("opencode")
            .expect("OpenCode descriptor");
        assert_eq!(desc.id, "opencode");
        assert_eq!(desc.name, "OpenCode");
    }
}
