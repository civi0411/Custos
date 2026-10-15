//! Local Model Folder Scanner & Auto-Discovery Service
//!
//! Recursively scans configured directories for .gguf model files, invokes the fast GGUF
//! inspector. External applications should be queried through their APIs.

use super::gguf_inspector::inspect_gguf_file;
use custos_domain::{LocalHardwareProfile, LocalModelDescriptor, LocalModelFolder};
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

pub fn scan_folder_for_models(
    folder: &LocalModelFolder,
    hardware: &LocalHardwareProfile,
) -> Vec<LocalModelDescriptor> {
    let root = Path::new(&folder.folder_path);
    if !root.exists() || !root.is_dir() {
        warn!(folder = %folder.folder_path, "Folder path does not exist or is not a directory");
        return Vec::new();
    }

    info!(folder = %folder.folder_path, "Scanning folder for GGUF models...");
    let mut models = Vec::new();
    collect_gguf_files(root, 0, 12, folder, hardware, &mut models);
    info!(folder = %folder.folder_path, count = models.len(), "Scan complete");
    models
}

fn collect_gguf_files(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    folder: &LocalModelFolder,
    hardware: &LocalHardwareProfile,
    acc: &mut Vec<LocalModelDescriptor>,
) {
    if current_depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(error) => {
            warn!(path = %dir.display(), %error, "Cannot scan model directory");
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden or build directories
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
            }
            collect_gguf_files(&path, current_depth + 1, max_depth, folder, hardware, acc);
        } else if path.is_file() {
            let is_gguf = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gguf"))
                .unwrap_or(false);

            if is_gguf {
                match inspect_gguf_file(&path, &folder.id, hardware) {
                    Ok(descriptor) => {
                        debug!(model = %descriptor.model_name, path = %descriptor.file_path, "Discovered GGUF model");
                        acc.push(descriptor);
                    }
                    Err(err) => {
                        warn!(path = %path.display(), error = %err, "Failed to inspect GGUF file");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_gguf_below_legacy_three_level_limit() {
        let temp = tempfile::tempdir().expect("temporary model folder");
        let nested = temp.path().join("one/two/three/four/five");
        fs::create_dir_all(&nested).expect("nested model folder");
        let model_path = nested.join("coder.gguf");
        let mut minimal_header = b"GGUF".to_vec();
        minimal_header.extend_from_slice(&3_u32.to_le_bytes());
        minimal_header.extend_from_slice(&1_u64.to_le_bytes());
        minimal_header.extend_from_slice(&0_u64.to_le_bytes());
        fs::write(&model_path, minimal_header).expect("GGUF header fixture");

        let folder = LocalModelFolder {
            id: "test-folder".into(),
            folder_path: temp.path().to_string_lossy().into_owned(),
            label: None,
            is_active: true,
            auto_scan: true,
            created_at: 0,
            updated_at: 0,
        };
        let hardware = LocalHardwareProfile {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            metal_vram_limit_bytes: 8 * 1024 * 1024 * 1024,
            free_ram_bytes: 8 * 1024 * 1024 * 1024,
            is_apple_silicon: false,
            chip_name: "test".into(),
        };

        let models = scan_folder_for_models(&folder, &hardware);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].file_path, model_path.to_string_lossy());
    }
}
