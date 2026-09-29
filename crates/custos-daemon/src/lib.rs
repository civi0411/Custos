//! Custos Daemon Library
//!
//! Provides the local background service composition root, IPC dispatch,
//! and local API client bindings.

pub extern crate custos_core as custos_kernel;
pub extern crate custos_domain as custos_core_domain;
pub extern crate custos_runtime as custos_session;

#[path = "local_api/lib.rs"]
pub mod local_api;
pub use local_api as custos_local_api;

pub mod api;
pub mod runtime;

pub use api::*;
pub use runtime::*;
