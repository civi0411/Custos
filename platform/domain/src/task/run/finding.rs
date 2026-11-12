//! Watchdog anomaly findings and normalized error signature grouping.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::step_span::StepError;

/// Discrete deterministic anomaly finding categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    LongRunning,
    RepeatedTool,
    RepeatedError,
    NoActivity,
    PossibleLoop,
}

impl FindingKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::LongRunning => "long_running",
            Self::RepeatedTool => "repeated_tool",
            Self::RepeatedError => "repeated_error",
            Self::NoActivity => "no_activity",
            Self::PossibleLoop => "possible_loop",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "long_running" => Some(Self::LongRunning),
            "repeated_tool" => Some(Self::RepeatedTool),
            "repeated_error" => Some(Self::RepeatedError),
            "no_activity" => Some(Self::NoActivity),
            "possible_loop" => Some(Self::PossibleLoop),
            _ => None,
        }
    }
}

/// Anomaly severity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisionSeverity {
    Info,
    Warning,
    Critical,
}

impl SupervisionSeverity {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "critical" | "error" | "fatal" => Self::Critical,
            "warning" | "warn" => Self::Warning,
            _ => Self::Info,
        }
    }
}

/// Bounded anomaly finding produced by deterministic rule evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisionFinding {
    pub id: String,
    pub run_id: String,
    pub kind: FindingKind,
    pub severity: SupervisionSeverity,
    pub message: String,
    pub created_at_ms: i64,
}

/// Aggregated error signature summary for fleet-level deduplication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorGroupSummary {
    pub signature_hash: String,
    pub error_type: String,
    pub sample_message: String,
    pub count: usize,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub affected_runs: Vec<String>,
}

/// Normalizes an error message by stripping volatile tokens (hex, UUIDs, line numbers, timestamps)
/// and computes a 16-hex signature hash for deterministic grouping.
pub fn compute_error_signature(error_type: &str, raw_message: &str) -> (String, String) {
    let mut normalized = String::with_capacity(raw_message.len());
    for word in raw_message.split_whitespace() {
        let is_uuid = word.len() >= 32
            && word
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_');
        let is_num = !word.is_empty() && word.chars().all(|c| c.is_ascii_digit());
        let is_hex = word.starts_with("0x")
            || (word.len() >= 8 && word.chars().all(|c| c.is_ascii_hexdigit()));

        if is_uuid || is_hex {
            normalized.push_str("<ID> ");
        } else if is_num {
            normalized.push_str("<NUM> ");
        } else {
            normalized.push_str(word);
            normalized.push(' ');
        }
    }
    let trimmed_norm = normalized.trim().to_string();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&error_type, &mut hasher);
    std::hash::Hash::hash(&trimmed_norm, &mut hasher);
    let hash_val = std::hash::Hasher::finish(&hasher);
    let signature_hash = format!("{hash_val:016x}");

    (trimmed_norm, signature_hash)
}

/// Groups a slice of recorded errors by their normalized signature hash.
pub fn group_step_errors(errors: &[StepError]) -> Vec<ErrorGroupSummary> {
    let mut groups: HashMap<String, ErrorGroupSummary> = HashMap::new();

    for err in errors {
        let entry = groups
            .entry(err.signature_hash.clone())
            .or_insert_with(|| ErrorGroupSummary {
                signature_hash: err.signature_hash.clone(),
                error_type: err.error_type.clone(),
                sample_message: err.message.clone(),
                count: 0,
                first_seen_ms: err.timestamp_ms,
                last_seen_ms: err.timestamp_ms,
                affected_runs: Vec::new(),
            });

        entry.count += 1;
        entry.first_seen_ms = entry.first_seen_ms.min(err.timestamp_ms);
        entry.last_seen_ms = entry.last_seen_ms.max(err.timestamp_ms);
        if !entry.affected_runs.contains(&err.run_id) {
            entry.affected_runs.push(err.run_id.clone());
        }
    }

    let mut result: Vec<_> = groups.into_values().collect();
    result.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| b.last_seen_ms.cmp(&a.last_seen_ms)));
    result
}

