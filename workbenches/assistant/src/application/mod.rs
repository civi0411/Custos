//! Personal Assistant Application Services

pub mod drafts;
pub mod outbox;

pub use drafts::DraftsService;
pub use outbox::OutboxService;
