//! Custos Canonical Wire Protocol (`custos-protocol`)
//!
//! Single compiled owner for IPC wire contracts, Local API envelopes/methods/params,
//! worker stdio framing, A2A schemas, harness capability descriptors, and ACP extensions.

pub mod a2a;
pub mod harness;
pub mod local_api;
pub mod profile;
pub mod research;
pub mod wire_types;
pub mod worker;

pub use wire_types::{custom_notifications, custom_requests};

