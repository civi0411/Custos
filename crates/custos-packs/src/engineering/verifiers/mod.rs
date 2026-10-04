//! Engineering Verifiers
//!
//! Independent oracles for verifying engineering tasks before completion.

pub mod cargo_test_oracle;

pub use cargo_test_oracle::CargoTestOracle;
