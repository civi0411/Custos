//! Managed Native Sidecar Engine for Local Model Inference
//!
//! Spawns and manages a separately installed `llama-server` process.
//! Compatibility is established by a real load attempt; discovery never claims
//! that every GGUF architecture is supported.

mod chat;
mod lifecycle;
mod types;

pub use chat::stream_sidecar_chat;
pub use lifecycle::{
    set_sidecar_generation_verified, start_sidecar, stop_sidecar, verify_sidecar_generation,
};
pub use types::{
    find_llama_server_binary, get_loaded_sidecar_instance, get_sidecar_port, is_sidecar_healthy,
    ActiveSidecar,
};
