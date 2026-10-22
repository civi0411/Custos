//! The Custos Development Kit (SDK).
//!
//! With default features this crate exports the shared wire types from
//! `wire_types` so you can build an Agent Client Protocol (ACP) client.

pub mod wire_types;
pub use wire_types::{custom_notifications, custom_requests};

pub mod local_api;
pub use local_api::*;

pub mod profile;
pub use profile::*;

#[cfg(feature = "uniffi")]
compile_error!("uniffi feature not supported — see F9");

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("custos-engine");

#[cfg(feature = "uniffi")]
pub mod bindings;

#[cfg(feature = "uniffi")]
pub mod observability;
