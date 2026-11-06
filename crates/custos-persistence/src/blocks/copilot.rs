//! Copilot Workbench Persistence Block
//!
//! Owns relational persistence for personal assistant profiles, communication drafts,
//! interactive chat sessions, turns, and mutating outbox approvals.

pub use crate::repositories::assistant::AssistantRepository;
pub use crate::repositories::outbox::OutboxRepository;
pub use crate::repositories::providers::ProviderRepository;
pub use crate::repositories::session::SessionRepository;
