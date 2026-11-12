pub(crate) mod backend;
pub(crate) mod generation;
pub(crate) mod prompt;

#[cfg(test)]
mod tests;

pub(crate) use backend::{validate_model_directory, MlxBackend, MLX_BACKEND_ID};
