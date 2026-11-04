//! Scientific Artifact Annotation Domain Model
//!
//! Models interactive annotations on research artifacts:
//! text selection spans, image pin-drop coordinates (X%, Y%),
//! DOM element selectors, and side-chat branching links.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnnotationTarget {
    Text {
        start_offset: usize,
        end_offset: usize,
        exact_text: String,
    },
    ImagePin {
        x_ratio: f32, // 0.0 .. 1.0 (normalized coordinate)
        y_ratio: f32, // 0.0 .. 1.0 (normalized coordinate)
        pin_number: u32,
    },
    DomElement {
        selector: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationStatus {
    Pending,
    Submitted,
    Addressed,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationRecord {
    pub id: String,
    pub artifact_path: String,
    pub artifact_version: u32,
    pub target: AnnotationTarget,
    pub note: String,
    pub actor: String,
    pub status: AnnotationStatus,
    pub side_chat_session_id: Option<String>,
    pub submitted_turn_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AnnotationRecord {
    pub fn new_text(
        artifact_path: impl Into<String>,
        artifact_version: u32,
        start_offset: usize,
        end_offset: usize,
        exact_text: impl Into<String>,
        note: impl Into<String>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            id: new_id("ann"),
            artifact_path: artifact_path.into(),
            artifact_version,
            target: AnnotationTarget::Text {
                start_offset,
                end_offset,
                exact_text: exact_text.into(),
            },
            note: note.into(),
            actor: actor.into(),
            status: AnnotationStatus::Pending,
            side_chat_session_id: None,
            submitted_turn_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn new_image_pin(
        artifact_path: impl Into<String>,
        artifact_version: u32,
        x_ratio: f32,
        y_ratio: f32,
        pin_number: u32,
        note: impl Into<String>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            id: new_id("ann"),
            artifact_path: artifact_path.into(),
            artifact_version,
            target: AnnotationTarget::ImagePin {
                x_ratio,
                y_ratio,
                pin_number,
            },
            note: note.into(),
            actor: actor.into(),
            status: AnnotationStatus::Pending,
            side_chat_session_id: None,
            submitted_turn_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_side_chat(mut self, session_id: impl Into<String>) -> Self {
        self.side_chat_session_id = Some(session_id.into());
        self
    }

    pub fn mark_submitted(&mut self, turn_id: impl Into<String>) {
        self.status = AnnotationStatus::Submitted;
        self.submitted_turn_id = Some(turn_id.into());
    }

    pub fn mark_addressed(&mut self) {
        self.status = AnnotationStatus::Addressed;
    }

    pub fn mark_dismissed(&mut self) {
        self.status = AnnotationStatus::Dismissed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_pin_annotation_lifecycle() {
        let mut ann = AnnotationRecord::new_image_pin(
            "figures/ablation_loss.png",
            1,
            0.45,
            0.62,
            1,
            "Switch Y axis to logarithmic scale",
            "researcher",
        );
        assert_eq!(ann.status, AnnotationStatus::Pending);
        assert!(ann.side_chat_session_id.is_none());

        ann.mark_submitted("turn_42");
        assert_eq!(ann.status, AnnotationStatus::Submitted);
        assert_eq!(ann.submitted_turn_id.as_deref(), Some("turn_42"));

        ann = ann.with_side_chat("session_side_01");
        assert_eq!(ann.side_chat_session_id.as_deref(), Some("session_side_01"));
    }
}
