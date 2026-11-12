//! Supervised Experiment Execution & Recovery — pure domain model (F10 / P1).
//!
//! Zero I/O. Two independent axes per attempt (execution vs. compute resource),
//! a verification report that distinguishes `Unverified` from `Pass`, and a
//! resume verdict derived from evidence rather than declared booleans.

pub mod attempt;
pub mod protection;
pub mod verdict;

#[cfg(test)]
mod tests;

pub use attempt::*;
pub use protection::*;
pub use verdict::*;
