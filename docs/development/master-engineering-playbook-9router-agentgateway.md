# CẨM NANG THIẾT KẾ MASTER: ĐẶC TẢ CHI TIẾT TỪNG PHÂN HỆ CUSTOS SAU KHI HẤP THỤ 9ROUTER & AGENTGATEWAY
## Master Architecture & Implementation Specification for Custos Engineering Team

> **Tài liệu tham chiếu:** `Custos Master Spec (Custos.md)`, `system-design-9router-agentgateway-custos.md`, `9router-custos-integration-plan.md`.  
> **Mục tiêu:** Bản đặc tả kỹ thuật chi tiết mức code (Low-Level Design - LLD) cho từng phân hệ trong Custos sau khi mổ xẻ và hấp thụ 9Router cùng AgentGateway. Kỹ sư có thể dùng tài liệu này làm bản vẽ thi công chính thức.

---

## 1. TỔNG QUAN PHÂN RÃ HỆ THỐNG (SYSTEM DECOMPOSITION)

Sau khi mổ xẻ mã nguồn thực tế của `9router` (Node.js) và `agentgateway` (Rust), hệ thống Custos được chia thành 5 phân hệ mục tiêu trong các crate nội bộ:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           CUSTOS OPERATING SYSTEM                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│ [PHÂN HỆ 3] Context Compiler & RTK Engine       (crates/custos-runtime/context) │
│             Pipeline 8 bước, Headroom calculation, OmissionRecord audit         │
├─────────────────────────────────────────────────────────────────────────────────┤
│ [PHÂN HỆ 1] Model Connectivity & Catalog Plane  (crates/custos-provider)        │
│             ModelTurn contract, ModelDescriptor, Pricing, TokenCostCalculator   │
├─────────────────────────────────────────────────────────────────────────────────┤
│ [PHÂN HỆ 2] Protocol Adapters & Fallback Cascade (crates/custos-adapters)       │
│             Translators (Claude, OpenAI, Gemini), CooldownTracker, Failover     │
├─────────────────────────────────────────────────────────────────────────────────┤
│ [PHÂN HỆ 4] Federated MCP & Sandbox Defense     (crates/custos-adapters/mcp)    │
│             McpHub, Tool Mergestream, 4-Layer Sandbox Guard, ExecutionPermit    │
├─────────────────────────────────────────────────────────────────────────────────┤
│ [PHÂN HỆ 5] Evidence Ledger & Completion Gate   (custos-persistence & core)     │
│             Attempt Ledger, Actual Model Record, 3-Tier Verifier                │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. ĐẶC TẢ PHÂN HỆ 1: MODEL CONNECTIVITY & CATALOG (`crates/custos-provider`)

### 2.1. Hợp đồng Giao tiếp Chuẩn: `ModelTurnRequest` & `ModelTurnEvent`
File đích: `crates/custos-provider/src/turn.rs`

Khắc phục triệt để hạn chế của prompt phẳng cũ, hợp đồng Model Turn mới chứa đầy đủ ngữ cảnh có cấu trúc, budget, công cụ và reasoning:

```rust
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::Stream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurnRequest {
    pub attempt_id: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub model_id: String,
    pub messages: Vec<TurnMessage>,
    pub tools: Vec<TurnToolDefinition>,
    pub reasoning: Option<TurnReasoningConfig>,
    pub budget_reservation_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMessage {
    pub role: TurnRole,
    pub content: Vec<TurnContentPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnRole {
    System,
    User,
    Assistant,
    ToolResult { tool_call_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnContentPart {
    Text(String),
    Image { media_type: String, data: Vec<u8> },
    ToolCall { id: String, name: String, arguments: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnReasoningConfig {
    pub effort: ReasoningEffort, // Low, Medium, High
    pub max_reasoning_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelTurnEvent {
    ContentDelta { delta: String },
    ReasoningDelta { text: String },
    ToolCallRequested {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    UsageReported { usage: TurnUsage },
    TurnCompleted { finish_reason: FinishReason },
    TurnFailed { error: String, retryable: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnUsage {
    pub prompt_tokens: u32,
    pub cached_prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
    Error,
}
```

### 2.2. Trait `ModelPort`
File đích: `crates/custos-provider/src/port.rs`

