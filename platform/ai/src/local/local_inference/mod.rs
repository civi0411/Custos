//! On-device inference subsystem organized by discovery, runtime, and ModelPort adapter.

pub mod adapter;
pub mod discovery;
pub mod runtime;

#[cfg(test)]
mod tests;

pub use adapter::{
    provider, provider_utils, LocalInferenceProvider, NativeLocalModelAdapter,
    LOCAL_LLM_MODEL_CONFIG_KEY,
};
pub(crate) use adapter::StreamSender;
pub use discovery::{folder_scanner, gguf_inspector, metal_probe};
#[cfg(feature = "mlx")]
pub(crate) use runtime::native_tool_parsing;
#[cfg(feature = "safemlx")]
pub(crate) use runtime::mlx;
#[cfg(feature = "local-inference-native")]
pub(crate) use runtime::multimodal;
#[cfg(any(feature = "local-inference-native", feature = "safemlx"))]
pub(crate) use runtime::thinking_output;
pub(crate) use runtime::extract_text_content;
#[cfg(feature = "local-inference-native")]
pub(crate) use runtime::llamacpp;
pub use runtime::{
    available_inference_memory_bytes, backend, builtin_chat_template_names, config_resolver,
    evict_model, is_model_loaded, list_loaded_instances, loaded_model_ids, local_context_limit,
    messages, model, paths, preload, preload_model, preload_model_with_profile, prompt_template,
    resolution, runtime as execution_runtime, set_admission_coordinator, set_sidecar_generation_verified,
    sidecar, start_sidecar, stop_sidecar, tool_emulation, tool_parsing, verify_sidecar_generation,
    get_loaded_sidecar_instance, get_sidecar_port, is_sidecar_healthy,
    InferenceRuntime, ResolvedModelPaths, LLAMACPP_BACKEND_ID,
};
