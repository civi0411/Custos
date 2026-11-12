//! Unified Model Intelligence Catalog
//!
//! Consolidates model capabilities, combo routing rules, pricing tables,
//! the bundled canonical model registry, and the TTLed catalog service.

pub mod canonical;
pub mod capabilities;
pub mod combo;
pub mod models;
pub mod pricing;
pub mod service;

pub use canonical::{
    canonical_name, map_provider_name, map_to_canonical_model, maybe_get_canonical_model,
    recommended_models_from_registry, CanonicalModalities, CanonicalModel,
    CanonicalModelRegistry, Limit, Modality, ModelMapping, Pricing, ThinkingMode,
};
pub use capabilities::*;
pub use combo::*;
pub use models::*;
pub use pricing::*;
pub use service::{default_canonical_models, ModelCatalogService, MODEL_CATALOG_FRESH_DURATION};

