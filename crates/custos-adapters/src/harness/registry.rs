//! Harness Registry for Native Coding and Governed Agent Runtimes
//!
//! Provides discovery, availability probe, and lifecycle dispatch across:
//! - Claude Code CLI (`claude-code`)
//! - OpenAI Codex (`codex`)
//! - Block Goose (`goose`)
//! - Custos Governed Agent Runtime (`governed-agent-runtime`)

use custos_core::contracts::harness::{
    AgentRuntimePort, HarnessDescriptor, HarnessExecutionResult, HarnessMaturity, ToolMediationLevel,
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

pub fn probe_binary_available(binary_name: &str) -> bool {
    if binary_name == "internal" || binary_name.is_empty() {
        return true;
    }
    for sub in binary_name.split(|c| c == ',' || c == '|').map(str::trim) {
        let bin_name = sub.split_whitespace().next().unwrap_or(sub);
        let bin = Path::new(bin_name);
        if bin.exists() || which::which(bin).is_ok() {
            return true;
        }
        let candidates = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
        ];
        for dir in candidates {
            if Path::new(dir).join(bin).exists() {
                return true;
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if Path::new(&home).join(".cargo/bin").join(bin).exists() {
                return true;
            }
            if Path::new(&home).join(".local/bin").join(bin).exists() {
                return true;
            }
        }
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let cargo_bin = Path::new(&userprofile).join(".cargo/bin");
                if cargo_bin.join(bin).exists()
                    || cargo_bin.join(format!("{bin_name}.exe")).exists()
                    || cargo_bin.join(format!("{bin_name}.cmd")).exists()
                {
                    return true;
                }
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                let npm_bin = Path::new(&appdata).join("npm");
                if npm_bin.join(bin).exists()
                    || npm_bin.join(format!("{bin_name}.cmd")).exists()
                    || npm_bin.join(format!("{bin_name}.exe")).exists()
                {
                    return true;
                }
            }
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                let pnpm_bin = Path::new(&localappdata).join("pnpm");
                if pnpm_bin.join(bin).exists()
                    || pnpm_bin.join(format!("{bin_name}.cmd")).exists()
                    || pnpm_bin.join(format!("{bin_name}.exe")).exists()
                {
                    return true;
                }
            }
        }
    }
    false
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
    /// Wraps all first-party and community AI coding agents parity-matched with Orca's catalog.
    pub fn default_with_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let root = workspace_root.into();
        let mut registry = Self::new();

        // 1. Dedicated first-party harnesses (Verified Native with structured event conformance)
        let claude = Arc::new(super::claude_code::ClaudeCodeHarnessAdapter::new(
            root.clone(),
        ));
        registry.register_with_maturity(
            claude,
            "Claude Code",
            "Anthropic Claude Code sub-process native coding harness.",
            "claude",
            "claude",
            HarnessMaturity::VerifiedNative,
        );

        let codex = Arc::new(super::codex::CodexHarnessAdapter::new(root.clone()));
        registry.register_with_maturity(
            codex,
            "OpenAI Codex",
            "OpenAI Codex / GPT execution loop native coding harness.",
            "codex",
            "codex",
            HarnessMaturity::VerifiedNative,
        );

        let goose = Arc::new(super::goose::GooseHarnessAdapter::new(root.clone()));
        registry.register_with_maturity(
            goose,
            "Block Goose",
            "Block Goose autonomous developer CLI native coding harness.",
            "goose",
            "goose",
            HarnessMaturity::VerifiedNative,
        );

        let opencode = Arc::new(super::opencode::OpenCodeHarnessAdapter::new(root.clone()));
        registry.register_with_maturity(
            opencode,
            "OpenCode",
            "OpenCode native server/CLI agent loop with provider-governed execution.",
            "opencode",
            "opencode",
            HarnessMaturity::VerifiedNative,
        );

        // 2. Comprehensive catalog of all Orca-supported AI coding agents
        struct GenericAgentDef {
            id: &'static str,
            name: &'static str,
            desc: &'static str,
            bin: &'static str,
            launch_cmd: &'static str,
        }

        let catalog_agents = [
            GenericAgentDef {
                id: "claude-agent-teams",
                name: "Claude Agent Teams",
                desc: "Claude Code Agent Teams multi-pane orchestration.",
                bin: "claude",
                launch_cmd: "claude",
            },
            GenericAgentDef {
                id: "openclaude",
                name: "OpenClaude",
                desc: "Open-source Claude Code derivative agent.",
                bin: "openclaude",
                launch_cmd: "openclaude",
            },
            GenericAgentDef {
                id: "grok",
                name: "xAI Grok",
                desc: "xAI Grok CLI coding assistant.",
                bin: "grok",
                launch_cmd: "grok",
            },
            GenericAgentDef {
                id: "copilot",
                name: "GitHub Copilot",
                desc: "GitHub Copilot CLI coding assistant.",
                bin: "copilot",
                launch_cmd: "copilot",
            },
            GenericAgentDef {
                id: "opencode2",
                name: "OpenCode 2",
                desc: "OpenCode 2 standalone coding agent.",
                bin: "opencode2",
                launch_cmd: "opencode2 --standalone",
            },
            GenericAgentDef {
                id: "mimo-code",
                name: "MiMo Code",
                desc: "Xiaomi MiMo Code developer CLI.",
                bin: "mimo",
                launch_cmd: "mimo",
            },
            GenericAgentDef {
                id: "ante",
                name: "Ante",
                desc: "Antigma Labs Ante preview coding agent.",
                bin: "ante",
                launch_cmd: "ante",
            },
            GenericAgentDef {
                id: "trae",
                name: "ByteDance Trae",
                desc: "ByteDance Trae CLI coding assistant.",
                bin: "traecli,trae-cli",
                launch_cmd: "traecli",
            },
            GenericAgentDef {
                id: "muse",
                name: "Meta Muse",
                desc: "Meta Muse Code developer CLI.",
                bin: "muse",
                launch_cmd: "muse --trust-workspace",
            },
            GenericAgentDef {
                id: "dsh",
                name: "DeepSeek Harness",
                desc: "DeepSeek Harness (dsh-tui) terminal agent.",
                bin: "dsh-tui,dsh,dst",
                launch_cmd: "dsh-tui .",
            },
            GenericAgentDef {
                id: "qoder",
                name: "Qoder CLI",
                desc: "Qoder CLI coding agent.",
                bin: "qodercli,qoder",
                launch_cmd: "qodercli",
            },
            GenericAgentDef {
                id: "qoder-cn",
                name: "Qoder CLI China",
                desc: "Qoder CLI China coding agent.",
                bin: "qoderclicn,qodercn",
                launch_cmd: "qoderclicn",
            },
            GenericAgentDef {
                id: "zcode",
                name: "ZCode",
                desc: "Z.ai ZCode autonomous coding CLI.",
                bin: "zcode,zcode-cli",
                launch_cmd: "zcode",
            },
            GenericAgentDef {
                id: "pi",
                name: "Pi Code",
                desc: "Pi minimal coding agent (pi.dev).",
                bin: "pi",
                launch_cmd: "pi",
            },
            GenericAgentDef {
                id: "omp",
                name: "OMP (Oh My Pi)",
                desc: "Oh My Pi terminal developer assistant.",
                bin: "omp",
                launch_cmd: "omp",
            },
            GenericAgentDef {
                id: "prime-agent",
                name: "Prime Agent",
                desc: "Prime Intellect autonomous coding agent.",
                bin: "prime-agent",
                launch_cmd: "prime-agent",
            },
            GenericAgentDef {
                id: "gemini",
                name: "Google Gemini CLI",
                desc: "Google Gemini developer assistant CLI.",
                bin: "gemini",
                launch_cmd: "gemini",
            },
            GenericAgentDef {
                id: "antigravity",
                name: "Google Antigravity",
                desc: "Google Antigravity advanced agentic coding CLI.",
                bin: "agy",
                launch_cmd: "agy",
            },
            GenericAgentDef {
                id: "aider",
                name: "Aider Pair Programmer",
                desc: "AI pair programming in your terminal.",
                bin: "aider",
                launch_cmd: "aider",
            },
            GenericAgentDef {
                id: "amp",
                name: "Amp Code",
                desc: "Amp Code developer assistant.",
                bin: "amp",
                launch_cmd: "amp",
            },
            GenericAgentDef {
                id: "kilo",
                name: "Kilocode",
                desc: "Kilocode autonomous developer CLI.",
                bin: "kilo",
                launch_cmd: "kilo",
            },
            GenericAgentDef {
                id: "kiro",
                name: "Kiro CLI",
                desc: "Kiro CLI developer assistant.",
                bin: "kiro-cli,kiro",
                launch_cmd: "kiro-cli chat --tui",
            },
            GenericAgentDef {
                id: "crush",
                name: "Charm Crush",
                desc: "Charm Crush terminal developer assistant.",
                bin: "crush",
                launch_cmd: "crush",
            },
            GenericAgentDef {
                id: "aug",
                name: "Auggie (Augment)",
                desc: "Augment Auggie coding assistant.",
                bin: "auggie,aug",
                launch_cmd: "auggie",
            },
            GenericAgentDef {
                id: "autohand",
                name: "Autohand Code",
                desc: "Autohand Code CLI developer assistant.",
                bin: "autohand",
                launch_cmd: "autohand",
            },
            GenericAgentDef {
                id: "cline",
                name: "Cline Coding Agent",
                desc: "Autonomous coding agent CLI for complex tasks.",
                bin: "cline",
                launch_cmd: "cline",
            },
            GenericAgentDef {
                id: "codebuff",
                name: "Codebuff",
                desc: "Codebuff terminal coding agent.",
                bin: "codebuff",
                launch_cmd: "codebuff",
            },
            GenericAgentDef {
                id: "freebuff",
                name: "Freebuff",
                desc: "Freebuff developer assistant.",
                bin: "freebuff",
                launch_cmd: "freebuff",
            },
            GenericAgentDef {
                id: "command-code",
                name: "Command Code",
                desc: "Command Code terminal assistant.",
                bin: "command-code",
                launch_cmd: "command-code --trust",
            },
            GenericAgentDef {
                id: "continue",
                name: "Continue CLI",
                desc: "Continue CLI coding assistant.",
                bin: "cn,continue",
                launch_cmd: "cn",
            },
            GenericAgentDef {
                id: "cursor",
                name: "Cursor Agent CLI",
                desc: "Cursor background terminal agent.",
                bin: "cursor-agent,cursor",
                launch_cmd: "cursor-agent",
            },
            GenericAgentDef {
                id: "droid",
                name: "Factory Droid",
                desc: "Factory Droid autonomous agent CLI.",
                bin: "droid",
                launch_cmd: "droid",
            },
            GenericAgentDef {
                id: "kimi",
                name: "Moonshot Kimi CLI",
                desc: "Kimi coding and engineering assistant CLI.",
                bin: "kimi,kimi-code",
                launch_cmd: "kimi",
            },
            GenericAgentDef {
                id: "mistral-vibe",
                name: "Mistral Vibe",
                desc: "Mistral Vibe interactive coding assistant.",
                bin: "vibe,mistral-vibe",
                launch_cmd: "vibe",
            },
            GenericAgentDef {
                id: "qwen-code",
                name: "Alibaba Qwen Code",
                desc: "Qwen Code developer assistant CLI.",
                bin: "qwen,qwen-code",
                launch_cmd: "qwen",
            },
            GenericAgentDef {
                id: "rovo",
                name: "Atlassian Rovo Dev",
                desc: "Atlassian Rovo Dev CLI assistant.",
                bin: "rovo",
                launch_cmd: "rovo",
            },
            GenericAgentDef {
                id: "hermes",
                name: "Nous Hermes Agent",
                desc: "Nous Research Hermes autonomous agent CLI.",
                bin: "hermes",
                launch_cmd: "hermes --tui",
            },
            GenericAgentDef {
                id: "devin",
                name: "Cognition Devin CLI",
                desc: "Cognition Devin CLI agent.",
                bin: "devin",
                launch_cmd: "devin",
            },
            GenericAgentDef {
                id: "openclaw",
                name: "OpenClaw",
                desc: "OpenClaw autonomous agent CLI.",
                bin: "openclaw",
                launch_cmd: "openclaw",
            },
            GenericAgentDef {
                id: "codebuddy",
                name: "CodeBuddy",
                desc: "CodeBuddy CLI coding agent.",
                bin: "codebuddy,cbc",
                launch_cmd: "codebuddy",
            },
            GenericAgentDef {
                id: "jcode",
                name: "Jcode",
                desc: "Jcode developer assistant.",
                bin: "jcode",
                launch_cmd: "jcode",
            },
            GenericAgentDef {
                id: "governed-agent-runtime",
                name: "Custos Governed Runtime",
                desc: "Supervised agent runtime with strict Permit boundaries and proof ledger.",
                bin: "internal",
                launch_cmd: "internal",
            },
        ];

        for agent in catalog_agents {
            let (mediation, maturity) = if agent.bin == "internal" {
                (ToolMediationLevel::CustosMediated, HarnessMaturity::VerifiedNative)
            } else {
                (ToolMediationLevel::ProviderGoverned, HarnessMaturity::DiscoveredCli)
            };
            let adapter = Arc::new(super::generic_cli::GenericCliHarnessAdapter::new(
                agent.id,
                agent.bin,
                root.clone(),
                mediation,
            ));
            registry.register_with_maturity(
                adapter,
                agent.name,
                agent.desc,
                agent.bin,
                agent.launch_cmd,
                maturity,
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
        assert_eq!(harnesses.len(), 46);

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
        assert!(ids.contains(&"governed-agent-runtime"));

        assert_eq!(registry.default_harness_id(), Some("claude-code"));

        let opencode = registry.get("opencode").expect("OpenCode must be found");
        assert_eq!(opencode.harness_id(), "opencode");

        let runtimes = registry.runtimes();
        assert_eq!(runtimes.len(), 46);
        assert!(runtimes.contains_key("codex"));

        let desc = registry
            .get_descriptor("opencode")
            .expect("OpenCode descriptor");
        assert_eq!(desc.id, "opencode");
        assert_eq!(desc.name, "OpenCode");
    }
}

