//! Declarative harness specifications, argument strategies, and streaming protocols.

use custos_core::contracts::harness::{CostVisibility, HarnessMaturity, ToolMediationLevel};
use std::path::Path;

/// Wire protocol emitted by the harness process stdout/stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamProtocol {
    ClaudeJson,
    CodexJson,
    OpenCodeJson,
    PlainText,
}

/// Headless invocation arguments mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessArgs {
    /// `--print <instruction> --output-format stream-json --verbose --include-partial-messages`
    ClaudeCode,
    /// `exec --json <instruction>`
    Codex,
    /// `run --format json <instruction>`
    OpenCode,
    /// `run --text <instruction>`
    Goose,
    /// `--message <instruction> --yes-always`
    Aider,
    /// `-p <instruction>`
    DashP,
    /// `run <instruction>`
    RunPositional,
    /// `-- <instruction>`
    DoubleDash,
    /// `<instruction>`
    Positional,
}

impl HeadlessArgs {
    pub fn build_args(&self, instruction: &str) -> Vec<String> {
        match self {
            Self::ClaudeCode => vec![
                "--print".to_string(),
                instruction.to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "--include-partial-messages".to_string(),
            ],
            Self::Codex => vec![
                "exec".to_string(),
                "--json".to_string(),
                instruction.to_string(),
            ],
            Self::OpenCode => vec![
                "run".to_string(),
                "--format".to_string(),
                "json".to_string(),
                instruction.to_string(),
            ],
            Self::Goose => vec![
                "run".to_string(),
                "--text".to_string(),
                instruction.to_string(),
            ],
            Self::Aider => vec![
                "--message".to_string(),
                instruction.to_string(),
                "--yes-always".to_string(),
            ],
            Self::DashP => vec![
                "-p".to_string(),
                instruction.to_string(),
            ],
            Self::RunPositional => vec![
                "run".to_string(),
                instruction.to_string(),
            ],
            Self::DoubleDash => vec![
                "--".to_string(),
                instruction.to_string(),
            ],
            Self::Positional => vec![
                instruction.to_string(),
            ],
        }
    }

    pub fn apply(&self, cmd: &mut tokio::process::Command, instruction: &str) {
        match self {
            Self::ClaudeCode => {
                cmd.arg("--print")
                    .arg(instruction)
                    .args(["--output-format", "stream-json", "--verbose", "--include-partial-messages"]);
            }
            Self::Codex => {
                cmd.arg("exec").arg("--json").arg(instruction);
            }
            Self::OpenCode => {
                cmd.arg("run").args(["--format", "json"]).arg(instruction);
            }
            Self::Goose => {
                cmd.arg("run").arg("--text").arg(instruction);
            }
            Self::Aider => {
                cmd.arg("--message").arg(instruction).arg("--yes-always");
            }
            Self::DashP => {
                cmd.arg("-p").arg(instruction);
            }
            Self::RunPositional => {
                cmd.arg("run").arg(instruction);
            }
            Self::DoubleDash => {
                cmd.arg("--").arg(instruction);
            }
            Self::Positional => {
                cmd.arg(instruction);
            }
        }
    }
}

/// Declarative specification for an agent CLI or native harness.
#[derive(Debug, Clone)]
pub struct HarnessSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub bins: &'static str,
    pub launch_cmd: &'static str,
    pub headless: HeadlessArgs,
    pub protocol: StreamProtocol,
    pub mediation: ToolMediationLevel,
    pub cost: CostVisibility,
    pub maturity: HarnessMaturity,
}

impl HarnessSpec {
    pub fn build_command(
        &self,
        resolved_bin: &Path,
        cwd: &Path,
        instruction: &str,
    ) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(resolved_bin);
        cmd.current_dir(cwd);
        self.headless.apply(&mut cmd, instruction);
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_args_generation() {
        assert_eq!(
            HeadlessArgs::ClaudeCode.build_args("hi"),
            vec!["--print", "hi", "--output-format", "stream-json", "--verbose", "--include-partial-messages"]
        );
        assert_eq!(HeadlessArgs::Codex.build_args("hi"), vec!["exec", "--json", "hi"]);
        assert_eq!(HeadlessArgs::OpenCode.build_args("hi"), vec!["run", "--format", "json", "hi"]);
        assert_eq!(HeadlessArgs::Goose.build_args("hi"), vec!["run", "--text", "hi"]);
        assert_eq!(HeadlessArgs::Aider.build_args("hi"), vec!["--message", "hi", "--yes-always"]);
        assert_eq!(HeadlessArgs::DashP.build_args("hi"), vec!["-p", "hi"]);
        assert_eq!(HeadlessArgs::RunPositional.build_args("hi"), vec!["run", "hi"]);
        assert_eq!(HeadlessArgs::DoubleDash.build_args("hi"), vec!["--", "hi"]);
        assert_eq!(HeadlessArgs::Positional.build_args("hi"), vec!["hi"]);
    }
}
