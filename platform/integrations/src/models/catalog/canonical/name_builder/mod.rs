mod mapper;
mod patterns;

#[cfg(test)]
mod tests;

pub use mapper::{canonical_name, map_provider_name, map_to_canonical_model};
pub(crate) use mapper::is_meta_provider;
pub use patterns::strip_version_suffix;
