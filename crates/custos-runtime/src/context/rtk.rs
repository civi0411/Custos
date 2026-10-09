//! Real-Time Context Compression (RTK) & Headroom Engine
//!
//! Absorbs 9Router's `open-sse/rtk/headroom.js` and `pxpipe.js` into Custos native Rust.
//! Enforces safety headroom before model dispatch and performs verifiable context pruning
//! with tamper-evident `OmissionRecord` generation for the Evidence Ledger.

use blake3;
use custos_provider::types::conversation::message::{Message, MessageContentBlock};
use rmcp::model::{ContentBlock, Role};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum RtkError {
    #[error("Context window overflow: current {current} tokens exceed max allowed {max_allowed} (context window {context_window} - headroom {headroom})")]
    HeadroomExceeded {
        current: usize,
        max_allowed: usize,
        context_window: usize,
        headroom: usize,
    },
}

/// Record of context pruned or omitted by RTK.
/// Required by Custos Completion Gate and Evidence Ledger to maintain
/// verifiable audit trails of prompt transformations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmissionRecord {
    pub message_index: usize,
    pub original_tokens: usize,
    pub pruned_tokens: usize,
    pub reason: String,
    /// BLAKE3 digest of the omitted/pruned content to ensure integrity
    pub omission_hash: String,
}

/// Policy for calculating context headroom and token limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadroomPolicy {
    /// Reserved tokens for model reasoning and completion (e.g. 4096)
    pub safety_headroom_tokens: usize,
    /// Total context window of the target model (e.g. 128_000)
    pub context_window: usize,
}

impl Default for HeadroomPolicy {
    fn default() -> Self {
        Self {
            safety_headroom_tokens: 4096,
            context_window: 128_000,
        }
    }
}

impl HeadroomPolicy {
    pub fn new(context_window: usize, safety_headroom_tokens: usize) -> Self {
        Self {
            context_window,
            safety_headroom_tokens,
        }
    }

    /// Maximum tokens available for the prompt before violating the headroom reservation
    pub fn max_prompt_tokens(&self) -> usize {
        self.context_window.saturating_sub(self.safety_headroom_tokens)
    }

    /// Verifies if the prompt fits within the safe window
    pub fn check_headroom(&self, current_tokens: usize) -> Result<(), RtkError> {
        let max_allowed = self.max_prompt_tokens();
        if current_tokens > max_allowed {
            Err(RtkError::HeadroomExceeded {
                current: current_tokens,
                max_allowed,
                context_window: self.context_window,
                headroom: self.safety_headroom_tokens,
            })
        } else {
            Ok(())
        }
    }
}

/// RTK Pruner that compresses and compacts messages to fit within headroom limits.
pub struct RtkPruner;

impl RtkPruner {
    /// Fast estimation of tokens for a string (~4 chars per token).
    pub fn estimate_tokens(text: &str) -> usize {
        let chars = text.chars().count();
        if chars == 0 {
            0
        } else {
            chars.div_ceil(4)
        }
    }

    /// Estimates the token count of a Message.
    pub fn estimate_message_tokens(message: &Message) -> usize {
        let mut tokens = 4; // per-message envelope overhead
        for block in &message.content {
            match block {
                MessageContentBlock::Text(t) => {
                    tokens += Self::estimate_tokens(&t.text);
                }
                MessageContentBlock::Thinking(th) => {
                    tokens += Self::estimate_tokens(&th.thinking);
                }
                MessageContentBlock::ToolResponse(r) => {
                    if let Ok(res) = &r.tool_result {
                        for c in &res.content {
                            if let Some(text) = c.as_text() {
                                tokens += Self::estimate_tokens(&text.text);
                            }
                        }
                    }
                }
                _ => {
                    tokens += 10;
                }
            }
        }
        tokens
    }

