//! Custos Sovereign Prompt Manager
//!
//! Generates and manages system prompts with deterministic sorting for optimal LLM prompt caching,
//! unicode tag sanitization for prompt injection defense, and multi-mode support (Autonomous, Assisted, Chat, Deliberation).

use chrono::Utc;
use custos_provider_types::utils::sanitize_unicode_tags;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CustosAgentMode {
    #[default]
    Assisted,
    Autonomous,
    Chat,
    Deliberation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPromptInfo {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

impl ExtensionPromptInfo {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            instructions: instructions.into(),
        }
    }
}

pub struct PromptManager {
    system_prompt_override: Option<String>,
    system_prompt_extras: BTreeMap<String, String>,
    current_date_timestamp: String,
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptManager {
    pub fn new() -> Self {
        Self {
            system_prompt_override: None,
            system_prompt_extras: BTreeMap::new(),
            current_date_timestamp: Utc::now().format("%Y-%m-%d %H:00 %:z").to_string(),
        }
    }

    pub fn with_timestamp(timestamp: impl Into<String>) -> Self {
        Self {
            system_prompt_override: None,
            system_prompt_extras: BTreeMap::new(),
            current_date_timestamp: timestamp.into(),
        }
    }

    pub fn add_system_prompt_extra(
        &mut self,
        key: impl Into<String>,
        instruction: impl Into<String>,
    ) {
        self.system_prompt_extras
            .insert(key.into(), instruction.into());
    }

    pub fn remove_system_prompt_extra(&mut self, key: &str) -> Option<String> {
        self.system_prompt_extras.remove(key)
    }

    pub fn set_system_prompt_override(&mut self, prompt: impl Into<String>) {
        self.system_prompt_override = Some(prompt.into());
    }

    pub fn clear_system_prompt_override(&mut self) {
        self.system_prompt_override = None;
    }

    pub fn builder(&self) -> SystemPromptBuilder<'_> {
        SystemPromptBuilder {
            manager: self,
            extensions: Vec::new(),
            prompt_extras: BTreeMap::new(),
            subagents_enabled: false,
            hints: None,
            agent_mode: CustosAgentMode::Assisted,
            include_extensions: true,
        }
    }
}

pub struct SystemPromptBuilder<'a> {
    manager: &'a PromptManager,
    extensions: Vec<ExtensionPromptInfo>,
    prompt_extras: BTreeMap<String, String>,
    subagents_enabled: bool,
    hints: Option<String>,
    agent_mode: CustosAgentMode,
    include_extensions: bool,
}

impl<'a> SystemPromptBuilder<'a> {
    pub fn with_extension(mut self, ext: ExtensionPromptInfo) -> Self {
        self.extensions.push(ext);
        self
    }

    pub fn with_extensions(mut self, exts: impl IntoIterator<Item = ExtensionPromptInfo>) -> Self {
        self.extensions.extend(exts);
        self
    }

    pub fn with_prompt_extras(
        mut self,
        extras: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        self.prompt_extras.extend(extras);
        self
    }

    pub fn with_hints(mut self, hints: impl Into<String>) -> Self {
        self.hints = Some(hints.into());
        self
    }

    pub fn with_agent_mode(mut self, mode: CustosAgentMode) -> Self {
        self.agent_mode = mode;
        self
    }

    pub fn with_subagents(mut self, enabled: bool) -> Self {
        self.subagents_enabled = enabled;
        self
    }

    pub fn without_extensions(mut self) -> Self {
        self.include_extensions = false;
        self
    }

