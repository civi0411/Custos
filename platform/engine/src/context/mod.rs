//! Runtime Context Subsystem
//!
//! Provides runtime context preparation, 8-step pack compilation, and live tracking:
//! - `pack`: deterministic 8-step Context Compilation Pipeline (`ContextCompiler`)
//! - `token_counter`: Token estimation for pre-compilation budget checks
//! - `compiler`: TokenAwareContextCompiler and `HeadroomPolicy` for budget-constrained packing
//! - `traits`: Context-aware runtime trait extensions

pub mod compiler;
pub mod memory;
pub mod pack;
pub mod token_counter;
pub mod traits;

pub use compiler::*;
pub use pack::{
    AnchorRetriever, CompileContextRequest, ContextCompiler, HashDeduplicator,
    ProgressiveCompactor, SecretRedactor, StructuralExtractor,
};
pub use token_counter::*;
pub use traits::*;
