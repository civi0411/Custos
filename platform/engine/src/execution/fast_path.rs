//! FastPath 0-Token Ingress Filter (RFC 004 §7.1, DeepSeek Super-Agent Ingress)
//!
//! Intercepts deterministic, mechanical commands at the API/chat ingress boundary before
//! allocating LLM context or dispatching to Subprocess Agent Drivers.
//!
//! Evaluates in < 1 ms, consumes 0 tokens, and incurs $0.00 cost.
//!
//! Supported deterministic categories:
//! - Navigation / Help: `help`, `/help`, `?`, `version`, `ping`, `health`
//! - Terminal Clean: `clear`, `/clear`, `reset`
//! - Workspace Status: `pwd`, `git status`, `git diff`, `git log`
//! - Workbench Operations: `list workbenches`, `/workbenches`, `active workbench`

use serde::{Deserialize, Serialize};

/// Deterministic evaluation outcome for an incoming user turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FastPathResult {
    /// Command was recognized and resolved deterministically without model deliberation.
    Matched {
        command_kind: FastPathCommand,
        response: String,
    },
    /// Query is deliberative or unstructured; pass through to the deliberative Agent runtime.
    PassThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FastPathCommand {
    Help,
    Version,
    Ping,
    Clear,
    Workbenches,
    GitStatus,
    GitDiff,
    GitLog,
    Pwd,
}

pub struct FastPathFilter;

impl FastPathFilter {
    /// Evaluates `input` against deterministic rules.
    ///
    /// Returns `FastPathResult::Matched` if a pattern matches, else `FastPathResult::PassThrough`.
    pub fn evaluate(input: &str) -> FastPathResult {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Help,
                response: "Type `/help` or enter an instruction for the Custos Agent OS.".into(),
            };
        }

        let lower = trimmed.to_lowercase();

        // 1. Help & System Information
        if matches!(
            lower.as_str(),
            "help" | "/help" | "?" | "--help" | "-h" | "man" | "commands"
        ) {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Help,
                response: Self::help_text(),
            };
        }

        // 2. Health & Ping
        if matches!(lower.as_str(), "ping" | "/ping" | "health" | "/health") {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Ping,
                response: "pong (Custos Agent OS kernel online, 0 tokens consumed)".into(),
            };
        }

        // 3. Version
        if matches!(
            lower.as_str(),
            "version" | "/version" | "--version" | "-v" | "info"
        ) {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Version,
                response: format!(
                    "Custos SADE Agent OS v{} [Local-First, Zero-Scatter Kernel]",
                    env!("CARGO_PKG_VERSION")
                ),
            };
        }

        // 4. Clear Screen / Reset
        if matches!(lower.as_str(), "clear" | "/clear" | "cls" | "reset") {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Clear,
                response: "Session view cleared.".into(),
            };
        }

        // 5. Workbench Directory
        if matches!(
            lower.as_str(),
            "workbenches"
                | "/workbenches"
                | "list workbenches"
                | "workbenches list"
                | "active workbench"
        ) {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Workbenches,
                response: Self::workbenches_text(),
            };
        }

        // 6. Safe Workspace Inspection (Fast Local Feedback)
        if lower == "pwd" {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::Pwd,
                response: "Working directory is anchored to active workspace worktree.".into(),
            };
        }

        if matches!(lower.as_str(), "git status" | "/git status" | "git st") {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::GitStatus,
                response: "[FastPath: git status] Inspecting local workspace worktree state."
                    .into(),
            };
        }

        if matches!(lower.as_str(), "git diff" | "/git diff") {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::GitDiff,
                response: "[FastPath: git diff] Uncommitted changes ready for inspection.".into(),
            };
        }

        if matches!(lower.as_str(), "git log" | "/git log") {
            return FastPathResult::Matched {
                command_kind: FastPathCommand::GitLog,
                response: "[FastPath: git log] Latest worktree revision commits.".into(),
            };
        }

        FastPathResult::PassThrough
    }

    /// Convenience check: whether `input` can be resolved via FastPath.
    pub fn is_fast_path(input: &str) -> bool {
        matches!(Self::evaluate(input), FastPathResult::Matched { .. })
    }

    fn help_text() -> String {
        r#"Custos SADE Agent OS — FastPath Command Summary (0 Tokens)
Commands:
  /help, help         Show this guide
  /version            Display kernel version
  /health, ping       Verify daemon liveness
  /clear              Clear current session transcript
  /workbenches        List available domain workbenches (Assistant, Coding, Research, Quant)
  git status, diff    Inspect active git worktree state

For complex tasks, enter plain-text instructions or prompts to activate the governed Agent deliberative pipeline."#
            .into()
    }

    fn workbenches_text() -> String {
        r#"Registered Domain Workbenches:
  1. Assistant  - Personal drafting, contact management, and governed automation
  2. Coding     - Multi-worktree isolated coding, AST navigation, and cargo verification
  3. Research   - Document sourcing, passage extraction, claims, and ALCE verification
  4. Quant      - Alpha factor discovery, OHLCV backtesting, PBO/DSR gates, and order tickets"#
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_matching() {
        assert_eq!(
            FastPathFilter::evaluate("help"),
            FastPathResult::Matched {
                command_kind: FastPathCommand::Help,
                response: FastPathFilter::help_text(),
            }
        );
        assert_eq!(
            FastPathFilter::evaluate("  /help  "),
            FastPathResult::Matched {
                command_kind: FastPathCommand::Help,
                response: FastPathFilter::help_text(),
            }
        );
        assert_eq!(
            FastPathFilter::evaluate("?"),
            FastPathResult::Matched {
                command_kind: FastPathCommand::Help,
                response: FastPathFilter::help_text(),
            }
        );
    }

    #[test]
    fn test_ping_matching() {
        match FastPathFilter::evaluate("ping") {
            FastPathResult::Matched { command_kind, response } => {
                assert_eq!(command_kind, FastPathCommand::Ping);
                assert!(response.contains("pong"));
            }
            _ => panic!("Expected matched ping"),
        }
    }

    #[test]
    fn test_workbenches_matching() {
        match FastPathFilter::evaluate("list workbenches") {
            FastPathResult::Matched { command_kind, response } => {
                assert_eq!(command_kind, FastPathCommand::Workbenches);
                assert!(response.contains("Assistant"));
                assert!(response.contains("Quant"));
            }
            _ => panic!("Expected matched workbenches"),
        }
    }

    #[test]
    fn test_deliberative_passthrough() {
        assert_eq!(
            FastPathFilter::evaluate("Implement a binary search algorithm in Rust"),
            FastPathResult::PassThrough
        );
        assert_eq!(
            FastPathFilter::evaluate("Run backtest on strategy momentum_v1 with 1y daily bars"),
            FastPathResult::PassThrough
        );
        assert_eq!(
            FastPathFilter::evaluate("Why did the last task fail?"),
            FastPathResult::PassThrough
        );
    }

    #[test]
    fn test_is_fast_path_helper() {
        assert!(FastPathFilter::is_fast_path("clear"));
        assert!(FastPathFilter::is_fast_path("/version"));
        assert!(!FastPathFilter::is_fast_path("Refactor the login controller"));
    }
}

