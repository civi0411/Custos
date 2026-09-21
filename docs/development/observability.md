# Giám Sát & Đo Lường Hệ Thống (Observability & Metrics)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần VI (§34) Canonical Specification

Hệ thống giám sát của Custos được thiết kế dựa trên tiêu chuẩn mở **OpenTelemetry**, cung cấp khả năng quan sát toàn diện mọi khía cạnh vận hành cục bộ mà vẫn bảo vệ quyền riêng tư của người dùng.

---

## 1. Mô Hình Phân Cấp Dấu Vết (Trace Span Hierarchy)

Mọi hoạt động trong một Task được cấu trúc theo cây span rõ ràng:

```text
Workspace
└── Task Trace (task_id: "tsk_01J8N6...")
    ├── Step Spans (step: 1, role: "engineering.explorer")
    │   ├── Context Compilation Span (scoring & token ranking)
    │   ├── Provider Call Span (model: "codex", streaming)
    │   └── Decision Case Span (RDC request/response)
    ├── Step Spans (step: 2, role: "engineering.patcher")
    │   ├── Tool Execution Span (tool: "apply_patch", sandbox)
    │   └── Verifier Span (tool: "cargo_test", receipt_id)
    └── Human Approval Span (waiting for user exact-payload approval)
```

---

## 2. Các Chỉ Số Đo Lường Chính (Core Metrics)

- **`custos_task_total`:** Tổng số task phân loại theo trạng thái (Success, Failed, Cancelled).
- **`custos_step_duration_seconds`:** Thời gian thực thi của từng bước theo từng vai trò worker.
- **`custos_tokens_consumed_total`:** Số token input/output tiêu tốn phân loại theo model provider.
- **`custos_judgment_cost_ratio`:** Tỉ lệ chi phí giữa System One (phán đoán nhanh) và System Two (LLM suy luận sâu).
- **`custos_human_interruptions_total`:** Số lần phải dừng lại xin ý kiến hoặc phê duyệt từ con người.
- **`custos_verifier_pass_ratio`:** Tỉ lệ vượt qua bài kiểm tra tự động ngay trong lần thử đầu tiên.

---

## 3. Ghi Nhật Ký Có Cấu Trúc (Structured Logging & Redaction)

- Sử dụng thư viện `tracing` chuẩn của Rust, xuất dữ liệu dạng JSON Lines (`.jsonl`).
- **Khử nhạy cảm trước khi ghi log:** Tự động lọc sạch token bí mật và dữ liệu nhạy cảm trước khi lưu xuống đĩa.
- Raw prompts mặc định bị tắt; người dùng chỉ bật khi cần debug chuyên sâu (`CUSTOS_LOG_PROMPTS=1`).
