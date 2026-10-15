//! Fast GGUF Binary Header Inspector
//!
//! Lazily reads GGUF file headers (< 64KB) to extract model architecture, context window,
//! quantization, and embedded Jinja2 chat template in under 5ms without loading tensor weights.

use custos_domain::{LocalHardwareProfile, LocalModelDescriptor, ModelCompatibility};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GgufType {
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Uint32 = 4,
    Int32 = 5,
    Float32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    Uint64 = 10,
    Int64 = 11,
    Float64 = 12,
}

impl TryFrom<u32> for GgufType {
    type Error = String;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Uint8),
            1 => Ok(Self::Int8),
            2 => Ok(Self::Uint16),
            3 => Ok(Self::Int16),
            4 => Ok(Self::Uint32),
            5 => Ok(Self::Int32),
            6 => Ok(Self::Float32),
            7 => Ok(Self::Bool),
            8 => Ok(Self::String),
            9 => Ok(Self::Array),
            10 => Ok(Self::Uint64),
            11 => Ok(Self::Int64),
            12 => Ok(Self::Float64),
            _ => Err(format!("Unknown GGUF type: {}", v)),
        }
    }
}

pub fn inspect_gguf_file(
    file_path: &Path,
    folder_id: &str,
    hardware: &LocalHardwareProfile,
) -> Result<LocalModelDescriptor, String> {
    let file = File::open(file_path).map_err(|e| format!("Cannot open {}: {}", file_path.display(), e))?;
    let metadata = file.metadata().map_err(|e| format!("Cannot read file metadata: {}", e))?;
    let file_size_bytes = metadata.len();
    let file_mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut reader = BufReader::with_capacity(64 * 1024, file);

    // 1. Magic check (4 bytes)
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(|e| format!("Failed to read magic: {}", e))?;
    if &magic != b"GGUF" {
        return Err("Not a valid GGUF file (magic header mismatch)".to_string());
    }

    // 2. Version (u32 LE)
    let mut buf4 = [0u8; 4];
    reader.read_exact(&mut buf4).map_err(|e| format!("Failed to read version: {}", e))?;
    let version = u32::from_le_bytes(buf4);
    if version < 2 {
        return Err(format!("Unsupported legacy GGUF version {}", version));
    }

    // 3. Tensor count (u64 LE) & Metadata KV count (u64 LE)
    let mut buf8 = [0u8; 8];
    reader.read_exact(&mut buf8).map_err(|e| format!("Failed to read tensor count: {}", e))?;
    let tensor_count = u64::from_le_bytes(buf8);
    if tensor_count == 0 {
        return Err("GGUF has no model tensors (it may be a tokenizer-only fixture)".to_string());
    }

    reader.read_exact(&mut buf8).map_err(|e| format!("Failed to read metadata count: {}", e))?;
    let metadata_count = u64::from_le_bytes(buf8);

    // 4. Read metadata key-values
    let mut kv_map: HashMap<String, String> = HashMap::new();
    let max_entries = metadata_count.min(2048);

    for _ in 0..max_entries {
        let key = match read_gguf_string(&mut reader) {
            Ok(k) => k,
            Err(_) => break,
        };

        let mut t_buf = [0u8; 4];
        if reader.read_exact(&mut t_buf).is_err() {
            break;
        }
        let val_type = match GgufType::try_from(u32::from_le_bytes(t_buf)) {
            Ok(t) => t,
            Err(_) => break,
        };

        match read_gguf_value(&mut reader, val_type) {
            Ok(Some(val)) => {
                kv_map.insert(key, val);
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }

    // 5. Extract domain fields
    let architecture = kv_map
        .get("general.architecture")
        .cloned()
        .unwrap_or_else(|| "llama".to_string());

    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown-model");

    let model_name = kv_map
        .get("general.name")
        .cloned()
        .unwrap_or_else(|| file_stem.to_string());

    let context_length: u64 = kv_map
        .get(&format!("{}.context_length", architecture))
        .or_else(|| kv_map.get("general.context_length"))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(4096);

    let embedding_length: u64 = kv_map
        .get(&format!("{}.embedding_length", architecture))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(4096);

    let block_count: u64 = kv_map
        .get(&format!("{}.block_count", architecture))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(32);

    let head_count: u64 = kv_map
        .get(&format!("{}.attention.head_count", architecture))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(32);

    let head_count_kv: u64 = kv_map
        .get(&format!("{}.attention.head_count_kv", architecture))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(head_count);

    let quantization = detect_quantization(file_stem, &kv_map);
    let chat_template = kv_map.get("tokenizer.chat_template").cloned();

    // 6. Memory estimation & Apple Silicon compatibility
    // Estimate based on a standard working context window (capped at 8192 for baseline estimate)
    // rather than the model's theoretical max context length (e.g. 262k), which would grossly overestimate KV cache.
    let head_dim = if head_count > 0 {
        embedding_length / head_count
    } else {
        128
    };
    let eval_context = context_length.min(8192);
    let kv_cache_bytes = 2 * block_count * head_count_kv * head_dim * eval_context * 2;
    let total_memory_required = file_size_bytes + kv_cache_bytes;

    let usable_vram = hardware.metal_vram_limit_bytes.saturating_sub(1_500_000_000); // 1.5GB display reserve
    let usable_ram = hardware.total_ram_bytes.saturating_sub(2_000_000_000); // 2GB OS headroom

    let compatibility = if total_memory_required <= usable_vram {
        ModelCompatibility::Green
    } else if total_memory_required <= usable_ram {
        ModelCompatibility::Yellow
    } else {
        ModelCompatibility::Red
    };

    let file_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model.gguf")
        .to_string();

    Ok(LocalModelDescriptor {
        file_path: file_path.to_string_lossy().to_string(),
        folder_id: folder_id.to_string(),
        file_name,
        file_size_bytes,
        file_mtime,
        architecture,
        model_name,
        context_length,
        quantization,
        chat_template,
        total_memory_required,
        compatibility,
        scanned_at: chrono::Utc::now().timestamp_millis(),
    })
}

fn detect_quantization(file_stem: &str, kv_map: &HashMap<String, String>) -> String {
    let upper = file_stem.to_uppercase();
    for q in &[
        "Q4_K_M", "Q4_K_S", "Q4_0", "Q4_1",
        "Q5_K_M", "Q5_K_S", "Q5_0", "Q5_1",
        "Q8_0", "Q8_1", "Q6_K", "Q2_K", "Q3_K_M", "Q3_K_S", "Q3_K_L",
        "BF16", "FP16", "FP32",
    ] {
        if upper.contains(q) {
            return q.to_string();
        }
    }
    if let Some(ft) = kv_map.get("general.file_type") {
        return format!("Type_{}", ft);
    }
    "GGUF".to_string()
}

fn read_gguf_string<R: Read>(reader: &mut R) -> Result<String, String> {
    let mut len_buf = [0u8; 8];
    reader.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u64::from_le_bytes(len_buf) as usize;
    if len > 1024 * 1024 {
        return Err("String length exceeds 1MB limit".to_string());
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

fn read_gguf_value<R: Read + Seek>(
    reader: &mut R,
    val_type: GgufType,
) -> Result<Option<String>, String> {
    match val_type {
        GgufType::Uint8 | GgufType::Bool => {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(b[0].to_string()))
        }
        GgufType::Int8 => {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some((b[0] as i8).to_string()))
        }
        GgufType::Uint16 => {
            let mut b = [0u8; 2];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(u16::from_le_bytes(b).to_string()))
        }
        GgufType::Int16 => {
            let mut b = [0u8; 2];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(i16::from_le_bytes(b).to_string()))
        }
        GgufType::Uint32 => {
            let mut b = [0u8; 4];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(u32::from_le_bytes(b).to_string()))
        }
        GgufType::Int32 => {
            let mut b = [0u8; 4];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(i32::from_le_bytes(b).to_string()))
        }
        GgufType::Float32 => {
            let mut b = [0u8; 4];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(f32::from_le_bytes(b).to_string()))
        }
        GgufType::Uint64 => {
            let mut b = [0u8; 8];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(u64::from_le_bytes(b).to_string()))
        }
        GgufType::Int64 => {
            let mut b = [0u8; 8];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(i64::from_le_bytes(b).to_string()))
        }
        GgufType::Float64 => {
            let mut b = [0u8; 8];
            reader.read_exact(&mut b).map_err(|e| e.to_string())?;
            Ok(Some(f64::from_le_bytes(b).to_string()))
        }
        GgufType::String => read_gguf_string(reader).map(Some),
        GgufType::Array => {
            let mut t_buf = [0u8; 4];
            reader.read_exact(&mut t_buf).map_err(|e| e.to_string())?;
            let elem_type = GgufType::try_from(u32::from_le_bytes(t_buf))?;

            let mut len_buf = [0u8; 8];
            reader.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
            let array_len = u64::from_le_bytes(len_buf);

            // Fast Skip: If array has > 24 elements (e.g. 128k token vocabularies),
            // seek past it immediately to avoid parsing lag.
            if array_len > 24 {
                skip_array(reader, elem_type, array_len)?;
                return Ok(None);
            }

            for _ in 0..array_len {
                let _ = read_gguf_value(reader, elem_type)?;
            }
            Ok(None)
        }
    }
}

