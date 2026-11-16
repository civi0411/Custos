use std::collections::{HashMap, HashSet};
use std::path::Path;

use custos_provider::errors::ProviderError;

fn safetensors_shard(filename: &str) -> Option<(&str, u32, u32)> {
    let stem = filename.strip_suffix(".safetensors")?;
    let (indexed_name, total) = stem.rsplit_once("-of-")?;
    let (family, index) = indexed_name.rsplit_once('-')?;
    let index = index.parse().ok()?;
    let total = total.parse().ok()?;
    (index > 0 && index <= total).then_some((family, index, total))
}

pub(crate) fn snapshot_files_are_complete(
    filenames: &HashSet<&str>,
    index: Option<&serde_json::Value>,
) -> bool {
    let safetensors: Vec<_> = filenames
        .iter()
        .copied()
        .filter(|filename| filename.ends_with(".safetensors"))
        .collect();
    if safetensors.is_empty() {
        return false;
    }

    if let Some(index) = index {
        let Some(weight_map) = index.get("weight_map").and_then(|value| value.as_object()) else {
            return false;
        };
        let Some(expected): Option<HashSet<_>> =
            weight_map.values().map(|value| value.as_str()).collect()
        else {
            return false;
        };
        return !expected.is_empty()
            && expected.iter().all(|filename| {
                filename.ends_with(".safetensors") && filenames.contains(filename)
            });
    }

    let mut shard_groups = HashMap::new();
    for filename in safetensors {
        if let Some((family, index, total)) = safetensors_shard(filename) {
            let (expected_total, indices) = shard_groups
                .entry(family)
                .or_insert_with(|| (total, HashSet::new()));
            if *expected_total != total {
                return false;
            }
            indices.insert(index);
        }
    }

    shard_groups.values().all(|(total, indices)| {
        indices.len() == *total as usize && (1..=*total).all(|index| indices.contains(&index))
    })
}

pub(crate) fn validate_snapshot_files(path: &Path) -> Result<(), ProviderError> {
    let filenames: Vec<_> = std::fs::read_dir(path)
        .map_err(mlx_file_error)?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    let filename_set = filenames.iter().map(String::as_str).collect();
    let index_path = path.join("model.safetensors.index.json");
    let index = if index_path.is_file() {
        let contents = std::fs::read(&index_path).map_err(mlx_file_error)?;
        Some(serde_json::from_slice(&contents).map_err(mlx_file_error)?)
    } else {
        None
    };
    if !snapshot_files_are_complete(&filename_set, index.as_ref()) {
        return Err(ProviderError::ExecutionError(format!(
            "MLX model at '{}' has incomplete SafeTensors weights",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn mlx_file_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::ExecutionError(format!("MLX model validation failed: {error}"))
}
