//! Relay protocol primitives for Custos Roaming & Mobile Command Deck.
//!
//! Includes:
//! - [`SpliceStateMachine`]: 9-step monotonic relay splice lifecycle
//! - [`HostProofTranscript`]: length-prefixed transcript + challenge digest
//! - [`ZeroPiiPushGateway`]: aggregate-counter push dispatch with zero PII in logs

pub mod push;
pub mod splice;
pub mod transcript;

pub use push::{
    PushCategory, PushDeliveryCounters, ZeroPiiPushAdapter, ZeroPiiPushCommand, ZeroPiiPushGateway,
};
pub use splice::{SpliceState, SpliceStateMachine};
pub use transcript::HostProofTranscript;

