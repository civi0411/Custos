use crate::conversation::fix::fix_messages;
use crate::conversation::message::{Message, MessageContentBlock, MessageMetadata};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conversation(Vec<Message>);

#[derive(Error, Debug)]
#[error("invalid conversation: {reason}")]
pub struct InvalidConversation {
    pub(crate) reason: String,
    pub(crate) conversation: Conversation,
}

impl Conversation {
    pub fn new<I>(messages: I) -> Result<Self, InvalidConversation>
    where
        I: IntoIterator<Item = Message>,
    {
        Self::new_unvalidated(messages).validate()
    }

    pub fn new_unvalidated<I>(messages: I) -> Self
    where
        I: IntoIterator<Item = Message>,
    {
        Self(messages.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self::new_unvalidated([])
    }

    pub fn messages(&self) -> &Vec<Message> {
        &self.0
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.0
    }

    pub fn push(&mut self, message: Message) {
        let output_token_limit_reached = message.metadata.output_token_limit_reached;
        if message.content.is_empty()
            && (message.metadata.inference.is_some() || output_token_limit_reached)
        {
            if let Some(existing) = self.0.iter_mut().rev().find(|existing| {
                existing.role == message.role
                    && existing.is_user_visible()
                    && (!output_token_limit_reached
                        || (message.id.is_some()
                            && existing.id.as_deref() == message.id.as_deref()))
            }) {
                if let Some(inference) = message.metadata.inference.clone() {
                    existing.metadata.inference = Some(inference);
                }
                existing.metadata.output_token_limit_reached |= output_token_limit_reached;
                return;
            }

            if output_token_limit_reached {
                self.0.push(message.with_visibility(true, false));
            }
            return;
        }

        if let Some(last) = self
            .0
            .last_mut()
            .filter(|m| m.id.is_some() && m.id == message.id)
        {
            if message.metadata.inference.is_some() {
                last.metadata.inference = message.metadata.inference.clone();
            }
            last.metadata.output_token_limit_reached |= message.metadata.output_token_limit_reached;
            match (last.content.last_mut(), message.content.last()) {
                (
                    Some(MessageContentBlock::Text(ref mut last)),
                    Some(MessageContentBlock::Text(new)),
                ) if message.content.len() == 1
                    && last.annotations.as_ref().and_then(|a| a.audience.as_ref())
                        == new.annotations.as_ref().and_then(|a| a.audience.as_ref()) =>
                {
                    last.text.push_str(&new.text);
                }
                (
                    Some(MessageContentBlock::Thinking(ref mut last)),
                    Some(MessageContentBlock::Thinking(new)),
                ) if message.content.len() == 1
                    && (last.signature.is_empty() || new.signature == last.signature) =>
                {
                    // Merge cases:
                    //   - `last` is still unsigned (block in progress) — append
                    //     and adopt `new.signature` if it's the closing delta.
                    //   - signatures match — same block continuing.
                    // An unsigned delta arriving after a signed block belongs
                    // to the next block (signature-at-end streams emit the
                    // first text of block N+1 before its signature), so the
                    // outer match arm falls through to push it separately.
                    last.thinking.push_str(&new.thinking);
                    if !new.signature.is_empty() {
                        last.signature = new.signature.clone();
                    }
                }
                (_, _) => {
                    last.content.extend(message.content);
                }
            }
        } else {
            self.0.push(message);
        }
    }

    pub fn last(&self) -> Option<&Message> {
        self.0.last()
    }

    pub fn first(&self) -> Option<&Message> {
        self.0.first()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Message>,
    {
        for message in iter {
            self.push(message);
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Message> {
        self.0.iter()
    }

    pub fn pop(&mut self) -> Option<Message> {
        self.0.pop()
    }

    pub fn remove(&mut self, index: usize) -> Message {
        self.0.remove(index)
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn filtered_messages<F>(&self, filter: F) -> Vec<Message>
    where
        F: Fn(&MessageMetadata) -> bool,
    {
        self.0
            .iter()
            .filter(|msg| filter(&msg.metadata))
            .cloned()
            .collect()
    }

    pub fn agent_visible_messages(&self) -> Vec<Message> {
        self.0
            .iter()
            .filter(|message| message.metadata.agent_visible)
            .map(Message::agent_visible_content)
            .filter(|message| !message.content.is_empty())
            .collect()
    }

    pub fn user_visible_messages(&self) -> Vec<Message> {
        self.0
            .iter()
            .filter(|message| message.metadata.user_visible)
            .map(Message::user_visible_content)
            .filter(|message| !message.content.is_empty())
            .collect()
    }

    fn validate(self) -> Result<Self, InvalidConversation> {
        let (_messages, issues) = fix_messages(self.0.clone());
        if !issues.is_empty() {
            let reason = issues.join("\n");
            Err(InvalidConversation {
                reason,
                conversation: self,
            })
        } else {
            Ok(self)
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::empty()
    }
}

impl IntoIterator for Conversation {
    type Item = Message;
    type IntoIter = std::vec::IntoIter<Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Conversation {
    type Item = &'a Message;
    type IntoIter = std::slice::Iter<'a, Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
