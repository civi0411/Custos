//! Runtime Run Supervision Engine (located inside canonical pillar `workflow`).

pub mod coordinator;
mod processor;
pub mod profile_registry;
pub mod redactor;

pub use coordinator::SupervisionCoordinator;
pub use profile_registry::SupervisionProfileRegistry;
pub use redactor::PayloadRedactor;

#[cfg(test)]
mod tests;
