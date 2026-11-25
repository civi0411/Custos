//! Provider Credential Resolver & Account Selection
//!
//! Provides account reservation, OAuth credential extraction, model availability observation,
//! and routing policy management for provider accounts.

#[allow(clippy::module_inception)]
mod resolver;
mod selection;

#[cfg(test)]
mod tests;

pub use resolver::DaemonCredentialResolver;
pub(crate) use selection::*;
