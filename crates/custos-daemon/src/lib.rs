//! Custos Daemon Library
//!
//! Provides the local background service composition root, IPC dispatch,
//! and local API client bindings.
pub use custos_sdk::local_api;
pub use custos_sdk::local_api as custos_local_api;

pub mod api;
pub mod application;
pub mod credential_resolver;
#[cfg(test)]
mod credential_tests;
pub mod credentials;
pub mod http_server;
pub mod oauth_callback_server;
pub mod profile;
pub mod runtime;
pub mod tool_executor;

pub use api::*;
pub use http_server::*;
pub use oauth_callback_server::*;
pub use profile::*;
pub use runtime::*;
