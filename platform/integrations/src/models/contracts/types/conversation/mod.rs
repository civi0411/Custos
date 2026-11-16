pub mod core;
pub mod fix;
pub mod message;
pub mod token_usage;
mod tool_request;
mod tool_result_serde;

#[cfg(test)]
mod tests;

pub use core::{Conversation, InvalidConversation};
pub use fix::{
    debug_conversation_fix, effective_role, fix_conversation, merge_consecutive_messages,
    merge_consecutive_messages_for_request, EffectiveRole, CURRENT_TIME_TAG, TURN_CONTEXT_TAG,
    WORKING_DIRECTORY_TAG,
};
