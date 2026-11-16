pub mod catalog;
pub mod cloud;
pub mod contracts;
pub mod local;
pub mod routing;

#[allow(unused_imports)]
pub use contracts::*;

// Convenience namespaces
pub use cloud as providers;
pub use local::local_inference;
pub use local::local_inference::prompt_template;

pub mod model {
    pub use super::contracts::types::model::*;
    pub use super::local::local_inference::model::*;
}
