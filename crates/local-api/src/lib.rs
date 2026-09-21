//! Custos Local API Contracts
//!
//! Protocol types for IPC communication between CLI / UI / VS Code and custosd.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
