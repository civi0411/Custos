//! Verifiable Claim Objects in Research & Reasoning

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub supported_by: Vec<String>,
    pub refuted_by: Vec<String>,
}
