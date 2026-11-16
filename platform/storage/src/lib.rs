extern crate custos_contracts as custos_domain;
extern crate custos_contracts as custos_core;

pub mod artifacts;
pub mod connection;
pub mod recovery;
pub mod repositories;
pub mod store;
pub mod transactions;

pub use artifacts::*;
pub use connection::*;
pub use recovery::migrator as migrations;
pub use recovery::*;
pub use repositories::*;
pub use store::*;
pub use transactions::*;

/// StorageEngine is the canonical SQLite WAL persistence engine for Custos.
pub type StorageEngine = store::SqliteTaskStore;
