use crate::cloud::transport::api_client::ApiClient;
use crate::base::ModelInfo;
use crate::cloud::codecs::anthropic::{AnthropicFormatOptions, ANTHROPIC_PROVIDER_NAME};
use anyhow::Result;

pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-4-5";
pub const ANTHROPIC_DOC_URL: &str = "https://docs.anthropic.com/en/docs/about-claude/models";
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

// Total-request timeout applied when a declarative provider does not set
// `timeout_seconds`. Matches the OpenAI engine's default (`openai.rs`) and the
// shared `ApiClient` default so behavior is unchanged for providers that leave
// the field unset.
pub const DEFAULT_ANTHROPIC_TIMEOUT_SECONDS: u64 = 600;

#[derive(serde::Serialize)]
pub struct AnthropicProvider {
    #[serde(skip)]
    pub(crate) api_client: ApiClient,
    pub(crate) supports_streaming: bool,
    pub(crate) name: String,
    pub(crate) custom_models: Option<Vec<ModelInfo>>,
    pub(crate) dynamic_models: Option<bool>,
    pub(crate) skip_canonical_filtering: bool,
    #[serde(skip)]
    pub(crate) format_options: AnthropicFormatOptions,
}

/// Builder for [`AnthropicProvider`].
///
/// Exposes every field of the provider so that constructors living outside
/// `anthropic.rs` (e.g. in `anthropic_def.rs`, which lives in the `goose`
/// crate) can assemble a provider without needing direct access to the
/// struct's private fields.
pub struct AnthropicProviderBuilder {
    pub(crate) api_client: ApiClient,
    pub(crate) supports_streaming: bool,
    pub(crate) name: String,
    pub(crate) custom_models: Option<Vec<ModelInfo>>,
    pub(crate) dynamic_models: Option<bool>,
    pub(crate) skip_canonical_filtering: bool,
    pub(crate) format_options: AnthropicFormatOptions,
}

impl AnthropicProviderBuilder {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            supports_streaming: true,
            name: ANTHROPIC_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            format_options: AnthropicFormatOptions::default(),
        }
    }

    pub fn api_client(mut self, api_client: ApiClient) -> Self {
        self.api_client = api_client;
        self
    }

    pub fn map_api_client(mut self, f: impl FnOnce(ApiClient) -> ApiClient) -> Self {
        self.api_client = f(self.api_client);
        self
    }

    pub fn try_map_api_client(
        mut self,
        f: impl FnOnce(ApiClient) -> Result<ApiClient>,
    ) -> Result<Self> {
        self.api_client = f(self.api_client)?;
        Ok(self)
    }

    pub fn supports_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn custom_models(mut self, custom_models: Option<Vec<ModelInfo>>) -> Self {
        self.custom_models = custom_models;
        self
    }

    pub fn dynamic_models(mut self, dynamic_models: Option<bool>) -> Self {
        self.dynamic_models = dynamic_models;
        self
    }

    pub fn skip_canonical_filtering(mut self, skip_canonical_filtering: bool) -> Self {
        self.skip_canonical_filtering = skip_canonical_filtering;
        self
    }

    pub fn format_options(mut self, format_options: AnthropicFormatOptions) -> Self {
        self.format_options = format_options;
        self
    }

    pub fn map_format_options(
        mut self,
        f: impl FnOnce(AnthropicFormatOptions) -> AnthropicFormatOptions,
    ) -> Self {
        self.format_options = f(self.format_options);
        self
    }

    pub fn build(self) -> AnthropicProvider {
        AnthropicProvider {
            api_client: self.api_client,
            supports_streaming: self.supports_streaming,
            name: self.name,
            custom_models: self.custom_models,
            dynamic_models: self.dynamic_models,
            skip_canonical_filtering: self.skip_canonical_filtering,
            format_options: self.format_options,
        }
    }
}
