//! Pseudo-Terminal Adapters Module
//!
//! Native POSIX / Windows PTY drivers for execution-scoped terminal sessions.

pub mod pty;

pub use pty::{NativePtyDriver, PtyHandle};
