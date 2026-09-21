# Quy Ước Đặt Tên Chuẩn Xác (Naming Conventions)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần XII (§34) Canonical Specification

Để đảm bảo tính nhất quán tuyệt đối giữa mã nguồn Rust, schemas, tài liệu, API và cơ sở dữ liệu, Custos áp dụng hệ thống quy ước đặt tên chuẩn hóa theo bảng sau:

---

## Bảng Tổng Hợp Quy Ước Đặt Tên Toàn Hệ Thống

| Tầng / Thành phần | Quy ước định dạng | Ví dụ cụ thể | Ghi chú & Phạm vi áp dụng |
|---|---|---|---|
| **Tên sản phẩm (Product)** | PascalCase | `Custos` | Tên chính thức của runtime |
| **Tên Repository** | kebab-case | `custos` | Tên repo trên GitHub / Git forge |
| **Rust Crates** | kebab-case | `custos-kernel`, `custos-gateway`, `custos-cognitive` | Tên crate trong Cargo workspace |
| **Binary executables** | lowercase | `custosd`, `custos` | `custosd` (daemon), `custos` (CLI) |
| **Modules (Rust/Python)** | snake_case | `task_state`, `event_store`, `context_scoring` | Tên file `.rs`, `.py` và modules |
| **Domain Entities** | PascalCase, singular | `Task`, `Contract`, `Artifact`, `Run` | Structs thực thể nghiệp vụ có ID |
| **Value Objects** | PascalCase | `TaskId`, `ExecutionPermit`, `ContextPack` | Giá trị bất biến, định danh typed |
| **Enums** | PascalCase | `TaskStatus`, `EvidenceLevel`, `RiskTier` | Kiểu liệt kê trạng thái |
| **Traits / Ports** | PascalCase, hậu tố `Port` | `ProviderPort`, `JudgmentPort`, `StoragePort` | Giao diện trừu tượng ở ranh giới |
| **Functions & Methods** | snake_case, động từ | `create_task()`, `verify_evidence()`, `commit_step()` | Hành vi nghiệp vụ rõ ràng |
| **Commands (CQRS)** | PascalCase, mệnh lệnh | `CreateTask`, `DispatchWorker`, `CancelTask` | Ý định thay đổi trạng thái |
| **Events (Event Sourced)** | PascalCase, thì quá khứ | `TaskCreated`, `WorkerDispatched`, `StepVerified` | Sự kiện bất biến đã xảy ra |
| **Schemas (Versioned)** | `<ns>.<name>.v<n>` | `custos.task.v1`, `custos.permit.v1` | Định danh schema JSON / Protobuf |
| **REST / IPC API Endpoints**| kebab-case, số nhiều | `/v1/tasks`, `/v1/execution-permits` | Đường dẫn HTTP / RPC |
| **CLI Commands** | kebab-case | `custos task start`, `custos audit trace` | Lệnh trong terminal |
| **Domain Packs** | kebab-case | `engineering`, `research`, `personal` | Tên gói miền nghiệp vụ |
| **Task Types** | `<domain>.<type>` | `engineering.bug_fix`, `research.literature_scan` | Định danh loại task |
| **Workflows** | `<domain>.<name>@<v>` | `engineering.bug_fix@1`, `research.claim_verify@2` | Quy trình có phiên bản |
| **Worker Roles** | `<domain>.<role>` | `engineering.explorer`, `engineering.patcher` | Vai trò ngắn hạn của worker |
| **AI Providers** | lowercase | `codex`, `claude`, `antigravity`, `ollama` | Mã định danh provider adapter |
| **Database Tables** | snake_case, số nhiều | `tasks`, `task_events`, `execution_permits` | Bảng trong SQLite |
| **Foreign Keys / Columns** | snake_case | `task_id`, `created_at`, `payload_hash` | Cột trong SQLite |
| **Error Types** | PascalCase, hậu tố `Error` | `TaskNotFoundError`, `PermitExpiredError` | Phân loại lỗi chuẩn |
| **Metrics / Telemetry** | snake_case, tiền tố `custos_`| `custos_task_duration_seconds`, `custos_tokens_used` | Prometheus / OpenTelemetry |
| **Environment Variables** | UPPER_SNAKE_CASE | `CUSTOS_HOME`, `CUSTOS_LOG_LEVEL` | Cấu hình qua môi trường |

---

## Nguyên Tắc Đặt Tên Bổ Sung

1. **Rõ ràng hơn ngắn gọn:** Tránh viết tắt gây hiểu lầm (ví dụ: dùng `verification_result` thay vì `vr_res`).
2. **Không dùng tên riêng thiếu ngữ nghĩa:** Không sử dụng các từ tự tạo không phản ánh bản chất kỹ thuật (loại bỏ JEP, UJE; thay bằng `DecisionContract`, `ExecutionPermit`).
3. **Phân biệt rành mạch Command và Event:** Command là yêu cầu hành động (chưa chắc chắn thành công), Event là sự kiện đã xảy ra và ghi vào Event Store (bất biến).
