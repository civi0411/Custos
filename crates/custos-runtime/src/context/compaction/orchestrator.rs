use custos_provider::conversation::message::Message;
use custos_provider::conversation::token_usage::ProviderUsage;
use custos_provider::conversation::Conversation;
use serde::{Deserialize, Serialize};

pub const CONVERSATION_CONTINUATION_TEXT: &str =
    "Your context was compacted. The previous message contains a summary of the conversation so far. \
Do not mention that you read a summary or that conversation summarization occurred. \
Just continue the conversation naturally based on the summarized context.";

pub const TOOL_LOOP_CONTINUATION_TEXT: &str =
    "Your context was compacted. The previous message contains a summary of the conversation so far. \
Do not mention that you read a summary or that conversation summarization occurred. \
Continue calling tools as necessary to complete the task.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub conversation: Conversation,
    pub usage: ProviderUsage,
    pub retained_context_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub threshold: f64,
    pub max_retained_tokens: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold: crate::DEFAULT_COMPACTION_THRESHOLD,
            max_retained_tokens: 32_000,
        }
    }
}

pub struct CompactionManager {
    config: CompactionConfig,
}

impl Default for CompactionManager {
    fn default() -> Self {
        Self::new(CompactionConfig::default())
    }
}

impl CompactionManager {
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Checks if a conversation has exceeded the compaction threshold
    pub fn needs_compaction(&self, current_tokens: usize, context_limit: usize) -> bool {
        if context_limit == 0 {
            return false;
        }
        let ratio = current_tokens as f64 / context_limit as f64;
        ratio >= self.config.threshold
    }

    /// Compacts messages into a single summary continuation message
    pub fn create_compacted_conversation(
        &self,
        summary_text: &str,
        retained_recent_messages: Vec<Message>,
    ) -> Conversation {
        let mut new_messages = Vec::new();

        // 1. Summary message as system/context anchor
        let summary_msg =
            Message::assistant().with_text(format!("[CONTEXT COMPACTED]\n{}", summary_text));
        new_messages.push(summary_msg);

        // 2. Continuation primer
        let primer_msg = Message::user().with_text(CONVERSATION_CONTINUATION_TEXT);
        new_messages.push(primer_msg);

        // 3. Append retained recent turns
        new_messages.extend(retained_recent_messages);

        Conversation::new_unvalidated(new_messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_threshold() {
        let mgr = CompactionManager::default(); // 0.8
        assert!(!mgr.needs_compaction(70_000, 100_000));
        assert!(mgr.needs_compaction(85_000, 100_000));
    }

    #[test]
    fn test_create_compacted_conversation() {
        let mgr = CompactionManager::default();
        let summary = "User asked to build a web server in Rust.";
        let recent = vec![Message::user().with_text("What port should we use?")];

        let conv = mgr.create_compacted_conversation(summary, recent);
        assert_eq!(conv.messages().len(), 3);
        assert!(conv.messages()[0].content[0]
            .as_text()
            .unwrap()
            .contains("[CONTEXT COMPACTED]"));
    }
}
