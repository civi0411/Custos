use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use super::config_resolver;
use super::mlx::{self, MLX_BACKEND_ID};
use super::runtime::{current_runtime, InferenceRuntime, LLAMACPP_BACKEND_ID};

#[derive(Clone)]
pub struct ResolvedModelPaths {
    pub model_path: PathBuf,
    pub context_limit: usize,
    pub settings: crate::model::ModelSettings,
    pub mmproj_path: Option<PathBuf>,
    pub backend_id: Option<String>,
    pub draft_model_path: Option<PathBuf>,
}

struct ExplicitModelPath {
    model_path: PathBuf,
    backend_id: &'static str,
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn is_mlx_model_directory(path: &Path) -> Result<bool> {
    if !path.join("config.json").is_file() || !path.join("tokenizer.json").is_file() {
        return Ok(false);
    }

    for entry in std::fs::read_dir(path)? {
        if has_extension(&entry?.path(), "safetensors") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn explicit_model_path(model_id: &str) -> Result<Option<ExplicitModelPath>> {
    let clean_id = model_id.strip_prefix("native:").unwrap_or(model_id);
    if clean_id.is_empty() {
        return Ok(None);
    }

    let path = PathBuf::from(clean_id);
    if !path.exists() {
        return Ok(None);
    }

    if path.is_file() && has_extension(&path, "gguf") {
        return Ok(Some(ExplicitModelPath {
            model_path: path,
            backend_id: LLAMACPP_BACKEND_ID,
        }));
    }

    let mlx_directory = if path.is_dir() {
        Some(path.as_path())
    } else if path.is_file() && has_extension(&path, "safetensors") {
        path.parent()
    } else {
        None
    };
    if let Some(directory) = mlx_directory {
        if is_mlx_model_directory(directory)? {
            mlx::validate_model_directory(directory)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            return Ok(Some(ExplicitModelPath {
                model_path: directory.to_path_buf(),
                backend_id: MLX_BACKEND_ID,
            }));
        }
    }

    bail!(
        "Unsupported local model path '{}': expected a GGUF file or an MLX directory containing config.json, tokenizer.json, and SafeTensors weights",
        path.display()
    )
}

async fn resolve_draft_model_path(model_id: &str) -> Result<Option<PathBuf>> {
    if let Some(explicit) = explicit_model_path(model_id)? {
        return Ok(Some(explicit.model_path));
    }
    Ok(None)
}

pub fn local_context_limit(model_id: &str) -> Option<usize> {
    config_resolver::model_settings(model_id)
        .ok()
        .and_then(|settings| settings.context_size)
        .map(|limit| limit as usize)
        .filter(|limit| *limit > 0)
}

fn configured_draft_model(
    model_id: &str,
    settings: &crate::model::ModelSettings,
) -> Option<String> {
    settings
        .draft_model
        .clone()
        .or_else(|| {
            config_resolver::string_param("GOOSE_LOCAL_DRAFT_MODEL")
                .ok()
                .flatten()
        })
        .filter(|draft_model| draft_model != model_id)
}

async fn resolve_loaded_model_path(model_id: &str) -> Result<Option<ResolvedModelPaths>> {
    let Some(runtime) = current_runtime() else {
        return Ok(None);
    };
    let mut settings = config_resolver::model_settings(model_id)?;
    let Some(mut resolved) = runtime
        .loaded_model_paths(model_id, &settings.chat_template)
        .await
    else {
        return Ok(None);
    };

    let draft_model = configured_draft_model(model_id, &settings);
    if draft_model != resolved.settings.draft_model {
        resolved.draft_model_path = if let Some(draft_model) = &draft_model {
            resolve_draft_model_path(draft_model).await?
        } else {
            None
        };
    }
    settings.draft_model = draft_model;
    settings.vision_capable = resolved.mmproj_path.is_some();
    settings.mmproj_size_bytes = resolved.settings.mmproj_size_bytes;
    resolved.context_limit = settings.context_size.unwrap_or(0) as usize;
    resolved.settings = settings;
    Ok(Some(resolved))
}

pub(crate) async fn resolve_model_path(model_id: &str) -> Result<Option<ResolvedModelPaths>> {
    if let Some(explicit) = explicit_model_path(model_id)? {
        let mut settings = config_resolver::model_settings(model_id)?;
        settings.vision_capable = false;
        settings.mmproj_size_bytes = 0;
        let draft_model = configured_draft_model(model_id, &settings);
        let draft_model_path = if let Some(draft_model) = &draft_model {
            resolve_draft_model_path(draft_model).await?
        } else {
            None
        };
        settings.draft_model = draft_model;

        return Ok(Some(ResolvedModelPaths {
            model_path: explicit.model_path,
            context_limit: settings.context_size.unwrap_or(0) as usize,
            settings,
            mmproj_path: None,
            backend_id: Some(explicit.backend_id.to_string()),
            draft_model_path,
        }));
    }

    if let Some(resolved) = resolve_loaded_model_path(model_id).await? {
        return Ok(Some(resolved));
    }

    Ok(None)
}

pub fn available_inference_memory_bytes(runtime: &InferenceRuntime) -> u64 {
    runtime
        .default_backend()
        .map(|b| b.available_memory_bytes())
        .unwrap_or(0)
}
