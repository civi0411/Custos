//! Transitional facade over `custos_integrations::models`.

extern crate self as custos_provider;

pub use custos_integrations::models::*;
pub use custos_integrations::models as ai;
pub use custos_integrations::models::cloud as providers;
pub use custos_integrations::models::local as local_inference;
pub use custos_integrations::models::local::local_inference::prompt_template;
pub use custos_integrations::model;
