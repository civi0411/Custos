use std::path::Path;

use crate::local_inference::backend::{
    BackendLoadedModel, LocalGenerationRequest, LocalInferenceBackend,
};
use crate::local_inference::model::ModelSettings;
use crate::local_inference::ResolvedModelPaths;
use custos_provider::errors::ProviderError;

pub(crate) const MLX_BACKEND_ID: &str = "mlx";

pub(crate) struct MlxBackend;

impl MlxBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

pub(crate) fn validate_model_directory(path: &Path) -> Result<(), ProviderError> {
    super::validation::validate_snapshot_files(path)?;
    let reason = if cfg!(target_os = "macos") {
        "MLX support was not compiled in. Rebuild with the `mlx` feature."
    } else {
        "MLX backend requires macOS."
    };
    Err(ProviderError::ExecutionError(reason.to_string()))
}

impl LocalInferenceBackend for MlxBackend {
    fn id(&self) -> &'static str {
        MLX_BACKEND_ID
    }

    fn load_model(
        &self,
        _model_id: &str,
        _resolved: &ResolvedModelPaths,
        _settings: &ModelSettings,
    ) -> Result<Box<dyn BackendLoadedModel>, ProviderError> {
        Err(ProviderError::ExecutionError(
            "MLX backend support was not compiled in. Rebuild with the `mlx` feature.".to_string(),
        ))
    }

    fn generate(
        &self,
        _loaded: &mut dyn BackendLoadedModel,
        _request: LocalGenerationRequest<'_>,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::ExecutionError(
            "MLX backend support was not compiled in. Rebuild with the `mlx` feature.".to_string(),
        ))
    }

    fn available_memory_bytes(&self) -> u64 {
        0
    }
}
