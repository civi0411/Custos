//! Assistant Verifiers
//!
//! Independent oracles for verifying user approvals, payload stability, and external actions.

pub mod user_acceptance_oracle;

pub use user_acceptance_oracle::UserAcceptanceOracle;
