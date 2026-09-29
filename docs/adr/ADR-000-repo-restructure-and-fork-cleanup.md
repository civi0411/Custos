# ADR-000: Tinh Gọn Cấu Trúc Repo & Chuẩn Hóa Kiến Trúc Sovereign Custos

- **Trạng thái**: Accepted
- **Ngày**: 2026-09-29
- **Tác giả**: Custos Architecture Core Team
- **Liên quan**: [ADR-001](file:///Users/mac/Project/AgentHub/Custos/docs/adr/action-intent-and-permit-contracts.md), [ADR-002](file:///Users/mac/Project/AgentHub/Custos/docs/adr/task-state-machine-alignment.md)

---

## 1. Bối cảnh & Vấn đề

Custos khởi nguồn từ một nền tảng fork kết hợp với các thử nghiệm micro-crate. Qua quá trình phát triển nhanh, repository đã phình to lên tới **42 crate Rust** trong workspace (và hơn 76 thư mục rải rác), dẫn đến các vấn đề nghiêm trọng:

1. **Phân mảnh quá mức (Over-fragmentation)**:
   - Các logic đơn giản (< 100 dòng code) như `crates/adapters/judgments/contracts`, `jev`, `onnx`, `rules` bị xé thành 4 crate riêng biệt.
   - Nhóm LLM Provider bị chia thành 8 crate (`claude`, `codex`, `local-model`, `fake`, `antigravity`, `custos-providers`, `custos-provider-sdk`, `custos-provider-types`).
   - Crate packs bị chia thành 3 crate riêng lẻ (`custos-packs-engineering`, `custos-packs-research`, `custos-packs-assistant`).
2. **Trùng lặp và mờ nhạt ranh giới trách nhiệm (Duplicate & Blurry Boundaries)**:
   - `custos-context` vs `custos-context-management` cùng quản lý context.
   - `custos-security` vs `custos-gateway` cùng xử lý policy và sandbox.
   - `custos-engine` vs `custos-agent` vs `custos-session` cùng quản lý lifecycle thực thi.
3. **Tàn dư Goose Fork**:
   - Vẫn còn các module, struct mang tên tiền tố hoặc logic của Goose chưa được thanh lọc hoàn toàn.
4. **Chi phí bảo trì và thời gian biên dịch (Compilation Overhead)**:
   - Quá nhiều ranh giới crate làm tăng thời gian build, làm phức tạp dependency graph và gây khó khăn cho việc refactor.

---

## 2. Quyết định (Decision)

Chúng tôi quyết định tiến hành **tinh chỉnh toàn diện (Master Refactoring)**, quy hoạch lại toàn bộ repository theo **6 Nguyên Tắc Cốt Lõi**, rút gọn từ **42 crates xuống đúng 12 crates chuẩn hóa**:

```
========================================================================================
                     CUSTOS SOVEREIGN ARCHITECTURE — 12 CRATES
========================================================================================

 [1] crates/custos-domain        (SSOT Khế ước hạt nhân: Pure Rust, Zero I/O, No Async)
  │
 ├── [2] crates/custos-engine    (Execution Engine: State Machine, Cognitive Loop S1/S2, Scheduler)
 ├── [3] crates/custos-persistence (SQLite WAL, Runs, Tasks, Evidence Journal, Migrations)
 ├── [4] crates/custos-authority (Security, Policy Engine, Capability Gateway, Permits)
 ├── [5] crates/custos-evidence  (Evidence Recorder, Verifiers, JEV/Rules/ONNX Judgments, Proofs)
 ├── [6] crates/custos-context   (Context Budget, Windowing, Memory Hierarchy, AST Integration)
 ├── [7] crates/custos-providers (Unified LLM Providers: Claude, Codex, Local, Antigravity)
 ├── [8] crates/custos-adapters  (MCP Host/Client, Seatbelt/Bubblewrap Sandbox Drivers, Roaming)
 └── [9] crates/custos-packs     (3 Domain Packs: engineering, research, assistant)
  │
 ├── [10] crates/custos-cli      (Terminal Interface, Interactive TUI, REPL)
 ├── [11] crates/custos-daemon   (Local API REST/WebSocket/IPC, Background Service)
 └── [12] crates/custos-test-support (Test Fixtures, Mocks, Verification Harness)
========================================================================================
```

---

## 3. Sáu Nguyên Tắc Thiết Kế Lại

1. **MỘT CONCERN, MỘT CHỖ**:
   - Không có hai crate làm cùng một việc.
   - Không có hai file cho cùng một logic.
2. **ÍT CRATE HƠN LÀ TỐT HƠN**:
   - Mỗi crate chỉ tồn tại nếu có lý do biên dịch hoặc kiểm thử độc lập rõ ràng.
   - Module < 500 LOC thì tích hợp thành submodule trong crate cha, không tạo crate riêng.
   - Xóa bỏ toàn bộ crate placeholder hoặc crate rỗng.
3. **TÊN PHẢN ÁNH VAI TRÒ, KHÔNG PHẢI NGUỒN GỐC**:
   - Đặt tên chuẩn xác: `custos-authority`, `custos-evidence`, `custos-engine`.
   - Cấm dùng alias tạm bợ kiểu `CustosMode = GooseMode`.
4. **THƯ MỤC PHẲNG VÀ NÔNG**:
   - Cấu trúc thư mục tối đa 2 tầng (`crates/custos-<name>/src/...`).
   - Bỏ cấu trúc phân cấp lồng 4 tầng sâu như `crates/adapters/judgments/contracts`.
5. **DOMAIN LOGIC THUẦN TÚY (Pure Domain)**:
   - `custos-domain` chỉ chứa structs, enums, traits, pure validation logic.
   - KHÔNG phụ thuộc I/O, KHÔNG phụ thuộc `tokio`, async runtime hay external services.
6. **MỘT NGUỒN SỰ THẬT (Single Source of Truth - SSOT)**:
   - Các khế ước hạt nhân (`ActionIntentV1`, `TaskContractV1`, `EvidenceRecordV1`, `PermitV1`) được định nghĩa duy nhất tại `custos-domain`. Mọi crate khác phải tham chiếu đến đây.

---

## 4. Ma Trận Ánh Xạ Chuyển Đổi (Migration Matrix)

| Crate Hiện Tại | Trạng Thái | Crate Đích (Sau Tinh Chỉnh) | Module Đích |
| :--- | :--- | :--- | :--- |
| `crates/core/custos-domain` | **GIỮ & NÂNG CẤP** | `crates/custos-domain` | Root domain contracts |
| `crates/core/custos-kernel` | **MERGE** | `crates/custos-engine` | `task_kernel`, `scheduler` |
| `crates/runtime/custos-engine` | **MERGE** | `crates/custos-engine` | Core runtime loop |
| `crates/runtime/custos-session` | **MERGE** | `crates/custos-engine` | `session` |
| `crates/runtime/custos-agent` | **MERGE** | `crates/custos-engine` | `agent_loop` |
| `crates/runtime/custos-workflow` | **MERGE** | `crates/custos-engine` | `workflow` |
| `crates/runtime/custos-cognitive` | **MERGE** | `crates/custos-engine` | `cognitive` (S1/S2) |
| `crates/infrastructure/custos-persistence` | **GIỮ & NÂNG CẤP** | `crates/custos-persistence` | Root persistence |
| `crates/runtime/custos-security` | **MERGE** | `crates/custos-authority` | `security`, `policies` |
| `crates/runtime/custos-gateway` | **MERGE** | `crates/custos-authority` | `capability_gateway` |
| `crates/adapters/judgments/*` (4 crates) | **MERGE** | `crates/custos-evidence` | `judgments::{jev, rules, onnx}` |
| `crates/runtime/custos-context` | **MERGE** | `crates/custos-context` | `budget`, `windowing` |
| `crates/runtime/custos-context-management` | **MERGE** | `crates/custos-context` | `management` |
| `crates/runtime/custos-memory-service` | **MERGE** | `crates/custos-context` | `memory_hierarchy` |
| `crates/adapters/providers/*` (5 crates) | **MERGE** | `crates/custos-providers` | `providers::{claude, codex, local, antigravity}` |
| `crates/adapters/custos-providers` | **MERGE** | `crates/custos-providers` | Core provider dispatch |
| `crates/core/custos-provider-sdk` | **MERGE** | `crates/custos-providers` | `sdk` |
| `crates/core/custos-provider-types` | **MERGE** | `crates/custos-providers` | `types` |
| `crates/adapters/sandboxes/*` (2 crates) | **MERGE** | `crates/custos-adapters` | `sandboxes::{seatbelt, bubblewrap}` |
| `crates/adapters/custos-mcp` | **MERGE** | `crates/custos-adapters` | `mcp` |
| `crates/adapters/custos-adapters-mcp` | **MERGE** | `crates/custos-adapters` | `mcp::adapters` |
| `crates/adapters/custos-roaming` | **MERGE** | `crates/custos-adapters` | `roaming` |
| `crates/adapters/custos-download-manager` | **MERGE** | `crates/custos-adapters` | `downloads` |
| `crates/adapters/custos-local-inference` | **MERGE** | `crates/custos-providers` | `local_inference` (llama.cpp) |
| `crates/packs/custos-packs-engineering` | **MERGE** | `crates/custos-packs` | `engineering` (Coding / Repo Intelligence) |
| `crates/packs/custos-packs-research` | **MERGE** | `crates/custos-packs` | `research` (Document Intelligence) |
| `crates/packs/custos-packs-assistant` | **MERGE** | `crates/custos-packs` | `assistant` (Personal Intelligence) |
| `crates/app/custos-cli` | **GIỮ & NÂNG CẤP** | `crates/custos-cli` | CLI & Interactive TUI |
| `crates/app/custos-daemon` | **MERGE** | `crates/custos-daemon` | Daemon service |
| `crates/app/custos-local-api` | **MERGE** | `crates/custos-daemon` | `api` (REST/WS handlers) |
| `crates/core/custos-bridge` | **XÓA / ARCHIVE** | N/A | Dead bridge code |
| `crates/core/custos-sdk` | **XÓA / ARCHIVE** | N/A | Placeholder code |
| `crates/core/custos-sdk-types` | **XÓA / ARCHIVE** | N/A | Placeholder code |
| `crates/adapters/custos-acp-macros` | **XÓA / ARCHIVE** | N/A | Dead macros |
| `crates/app/custos-vscode` | **GIỮ NGUYÊN (TS)** | `crates/app/custos-vscode` | TypeScript extension |

---

## 5. Lộ Trình Triển Khai 5 Giai Đoạn (Execution Roadmap)

- **Phase 1: Chuẩn bị & Đóng dấu Khế ước V1 (`custos-domain`)**:
  - Khóa chặt các structs V1 chuẩn SSOT (`ActionIntentV1`, `TaskContractV1`, `EvidenceRecordV1`, `PermitV1`).
  - Đảm bảo 100% test passing, zero breaking change cho các crate đang dùng.
- **Phase 2: Hợp nhất Packs thành `crates/custos-packs`**:
  - Gộp 3 packs (engineering, research, assistant) vào 1 crate duy nhất với 3 module và thư mục declarative dùng chung.
- **Phase 3: Hợp nhất Adapters & Providers**:
  - Gộp 8 provider crates thành `crates/custos-providers`.
  - Gộp MCP, Sandboxes, Roaming, Download Manager thành `crates/custos-adapters`.
- **Phase 4: Hợp nhất Core Runtime & Authority & Evidence & Context**:
  - Tạo `custos-engine` gộp toàn bộ state machine, scheduler, session, agent loop.
  - Tạo `custos-authority` gộp security + gateway.
  - Tạo `custos-evidence` gộp verifiers + judgments.
  - Tạo `custos-context` gộp memory + context management.
- **Phase 5: Hoàn thiện CLI, Daemon & Dọn dẹp Workspace Cargo.toml**:
  - Gộp `custos-local-api` vào `custos-daemon`.
  - Cập nhật root `Cargo.toml` với đúng 12 crates.
  - Xóa các crate rác, dead code và chạy `cargo check --workspace` & `cargo test --workspace`.
