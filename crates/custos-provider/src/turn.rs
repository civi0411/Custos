use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Yêu cầu (Intent) để thực hiện một lượt gọi LLM có cấu trúc.
/// Đây là bản nâng cấp từ `ProviderRequest` phẳng cũ, hỗ trợ đầy đủ
/// bối cảnh (Task/Run), quyền riêng tư, và giới hạn chi phí.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurnRequest {
    pub task_id: Uuid,
    pub run_id: Uuid,
    pub attempt_id: Uuid,
    
    /// ID của ProviderConnection thực sự sẽ được dùng.
    pub connection_id: String,
    
    /// Model mong muốn ban đầu (có thể bị thay đổi nếu 9Router logic kích hoạt fallback).
    pub requested_model: String,
    
    /// Tin nhắn có cấu trúc (có thể chứa text, image, cache breakpoints).
    pub structured_messages: Vec<crate::types::conversation::message::Message>,
    
    /// Danh sách công cụ (MCP, Native) được phép dùng trong lượt này.
    pub tool_schemas: Vec<crate::tool::ToolDefinition>,
    
    pub required_capabilities: Vec<String>,
    pub source_scope_digest: Option<String>,
    
    #[serde(default = "default_privacy")]
    pub privacy_class: String,
    
    /// Ngân sách tối đa cho lượt này (USD).
    pub budget_reservation: Option<f64>,
    
    pub deadline: Option<DateTime<Utc>>,
    pub cancellation_token: Option<String>,
    
    /// Nhiệt độ, top_p, v.v.
    pub decoding_options: Option<Value>,
}

fn default_privacy() -> String {
    "standard".to_string()
}

/// Sự kiện Streaming phát ra từ Provider (hoặc Proxy 9Router).
/// Tách biệt rõ "requested" và "actual" để phục vụ auditing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurnEvent {
    pub attempt_id: Uuid,
    pub sequence: u64,
    
    /// Model thực tế đã trả lời (cực kỳ quan trọng nếu có account/model fallback).
    pub actual_provider: Option<String>,
    pub actual_model: Option<String>,
    pub actual_transport: Option<String>,
    
    /// Nội dung luồng stream.
    pub delta: TurnDelta,
    
    /// known | estimated | unknown
    pub usage_certainty: Option<String>, 
    
    pub provider_response_id: Option<String>,
}

/// Các loại Delta siêu mịn hỗ trợ Reasoning (o1/o3) và Tool Call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum TurnDelta {
    Text { text: String },
    Reasoning { text: String }, // Cho các model có quá trình "Thinking"
    ToolCall { id: String, name: Option<String>, arguments_chunk: String },
    ToolCallComplete { id: String },
    Usage { 
        input_tokens: u64, 
        output_tokens: u64, 
        cache_read_tokens: Option<u64>, 
        cache_write_tokens: Option<u64> 
    },
    Completed { finish_reason: Option<String> },
    Failed { error_class: String, message: String },
    Uncertain { reason: String }, // Khẩn cấp khi bị ngắt kết nối giữa chừng
}