```rust
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Rate limited (HTTP 429): {0}")]
    RateLimited(String),
    #[error("Service overloaded (HTTP 503): {0}")]
    Overloaded(String),
    #[error("Context length exceeded: {0}")]
    ContextExceeded(String),
    #[error("Authentication failed: {0}")]
    AuthError(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
}

#[async_trait]
pub trait ModelPort: Send + Sync {
    async fn execute_turn(
        &self,
        request: ModelTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ModelTurnEvent, ProviderError>> + Send>>, ProviderError>;

    async fn estimate_cost(&self, request: &ModelTurnRequest) -> Result<rust_decimal::Decimal, ProviderError>;
}
```

### 2.3. Model Catalog & Pricing Engine (Kế thừa từ 9Router)
File đích: `crates/custos-provider/src/catalog/pricing.rs` và `capabilities.rs`

```rust
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub model_id: String,
    pub provider_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub prompt_per_million: Decimal,
    pub cached_prompt_per_million: Decimal,
    pub output_per_million: Decimal,
    pub reasoning_per_million: Decimal,
}

pub struct TokenCostCalculator;

impl TokenCostCalculator {
    pub fn calculate(usage: &TurnUsage, pricing: &ModelPricing) -> Decimal {
        let million = Decimal::from(1_000_000);
        
        let regular_prompt = Decimal::from(usage.prompt_tokens.saturating_sub(usage.cached_prompt_tokens));
        let cached_prompt = Decimal::from(usage.cached_prompt_tokens);
        let output = Decimal::from(usage.completion_tokens);
        let reasoning = Decimal::from(usage.reasoning_tokens);

        let cost_prompt = (regular_prompt / million) * pricing.prompt_per_million;
        let cost_cached = (cached_prompt / million) * pricing.cached_prompt_per_million;
        let cost_output = (output / million) * pricing.output_per_million;
        let cost_reasoning = (reasoning / million) * pricing.reasoning_per_million;

        cost_prompt + cost_cached + cost_output + cost_reasoning
    }
}
```

---

## 3. ĐẶC TẢ PHÂN HỆ 2: PROTOCOL TRANSLATORS & FALLBACK CASCADE (`crates/custos-adapters`)

### 3.1. Pure Rust Protocol Translators (Kế thừa AgentGateway `crates/llm`)
File đích: `crates/custos-adapters/src/providers/translator/`
- `anthropic.rs`: Serialize sang JSON endpoint `/v1/messages`. Cấu hình header `anthropic-version: 2023-06-01`, bọc reasoning blocks trong trường `thinking: { type: "enabled", budget_tokens: ... }`. Parse SSE stream các event `content_block_delta`, `thinking_delta`, `tool_use`.
- `openai.rs`: Serialize sang JSON endpoint `/v1/chat/completions`. Parse SSE chunks `delta.content`, `delta.tool_calls`, `usage`.
- `gemini.rs`: Serialize sang JSON endpoint Google Cloud/Vertex contents API (`contents: [{ role, parts }]`). Parse streaming chunks `candidates[0].content.parts`, functionCall và usageMetadata.
- `bedrock.rs`: Serialize sang AWS Bedrock Converse API (`messages`, `toolConfig`, `inferenceConfig`). Parse converse-stream chunks `contentBlockDelta`, `messageStop`, `metadata.usage`.

### 3.2. Fallback Cascade & Cooldown Tracker (Kế thừa 9Router `accountFallback.js`)
File đích: `crates/custos-adapters/src/providers/fallback.rs`

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct CooldownTracker {
    // Map từ provider_id -> thời điểm hết hạn phạt
    cooldowns: RwLock<HashMap<String, Instant>>,
    default_cooldown: Duration,
}

impl CooldownTracker {
    pub fn new(default_cooldown: Duration) -> Self {
        Self {
            cooldowns: RwLock::new(HashMap::new()),
            default_cooldown,
        }
    }

    pub async fn is_available(&self, provider_id: &str) -> bool {
        let lock = self.cooldowns.read().await;
        if let Some(expiry) = lock.get(provider_id) {
            Instant::now() >= *expiry
        } else {
            true
        }
    }

    pub async fn mark_cooldown(&self, provider_id: &str, custom_duration: Option<Duration>) {
        let mut lock = self.cooldowns.write().await;
        let duration = custom_duration.unwrap_or(self.default_cooldown);
        lock.insert(provider_id.to_string(), Instant::now() + duration);
    }
}

pub struct FallbackCascade {
    cooldown_tracker: Arc<CooldownTracker>,
    candidate_providers: Vec<Arc<dyn ModelPort>>,
}

