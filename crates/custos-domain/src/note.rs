//! Note & Scratchpad Domain Models
//!
//! Models versioned markdown notes, research scratchpads, and task annotations
//! with cryptographic content addressing and artifact lineage integration.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::{digest, new_id};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRecord {
    pub id: String,
    pub title: String,
    pub content: String,
    pub version: u32,
    pub content_hash: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteVersionRecord {
    pub id: String,
    pub note_id: String,
    pub version: u32,
    pub content: String,
    pub content_hash: String,
    pub created_at: i64,
}

impl NoteRecord {
    pub fn new(
        title: impl Into<String>,
        content: impl Into<String>,
        session_id: Option<String>,
        task_id: Option<String>,
        tags: Vec<String>,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        let content = content.into();
        if title.trim().is_empty() {
            return Err(DomainError::Validation("Note title cannot be empty".into()));
        }
        let now = chrono::Utc::now().timestamp();
        let content_hash = digest(content.as_bytes());
        Ok(Self {
            id: new_id("note"),
            title,
            content,
            version: 1,
            content_hash,
            session_id,
            task_id,
            tags,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update(
        &mut self,
        title: Option<String>,
        content: String,
        tags: Option<Vec<String>>,
    ) -> Result<NoteVersionRecord, DomainError> {
        if let Some(t) = title {
            if t.trim().is_empty() {
                return Err(DomainError::Validation("Note title cannot be empty".into()));
            }
            self.title = t;
        }
        if let Some(t) = tags {
            self.tags = t;
        }
        let now = chrono::Utc::now().timestamp();
        let old_content = std::mem::replace(&mut self.content, content);
        let old_hash = std::mem::replace(&mut self.content_hash, digest(self.content.as_bytes()));
        let old_version = self.version;
        self.version += 1;
        self.updated_at = now;

        Ok(NoteVersionRecord {
            id: format!("{}_v{}", self.id, old_version),
            note_id: self.id.clone(),
            version: old_version,
            content: old_content,
            content_hash: old_hash,
            created_at: self.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_lifecycle_and_versioning() {
        let mut note = NoteRecord::new(
            "Hypothesis on Latent Space",
            "# Hypothesis\nThe model exhibits linear latent structure.",
            Some("session_123".into()),
            None,
            vec!["physics".into(), "latent".into()],
        )
        .expect("valid note");

        assert_eq!(note.version, 1);
        assert!(!note.content_hash.is_empty());

        let prev = note
            .update(
                Some("Hypothesis on Latent Space (Revised)".into()),
                "# Hypothesis\nRevised with empirical ablation results.".into(),
                None,
            )
            .expect("update note");

        assert_eq!(note.version, 2);
        assert_eq!(prev.version, 1);
        assert_eq!(prev.content, "# Hypothesis\nThe model exhibits linear latent structure.");
        assert_ne!(note.content_hash, prev.content_hash);
    }
}
