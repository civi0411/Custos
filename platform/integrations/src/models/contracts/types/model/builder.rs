use std::collections::HashMap;

use serde_json::Value;

use super::reasoning::extract_reasoning_effort;

use super::types::{ModelConfig, DEFAULT_CONTEXT_LIMIT, INHERITED_SESSION_PARAM_KEYS};

impl ModelConfig {
    pub fn new(model_name: impl AsRef<str>) -> Self {
        let mut config = Self {
            model_name: model_name.as_ref().to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        config.normalize_effort_suffix();
        config
    }

    fn canonical_model(&self, provider_name: &str) -> Option<crate::catalog::canonical::CanonicalModel> {
        crate::catalog::canonical::maybe_get_canonical_model(provider_name, &self.model_name).or_else(|| {
            let (base, _effort) = extract_reasoning_effort(&self.model_name);
            if base != self.model_name {
                crate::catalog::canonical::maybe_get_canonical_model(provider_name, &base)
            } else {
                None
            }
        })
    }

    pub fn with_canonical_vision_support(mut self, provider_name: &str) -> Self {
        if self.supports_vision.is_none() {
            if let Some(canonical) = self.canonical_model(provider_name) {
                self.supports_vision = Some(
                    canonical
                        .modalities
                        .input
                        .contains(&crate::catalog::canonical::Modality::Image),
                );
            }
        }
        self
    }

    pub fn with_canonical_limits(mut self, provider_name: &str) -> Self {
        let canonical = self.canonical_model(provider_name);

        if let Some(canonical) = canonical {
            if self.max_tokens.is_none() {
                self.max_tokens = canonical
                    .limit
                    .output
                    .filter(|&output| output < canonical.limit.context)
                    .map(|output| output as i32);
            }
            if self.reasoning.is_none() {
                self.reasoning = canonical.reasoning;
            }
            if self.supports_vision.is_none() {
                self.supports_vision = Some(
                    canonical
                        .modalities
                        .input
                        .contains(&crate::catalog::canonical::Modality::Image),
                )
            }
        }

        self
    }

    pub fn with_context_limit(mut self, limit: Option<usize>) -> Self {
        if limit.is_some() {
            self.context_limit = limit;
        }
        self
    }

    pub fn with_temperature(mut self, temp: Option<f32>) -> Self {
        self.temperature = temp;
        self
    }

    pub fn with_max_tokens(mut self, tokens: Option<i32>) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn with_default_context_limit(mut self, limit: Option<usize>) -> Self {
        if self.context_limit.is_none() {
            self.context_limit = limit;
        }
        self
    }

    pub fn with_default_max_tokens(mut self, tokens: Option<i32>) -> Self {
        if self.max_tokens.is_none() {
            self.max_tokens = tokens;
        }
        self
    }

    pub fn with_toolshim(mut self, toolshim: bool) -> Self {
        self.toolshim = toolshim;
        self
    }

    pub fn with_toolshim_model(mut self, model: Option<String>) -> Self {
        self.toolshim_model = model;
        self
    }

    pub fn with_request_headers(mut self, headers: Option<HashMap<String, String>>) -> Self {
        self.request_headers = headers;
        self
    }

    pub fn with_merged_request_params(mut self, params: HashMap<String, Value>) -> Self {
        match self.request_params.as_mut() {
            Some(existing) => {
                for (k, v) in params {
                    existing.insert(k, v);
                }
            }
            None => {
                self.request_params = Some(params);
            }
        }
        self
    }

    pub fn with_vision_support(mut self, supports_vision: bool) -> Self {
        self.supports_vision = Some(supports_vision);
        self
    }

    pub fn with_inherited_session_settings_from(
        mut self,
        previous: Option<&ModelConfig>,
        request_params: Option<HashMap<String, Value>>,
    ) -> Self {
        if let Some(previous_params) = previous.and_then(|p| p.request_params.as_ref()) {
            for key in INHERITED_SESSION_PARAM_KEYS {
                if let Some(value) = previous_params.get(*key) {
                    self.request_params
                        .get_or_insert_with(HashMap::new)
                        .entry(key.to_string())
                        .or_insert_with(|| value.clone());
                }
            }
        }

        if let Some(request_params) = request_params {
            self = self.with_merged_request_params(request_params);
        }

        self
    }

    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }

    pub fn max_output_tokens(&self) -> i32 {
        if let Some(tokens) = self.max_tokens {
            return tokens;
        }

        4_096
    }

    pub fn request_param<T: for<'de> serde::Deserialize<'de>>(
        &self,
        request_key: &str,
    ) -> Option<T> {
        self.request_params
            .as_ref()
            .and_then(|params| params.get(request_key))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