    /// Prune messages to fit under `target_max_tokens`.
    /// Preserves the initial system instructions and the most recent user turn.
    /// Focuses pruning on historical verbose tool outputs and intermediate assistant messages.
    pub fn prune_messages(
        messages: &mut [Message],
        target_max_tokens: usize,
    ) -> Vec<OmissionRecord> {
        let mut omissions = Vec::new();
        let total_messages = messages.len();
        if total_messages <= 2 {
            return omissions;
        }

        let mut current_total: usize = messages.iter().map(Self::estimate_message_tokens).sum();
        if current_total <= target_max_tokens {
            return omissions;
        }

        // Iterate through intermediate messages (skip first message and last message)
        for idx in 1..(total_messages - 1) {
            if current_total <= target_max_tokens {
                break;
            }

            let msg = &mut messages[idx];
            // Prune ToolResponse blocks first (they are usually verbose stdout / file dumps)
            for block in &mut msg.content {
                if let MessageContentBlock::ToolResponse(ref mut r) = block {
                    if let Ok(ref mut res) = r.tool_result {
                        for item in &mut res.content {
                            if let ContentBlock::Text(ref mut text_block) = item {
                                let orig_tokens = Self::estimate_tokens(&text_block.text);
                                if orig_tokens > 100 {
                                    let hash = blake3::hash(text_block.text.as_bytes()).to_hex().to_string();
                                    let max_keep_chars = 200;
                                    let truncated: String = text_block.text.chars().take(max_keep_chars).collect();
                                    let replacement = format!(
                                        "{}\n\n[... RTK Pruned: {} tokens omitted for headroom ...]",
                                        truncated,
                                        orig_tokens.saturating_sub(50)
                                    );
                                    let new_tokens = Self::estimate_tokens(&replacement);
                                    let pruned_tokens = orig_tokens.saturating_sub(new_tokens);

                                    text_block.text = replacement;
                                    current_total = current_total.saturating_sub(pruned_tokens);

                                    omissions.push(OmissionRecord {
                                        message_index: idx,
                                        original_tokens: orig_tokens,
                                        pruned_tokens,
                                        reason: "Tool output trimmed to fit context headroom".to_string(),
                                        omission_hash: hash,
                                    });

                                    if current_total <= target_max_tokens {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If still over budget and message is intermediate Assistant text, trim long text
            if current_total > target_max_tokens && msg.role == Role::Assistant {
                for block in &mut msg.content {
                    if let MessageContentBlock::Text(ref mut t) = block {
                        let orig_tokens = Self::estimate_tokens(&t.text);
                        if orig_tokens > 200 {
                            let hash = blake3::hash(t.text.as_bytes()).to_hex().to_string();
                            let max_keep_chars = 300;
                            let truncated: String = t.text.chars().take(max_keep_chars).collect();
                            let replacement = format!(
                                "{}\n\n[... RTK Pruned assistant reasoning: {} tokens omitted ...]",
                                truncated,
                                orig_tokens.saturating_sub(75)
                            );
                            let new_tokens = Self::estimate_tokens(&replacement);
                            let pruned_tokens = orig_tokens.saturating_sub(new_tokens);

                            t.text = replacement;
                            current_total = current_total.saturating_sub(pruned_tokens);

                            omissions.push(OmissionRecord {
                                message_index: idx,
                                original_tokens: orig_tokens,
                                pruned_tokens,
                                reason: "Intermediate assistant message trimmed".to_string(),
                                omission_hash: hash,
                            });

                            if current_total <= target_max_tokens {
                                break;
                            }
                        }
                    }
                }
            }
        }

        omissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolResult;

    #[test]
    fn test_headroom_policy() {
        let policy = HeadroomPolicy::new(100_000, 4_000);
        assert_eq!(policy.max_prompt_tokens(), 96_000);
        assert!(policy.check_headroom(95_000).is_ok());
        assert!(policy.check_headroom(96_001).is_err());
    }

    #[test]
    fn test_rtk_pruning_with_evidence_omission() {
        let mut msg0 = Message::user().with_text("System instructions for task execution");
        msg0.role = Role::Assistant;

        // Long tool response
        let long_output = "X".repeat(2000);
        let mut msg1 = Message::user();
        msg1.content = vec![MessageContentBlock::ToolResponse(
            custos_provider::types::conversation::message::ToolResponse {
                id: "call_123".to_string(),
                tool_result: Ok(CallToolResult::success(vec![ContentBlock::text(long_output)])),
                metadata: None,
            },
        )];

        let msg2 = Message::user().with_text("Latest user prompt to solve bug");

        let mut messages = vec![msg0, msg1, msg2];
        let initial_tokens: usize = messages.iter().map(RtkPruner::estimate_message_tokens).sum();
        assert!(initial_tokens > 400);

        // Budget for 200 tokens
        let omissions = RtkPruner::prune_messages(&mut messages, 200);

        assert!(!omissions.is_empty());
        assert_eq!(omissions[0].message_index, 1);
        assert!(!omissions[0].omission_hash.is_empty());

        let final_tokens: usize = messages.iter().map(RtkPruner::estimate_message_tokens).sum();
        assert!(final_tokens < initial_tokens);
    }
}
