//! Custos Capability Gateway
//!
//! THE ONLY authorized route for executing side-effects in Custos.
//! Every tool call, network request, or filesystem edit must pass through here.

pub mod deterministic;
pub mod traits;

pub use deterministic::*;
pub use traits::*;
