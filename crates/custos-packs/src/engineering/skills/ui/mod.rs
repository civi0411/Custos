//! UI & Frontend Engineering Skills
//!
//! Provides skills for extracting external web design tokens, cloning components,
//! and diagnosing local runtime web pages within Custos OI boundaries.

pub mod component_clone;
pub mod local_ui_diagnostic;

pub use component_clone::ComponentCloneSkill;
pub use local_ui_diagnostic::LocalUiDiagnosticSkill;
