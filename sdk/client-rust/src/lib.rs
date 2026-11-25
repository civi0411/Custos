//! The Custos Development Kit (SDK).
//!
//! Exports client transports, daemon connection helpers, and re-exports shared wire
//! contracts from `custos-protocol`.

pub use custos_protocol::wire_types::{self, custom_notifications, custom_requests};

pub mod local_api;
pub use local_api::*;

pub mod profile;
pub use profile::*;
