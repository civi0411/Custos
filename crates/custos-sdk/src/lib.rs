//! The Custos Development Kit (SDK).
//!
//! With default features this crate exports the shared wire types from
//! `wire_types` so you can build an Agent Client Protocol (ACP) client.

pub mod wire_types;
pub use wire_types::{custom_notifications, custom_requests};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("custos-engine");

#[cfg(feature = "uniffi")]
pub mod bindings;

#[cfg(feature = "uniffi")]
pub mod observability;
