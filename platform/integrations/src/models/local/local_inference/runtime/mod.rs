//! On-device inference execution runtimes (llama.cpp, MLX, sidecar) and model preloading.

pub mod backend;
pub mod config_resolver;
#[cfg(feature = "local-inference-native")]
pub(crate) mod llamacpp;
pub mod messages;
pub(crate) mod mlx;
pub mod model;
pub(crate) mod multimodal;
#[cfg(feature = "mlx")]
pub(crate) mod native_tool_parsing;
pub mod paths;
pub mod preload;
pub mod prompt_template;
pub mod resolution;
#[allow(clippy::module_inception)]
pub mod runtime;
pub mod sidecar;
pub(crate) mod thinking_output;
pub mod tool_emulation;
pub mod tool_parsing;

#[cfg(feature = "local-inference-native")]
pub(crate) use crate::local_inference::StreamSender;
#[cfg(feature = "local-inference-native")]
pub(crate) use messages::finalize_usage;
pub(crate) use messages::extract_text_content;
pub use preload::{preload_model, preload_model_with_profile};
pub use resolution::{
    available_inference_memory_bytes, local_context_limit, ResolvedModelPaths,
};
pub use runtime::{
    builtin_chat_template_names, evict_model, is_model_loaded, list_loaded_instances,
    loaded_model_ids, set_admission_coordinator, InferenceRuntime, LLAMACPP_BACKEND_ID,
};
pub use sidecar::{
    get_loaded_sidecar_instance, get_sidecar_port, is_sidecar_healthy,
    set_sidecar_generation_verified, start_sidecar, stop_sidecar, verify_sidecar_generation,
};