fn skip_array<R: Read + Seek>(
    reader: &mut R,
    elem_type: GgufType,
    array_len: u64,
) -> Result<(), String> {
    match elem_type {
        GgufType::Uint8 | GgufType::Int8 | GgufType::Bool => {
            reader.seek(SeekFrom::Current(array_len as i64)).map_err(|e| e.to_string())?;
        }
        GgufType::Uint16 | GgufType::Int16 => {
            reader.seek(SeekFrom::Current((array_len * 2) as i64)).map_err(|e| e.to_string())?;
        }
        GgufType::Uint32 | GgufType::Int32 | GgufType::Float32 => {
            reader.seek(SeekFrom::Current((array_len * 4) as i64)).map_err(|e| e.to_string())?;
        }
        GgufType::Uint64 | GgufType::Int64 | GgufType::Float64 => {
            reader.seek(SeekFrom::Current((array_len * 8) as i64)).map_err(|e| e.to_string())?;
        }
        GgufType::String => {
            for _ in 0..array_len {
                let mut len_buf = [0u8; 8];
                reader.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
                let str_len = u64::from_le_bytes(len_buf);
                reader.seek(SeekFrom::Current(str_len as i64)).map_err(|e| e.to_string())?;
            }
        }
        GgufType::Array => {
            return Err("Nested arrays not supported".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_synthetic_gguf_inspection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("qwen2.5-coder-7b-q4_k_m.gguf");
        let mut file = File::create(&file_path).unwrap();

        // 1. Magic
        file.write_all(b"GGUF").unwrap();
        // 2. Version 3
        file.write_all(&3u32.to_le_bytes()).unwrap();
        // 3. At least one model tensor is required for discovery.
        file.write_all(&1u64.to_le_bytes()).unwrap();
        // 4. Metadata count 3
        file.write_all(&3u64.to_le_bytes()).unwrap();

        // Key 1: "general.architecture" -> String "qwen2"
        let k1 = "general.architecture";
        file.write_all(&(k1.len() as u64).to_le_bytes()).unwrap();
        file.write_all(k1.as_bytes()).unwrap();
        file.write_all(&8u32.to_le_bytes()).unwrap(); // GgufType::String
        let v1 = "qwen2";
        file.write_all(&(v1.len() as u64).to_le_bytes()).unwrap();
        file.write_all(v1.as_bytes()).unwrap();

        // Key 2: "qwen2.context_length" -> Uint64 32768
        let k2 = "qwen2.context_length";
        file.write_all(&(k2.len() as u64).to_le_bytes()).unwrap();
        file.write_all(k2.as_bytes()).unwrap();
        file.write_all(&10u32.to_le_bytes()).unwrap(); // GgufType::Uint64
        file.write_all(&32768u64.to_le_bytes()).unwrap();

        // Key 3: "general.name" -> String "Qwen2.5-Coder-7B-Instruct"
        let k3 = "general.name";
        file.write_all(&(k3.len() as u64).to_le_bytes()).unwrap();
        file.write_all(k3.as_bytes()).unwrap();
        file.write_all(&8u32.to_le_bytes()).unwrap(); // GgufType::String
        let v3 = "Qwen2.5-Coder-7B-Instruct";
        file.write_all(&(v3.len() as u64).to_le_bytes()).unwrap();
        file.write_all(v3.as_bytes()).unwrap();

        file.flush().unwrap();

        let hw = LocalHardwareProfile {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            metal_vram_limit_bytes: 24 * 1024 * 1024 * 1024,
            free_ram_bytes: 16 * 1024 * 1024 * 1024,
            is_apple_silicon: true,
            chip_name: "Apple M2 Pro".to_string(),
        };

        let descriptor = inspect_gguf_file(&file_path, "fld-test", &hw).unwrap();
        assert_eq!(descriptor.architecture, "qwen2");
        assert_eq!(descriptor.model_name, "Qwen2.5-Coder-7B-Instruct");
        assert_eq!(descriptor.context_length, 32768);
        assert_eq!(descriptor.quantization, "Q4_K_M");
        assert_eq!(descriptor.compatibility, ModelCompatibility::Green);
    }

    #[test]
    fn tokenizer_only_gguf_is_not_a_runnable_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("vocabulary.gguf");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"GGUF").unwrap();
        file.write_all(&3u32.to_le_bytes()).unwrap();
        file.write_all(&0u64.to_le_bytes()).unwrap();
        file.write_all(&0u64.to_le_bytes()).unwrap();
        file.flush().unwrap();

        let hardware = LocalHardwareProfile {
            total_ram_bytes: 1,
            metal_vram_limit_bytes: 0,
            free_ram_bytes: 1,
            is_apple_silicon: false,
            chip_name: "test".into(),
        };
        let error = inspect_gguf_file(&file_path, "test-folder", &hardware)
            .expect_err("tokenizer-only GGUF should not appear as a model");
        assert!(error.contains("no model tensors"));
    }
}
