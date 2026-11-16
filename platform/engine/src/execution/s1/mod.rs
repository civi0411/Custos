//! S1 Fabric: Fast Heuristics, Calibration, Scout, and Micro-Executors
//!
//! Provides sub-second evaluations, fast read-only workspace discovery,
//! and bounded zero-risk preview tasks.

pub mod calibration;
pub mod judge;
pub mod micro_executor;
pub mod scout;

pub use calibration::*;
pub use judge::*;
pub use micro_executor::*;
pub use scout::*;
