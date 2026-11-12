//! Host OS Operations Skills
//!
//! Physical workspace, HTTP, process, and lexical search skills executing against
//! the host operating system. Implements the `Skill` contract defined in `custos-core`.

pub mod fs;
pub mod http;
pub mod process;
pub mod search;

pub use fs::*;
pub use http::*;
pub use process::*;
pub use search::*;

