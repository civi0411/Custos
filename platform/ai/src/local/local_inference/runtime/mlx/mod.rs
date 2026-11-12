pub(crate) mod validation;

#[cfg(all(feature = "safemlx", target_os = "macos"))]
mod safemlx;
#[cfg(all(feature = "safemlx", target_os = "macos"))]
use safemlx as imp;

#[cfg(not(all(feature = "safemlx", target_os = "macos")))]
mod imp_stub;
#[cfg(not(all(feature = "safemlx", target_os = "macos")))]
use imp_stub as imp;

pub(crate) use imp::{validate_model_directory, MlxBackend, MLX_BACKEND_ID};
