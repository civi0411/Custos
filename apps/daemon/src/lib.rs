//! Custos Daemon Library
//!
//! Provides the local background service composition root, IPC dispatch,
//! and local API client bindings.
extern crate custos_contracts as custos_domain;
extern crate custos_engine as custos_core;
extern crate custos_engine as custos_runtime;
extern crate custos_engine as custos_packs;
extern crate custos_integrations as custos_provider;
extern crate custos_integrations as custos_adapters;
extern crate custos_storage as custos_persistence;

pub use custos_protocol::local_api;
pub use custos_protocol::local_api as custos_local_api;

pub mod api;
pub mod application;
pub mod auth;
pub mod bootstrap;
pub mod http_server;
pub mod profile;
pub mod runtime;
pub mod tool_executor;

pub use api::*;
pub use auth::credential_store;
pub use auth::credential_store as credentials;
#[cfg(test)]
pub(crate) use auth::credential_tests;
pub use auth::oauth_callback as oauth_callback_server;
pub use auth::oauth_callback::*;
pub use auth::resolver as credential_resolver;
pub use http_server::*;
pub use profile::*;
pub use runtime::*;