impl FallbackCascade {
    pub async fn execute_with_failover(
        &self,
        mut request: ModelTurnRequest,
    ) -> Result<(Pin<Box<dyn Stream<Item = Result<ModelTurnEvent, ProviderError>> + Send>>, String), ProviderError> {
        for provider in &self.candidate_providers {
            let provider_id = provider.provider_id();
            if !self.cooldown_tracker.is_available(&provider_id).await {
                continue; // Bỏ qua nếu đang trong thời gian phạt 429
            }

            match provider.execute_turn(request.clone()).await {
                Ok(stream) => {
                    // Thành công: trả về stream và TÊN MODEL THỰC TẾ ĐƯỢC CHẠY
                    return Ok((stream, provider_id));
                }
                Err(ProviderError::RateLimited(err)) | Err(ProviderError::Overloaded(err)) => {
                    tracing::warn!("Provider {} rate-limited: {}. Entering cooldown and falling back.", provider_id, err);
                    self.cooldown_tracker.mark_cooldown(&provider_id, Some(Duration::from_secs(60))).await;
                    // Tiếp tục thử provider tiếp theo trong cascade
                }
                Err(e) => return Err(e), // Lỗi nghiêm trọng (sai auth, bad request) thì dừng ngay
            }
        }
        Err(ProviderError::Overloaded("All fallback providers are currently unavailable or in cooldown".into()))
    }
}
```

---

## 4. ĐẶC TẢ PHÂN HỆ 3: CONTEXT COMPILER & RTK ENGINE (`crates/custos-runtime`)

### 4.1. Pipeline 8 Bước của Context Compiler
File đích: `crates/custos-runtime/src/context/compiler.rs`
1. **Scope Filter:** Chỉ nạp các file nằm trong whitelist của `TaskScope`.
2. **Sensitivity Classify:** Gắn cờ độ nhạy bảo mật cho tài liệu (Public, Internal, Confidential).
3. **Egress Filter:** Chặn các file nhạy cảm nếu task cấu hình `LocalOnly = true`.
4. **Secret Scanner:** Tự động lọc sạch API keys, passwords, private keys.
5. **Version Select:** Ghim cứng phiên bản file theo Commit SHA hiện tại.
6. **Hybrid Retrieval:** Kết hợp AST Tree-sitter + Ripgrep + Vector search.
7. **Structural Rank:** Ưu tiên định nghĩa hàm, call graph, unit tests.
8. **Token Budget Fit (RTK):** Tính toán Headroom và cắt tỉa theo thuật toán RTK.

### 4.2. RTK Headroom Pruner (Kế thừa 9Router `open-sse/rtk/headroom.js`)
File đích: `crates/custos-runtime/src/context/rtk.rs`

```rust
pub struct RtkHeadroomManager {
    safety_headroom_tokens: u32, // Dự trữ an toàn, vd: 4096 tokens
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmissionRecord {
    pub message_index: usize,
    pub original_token_count: u32,
    pub pruned_token_count: u32,
    pub reason: String,
    pub content_hash: [u8; 32], // Hash SHA-256 đoạn bị cắt để đối soát bằng chứng
}

impl RtkHeadroomManager {
    pub fn prune_messages_to_fit(
        &self,
        messages: &mut Vec<TurnMessage>,
        context_window: u32,
    ) -> Vec<OmissionRecord> {
        let max_allowed = context_window.saturating_sub(self.safety_headroom_tokens);
        let mut omissions = Vec::new();

        // Duyệt ngược từ các message cũ nhất (ưu tiên cắt tỉa tool call output cũ)
        for (idx, msg) in messages.iter_mut().enumerate() {
            // Nếu là kết quả ToolCallResult quá dài từ các bước trước -> Cắt ngắn lại
            // Ghi nhận OmissionRecord với SHA-256 hash
        }
        omissions
    }
}
```

---

## 5. ĐẶC TẢ PHÂN HỆ 4: FEDERATED MCP & SANDBOX DEFENSE (`crates/custos-adapters`)

### 5.1. Federated MCP Hub (Kế thừa AgentGateway `mergestream.rs`)
File đích: `crates/custos-adapters/src/mcp/hub.rs`
- **Session Pool:** Khởi chạy và quản lý các tiến trình Stdio con (`tokio::process::Command`) hoặc kết nối SSE.
- **Auto-Namespacing:** Khi đăng ký nhiều MCP server (ví dụ `github` và `filesystem`), Hub tự động đổi tên tool thành `github__create_issue` và `filesystem__write_file` để ngăn chặn xung đột tên tool giữa các server.

### 5.2. Hàng Rào Phòng Thủ Sandbox 4 Lớp (Defense-in-Depth)
File đích: `crates/custos-adapters/src/sandbox/guard.rs`

```rust
pub struct SandboxGuard;

impl SandboxGuard {
    pub async fn verify_and_intercept_tool_call(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        permit: Option<&CapabilityPermit>,
    ) -> Result<(), SandboxViolation> {
        // LỚP 1: Kiểm tra Whitelist Policy (TaskScope)
        if !self.is_within_task_scope(tool_name, arguments) {
            return Err(SandboxViolation::OutOfScope);
        }

        // LỚP 2: Băm tham số (Parameter Hash) và kiểm tra Cryptographic Permit
        let param_hash = sha256_digest(arguments);
        match permit {
            Some(p) if p.param_hash == param_hash && !p.is_expired() => {
                // Được phép chạy
            }
            _ => {
                // Chưa có Permit -> Kích hoạt cơ chế Human In The Loop
                return Err(SandboxViolation::RequireUserApproval {
                    tool_name: tool_name.to_string(),
                    arguments: arguments.clone(),
                });
            }
        }

        // LỚP 3: Cách ly tiến trình (Seatbelt trên macOS, Bubblewrap trên Linux)
        // LỚP 4: Thực thi trên Git Worktree tạm thời (isolated_worktree)
        Ok(())
    }
}
```

---

## 6. ĐẶC TẢ PHÂN HỆ 5: EVIDENCE LEDGER & COMPLETION GATE

### 6.1. Attempt Ledger & Đối Soát Chi Phí Minh Bạch
File đích: `crates/custos-persistence/src/ledger.rs`

Mỗi lần phát lệnh gọi Model, hệ thống lưu vào SQLite bảng `model_attempts`:
```sql
CREATE TABLE IF NOT EXISTS model_attempts (
    attempt_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    requested_model TEXT NOT NULL,
    actual_model TEXT NOT NULL,      -- Ghi nhận trung thực model đã chạy (kể cả khi fallback)
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    billed_cost_usd REAL NOT NULL,
    latency_ms INTEGER NOT NULL,
    input_hash TEXT NOT NULL,         -- SHA-256 của ContextPack
    output_hash TEXT NOT NULL,        -- SHA-256 của kết quả trả về
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 6.2. Completion Gate & Nguyên Tắc REAL
File đích: `crates/custos-core/src/completion_gate/gate.rs`

```rust
pub enum EvidenceKind {
    DeterministicFact { exit_code: i32, receipt_id: String }, // Tier 1: Test exit 0
    StructuralExtraction { file_path: PathBuf, ast_span: (usize, usize) }, // Tier 2: AST Span
    SemanticJudgment { rubric_id: String, score: f32 }, // Tier 3: Human sign-off / Rubric
}

pub struct CompletionGate;

impl CompletionGate {
    pub fn evaluate_task_completion(&self, task: &TaskContract, evidence: &[EvidenceRecord]) -> bool {
        // Invariant: Mọi tiêu chí nghiệm thu (Acceptance Criteria) trong TaskContract 
        // bắt buộc phải có ít nhất 1 EvidenceRecord hợp lệ.
        // Lời nói "I have completed the task" từ ModelTurnEvent có giá trị bằng 0!
        task.criteria.iter().all(|criterion| {
            evidence.iter().any(|e| e.criterion_id == criterion.id && e.is_valid())
        })
    }
}
```

---

## 7. BẢNG KIỂM TRA ĐỐI SOÁT TRƯỚC KHI SUBMIT PR (ENGINEERING CHECKLIST)

Trước khi mở PR, đồng đội của bạn cần tick đủ các mục sau:
- [ ] Không có mã JavaScript/Node.js nào được chạy trong runtime của Custos (Toàn bộ là Rust native).
- [ ] Mọi trường hợp HTTP 429 đều được xử lý êm qua `CooldownTracker` và ghi nhận đúng `actual_model` vào Ledger.
- [ ] Tất cả tool calls của MCP đều đi qua `SandboxGuard` để băm tham số và kiểm tra `CapabilityPermit`.
- [ ] Không có tình trạng context overflow nhờ module `rtk.rs` tính toán Headroom an toàn.
- [ ] Hoàn thành Task bằng test exit 0 (Evidence Tier 1) thay vì phụ thuộc vào câu trả lời của LLM.
