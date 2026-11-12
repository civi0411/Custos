use anyhow::Result;
use crate::base::ModelInfo;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarConfig {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
    /// Defaults to the value of `required` if not specified.
    /// UIs may use this to feature this config value more prominently.
    pub primary: Option<bool>,
    pub description: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    #[serde(alias = "openai_compatible")]
    OpenAI,
    #[serde(alias = "ollama_compatible")]
    Ollama,
    #[serde(alias = "anthropic_compatible")]
    Anthropic,
}

impl FromStr for ProviderEngine {
    type Err = anyhow::Error;

    fn from_str(engine: &str) -> Result<Self> {
        match engine.trim().to_lowercase().as_str() {
            "openai" | "openai_compatible" => Ok(Self::OpenAI),
            "anthropic" | "anthropic_compatible" => Ok(Self::Anthropic),
            "ollama" | "ollama_compatible" => Ok(Self::Ollama),
            _ => Err(anyhow::anyhow!("Invalid provider type: {}", engine)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Executed directly, never through a shell, so `args` is never
    /// shell-interpolated. For shell features, invoke an interpreter
    /// explicitly, e.g. `command: "/bin/bash"`, `args: ["-c", "..."]`.
    /// Bare names (no path separator) are resolved via `PATH`; paths are
    /// resolved against `cwd`.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// How long a fetched credential is cached before the command is re-run,
    /// in seconds. `0` disables proactive refresh entirely — the command
    /// only reruns reactively, after an auth failure (matches Codex's
    /// `refresh_interval_ms: 0` convention).
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    /// Timeout for the command, in seconds. Defaults to 10s if unset.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    /// Working directory for the command, and the base a relative `command`
    /// path is resolved against. Defaults to goose's current directory.
    #[serde(default)]
    pub cwd: Option<String>,
}

pub fn default_refresh_interval() -> u64 {
    3600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeProviderConfig {
    pub name: String,
    pub engine: ProviderEngine,
    pub display_name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
    pub headers: Option<HashMap<String, String>>,
    /// Overrides the default `agent-session-id` header name for session ID propagation.
    #[serde(default)]
    pub session_id_header_override: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub supports_streaming: Option<bool>,
    #[serde(default = "default_requires_auth")]
    pub requires_auth: bool,
    #[serde(default)]
    pub catalog_provider_id: Option<String>,
    #[serde(default)]
    pub base_path: Option<String>,
    #[serde(default)]
    pub env_vars: Option<Vec<EnvVarConfig>>,
    /// Alternative to `api_key_env`: run a command to fetch/refresh the credential
    /// instead of reading a static secret. Mutually exclusive with `api_key_env`.
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    /// Controls whether `fetch_supported_models` calls the provider's `/v1/models`
    /// endpoint or returns the static `models` list directly.
    ///
    /// - `Some(false)` + non-empty `models`: return the static list; no API call.
    ///   Construction fails if `models` is empty.
    /// - `Some(true)` or `None`: try the API; fall back to `models` on 404.
    #[serde(default)]
    pub dynamic_models: Option<bool>,
    #[serde(default)]
    pub skip_canonical_filtering: bool,
    #[serde(default, deserialize_with = "deserialize_non_empty_string")]
    pub model_doc_link: Option<String>,
    #[serde(default)]
    pub setup_steps: Vec<String>,
    #[serde(default)]
    pub toolshim: bool,
    #[serde(default)]
    pub preserves_thinking: bool,
    /// Enables Z.AI's `clear_thinking` field, which Anthropic does not support.
    #[serde(default)]
    pub emit_clear_thinking: bool,
    #[serde(default)]
    pub setup: Option<custos_provider::catalog::canonical::catalog::ProviderSetupMetadata>,
}

pub fn default_requires_auth() -> bool {
    true
}

pub fn should_preserve_thinking_by_default(engine: &ProviderEngine) -> bool {
    matches!(engine, ProviderEngine::OpenAI)
}

/// Deserialize an optional string, treating empty/whitespace-only values as None.
pub fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.trim().is_empty()))
}

impl DeclarativeProviderConfig {
    pub fn id(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Errors if both `api_key_env` and `auth.command` are set; they're
    /// alternative ways to authenticate and mutually exclusive.
    pub fn validate_auth(&self) -> anyhow::Result<()> {
        if self.auth.is_some() && !self.api_key_env.is_empty() {
            anyhow::bail!(
                "Provider '{}' sets both `api_key_env` and `auth.command`; these are mutually exclusive.",
                self.name
            );
        }
        Ok(())
    }
}

pub trait KeyResolver {
    type Error: std::error::Error + Send + Sync + 'static;

    fn resolve_key(&self, key: &str) -> std::result::Result<String, Self::Error>;
}

pub struct EnvKeyResolver;

impl EnvKeyResolver {
    pub fn new() -> Self {
        EnvKeyResolver {}
    }
}

impl Default for EnvKeyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyResolver for EnvKeyResolver {
    type Error = std::env::VarError;

    fn resolve_key(&self, key: &str) -> std::result::Result<String, Self::Error> {
        std::env::var(key)
    }
}