    pub fn build(mut self) -> String {
        let base = if let Some(override_prompt) = &self.manager.system_prompt_override {
            sanitize_unicode_tags(override_prompt)
        } else {
            let mut prompt = String::from(
                "You are Custos, a sovereign autonomous agentic operating system designed for \
                 mission-critical software engineering, multi-agent deliberation, and durable workflow execution.",
            );

            prompt.push_str(&format!(
                "\nCurrent system time: {}",
                self.manager.current_date_timestamp
            ));

            match self.agent_mode {
                CustosAgentMode::Autonomous => {
                    prompt.push_str(
                        "\n\nOperating Mode: AUTONOMOUS. You have authorization to plan, execute, and verify \
                         actions toward the user's objective within safety constraints and budget guards.",
                    );
                }
                CustosAgentMode::Assisted => {
                    prompt.push_str(
                        "\n\nOperating Mode: ASSISTED. Proactively recommend solutions and collaborate \
                         with the user, seeking approval on impactful state changes.",
                    );
                }
                CustosAgentMode::Chat => {
                    prompt.push_str(
                        "\n\nOperating Mode: CHAT-ONLY. Direct interaction without invoking external environment mutations.",
                    );
                }
                CustosAgentMode::Deliberation => {
                    prompt.push_str(
                        "\n\nOperating Mode: SYSTEM TWO DELIBERATION. Rigorous verification, multi-role \
                         critique (Architect, Coder, Critic, Tester), and consensus enforcement are active.",
                    );
                }
            }

            if self.subagents_enabled {
                prompt.push_str(
                    "\n\nSubagent Delegation: ACTIVE. You can spawn and coordinate specialized peer and subagent workers.",
                );
            }

            prompt
        };

        let mut all_extras = self.manager.system_prompt_extras.clone();
        all_extras.extend(self.prompt_extras);

        if let Some(hints) = self.hints {
            all_extras.insert("00_hints".to_string(), hints);
        }

        if self.include_extensions && !self.extensions.is_empty() {
            // Sort extensions by name for stable prompt caching
            self.extensions.sort_by(|a, b| a.name.cmp(&b.name));

            let mut ext_block = String::from("Available Capabilities & Extensions:\n");
            for ext in &self.extensions {
                ext_block.push_str(&format!(
                    "- **{}**: {}\n  Instructions: {}\n",
                    sanitize_unicode_tags(&ext.name),
                    sanitize_unicode_tags(&ext.description),
                    sanitize_unicode_tags(&ext.instructions)
                ));
            }
            all_extras.insert("10_extensions".to_string(), ext_block);
        }

        if all_extras.is_empty() {
            base
        } else {
            let mut result = base;
            result.push_str("\n\n# Additional Operational Directives\n");
            for (_key, instruction) in all_extras {
                let sanitized = sanitize_unicode_tags(&instruction);
                result.push('\n');
                result.push_str(&sanitized);
                result.push('\n');
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_manager_default_build() {
        let manager = PromptManager::with_timestamp("2026-09-27 12:00:00 +00:00");
        let prompt = manager.builder().build();

        assert!(prompt.contains("You are Custos"));
        assert!(prompt.contains("Current system time: 2026-09-27 12:00:00 +00:00"));
        assert!(prompt.contains("Operating Mode: ASSISTED"));
    }

    #[test]
    fn test_prompt_manager_modes() {
        let manager = PromptManager::new();
        let prompt_auto = manager
            .builder()
            .with_agent_mode(CustosAgentMode::Autonomous)
            .build();
        assert!(prompt_auto.contains("AUTONOMOUS"));

        let prompt_delib = manager
            .builder()
            .with_agent_mode(CustosAgentMode::Deliberation)
            .build();
        assert!(prompt_delib.contains("SYSTEM TWO DELIBERATION"));
    }

    #[test]
    fn test_prompt_manager_extensions_and_caching_order() {
        let manager = PromptManager::new();
        let ext_b = ExtensionPromptInfo::new("b_tool", "Beta tool", "Run beta");
        let ext_a = ExtensionPromptInfo::new("a_tool", "Alpha tool", "Run alpha");

        let prompt = manager
            .builder()
            .with_extension(ext_b)
            .with_extension(ext_a)
            .build();

        assert!(prompt.contains("Available Capabilities & Extensions:"));
        let pos_a = prompt.find("a_tool").unwrap();
        let pos_b = prompt.find("b_tool").unwrap();
        assert!(
            pos_a < pos_b,
            "Extensions must be sorted alphabetically for prompt caching"
        );
    }

    #[test]
    fn test_unicode_sanitization_in_prompt() {
        let manager = PromptManager::new();
        // Insert unicode tag character \u{E0020}
        let dangerous_hint = "Execute normal code\u{E0020}hidden injection";
        let prompt = manager.builder().with_hints(dangerous_hint).build();

        assert!(!prompt.contains('\u{E0020}'));
        assert!(prompt.contains("Execute normal code"));
        assert!(prompt.contains("hidden injection"));
    }

    #[test]
    fn test_prompt_override() {
        let mut manager = PromptManager::new();
        manager.set_system_prompt_override("Custom sovereign prompt override");
        let prompt = manager.builder().build();
        assert_eq!(prompt, "Custom sovereign prompt override");

        manager.clear_system_prompt_override();
        let prompt2 = manager.builder().build();
        assert!(prompt2.contains("You are Custos"));
    }
}
