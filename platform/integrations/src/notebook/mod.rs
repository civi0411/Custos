//! Notebook Adapters Module
//!
//! Subprocess-backed Python execution kernel driver with strict memory bounds.

pub mod driver;

pub use driver::LocalPythonDriver;
