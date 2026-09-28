# Custos — Sprint Status & Execution Roadmap
## Kế Hoạch Sprint Hiện Tại: Phase 1 — Vertical Slice Khép Kín Tự Động

> **Sprint Cadence:** Sprint 2 (Phase 1) — Autonomous Execution Loop & Zero-Overhead Repo Intelligence  
> **Master Reference:** [dev_docs/README.md](./README.md) | [dev_docs/MODULE_OWNERSHIP.md](./MODULE_OWNERSHIP.md)  
> **Target Date:** 2026-09-27  

---

## 1. Thành Tựu Đã Hoàn Thành Tuyệt Đối (Phase 0 / PR A — COMPLETED)

1. **Hấp thụ & Loại bỏ Goose hoàn toàn:**
   - Đã chuyển giao 100% 14 năng lực cốt lõi từ Goose vào bên trong Custos (`custos-providers`, `custos-mcp`, `custos-local-inference`, `custos-agent`, `custos-context-management`).
   - Đã xóa sạch thư mục ngoại lai `goose` khỏi workspace.
   - Thử nghiệm cô lập: `cargo check --workspace` (0.60s) và `cargo test --workspace` pass 100% khi không còn Goose.
2. **Chuẩn hóa Subsystem Repo Intelligence:**
   - Chuyển `nexus` thành `Custos/tools/repo_intelligent/`.
   - Re-index toàn bộ 2,196 files của Custos: 9,741 symbols AST và 68,002 call graph edges.
   - Xác lập nguyên tắc: Custos dùng **Direct In-process SQLite Reader** để truy vấn AST/CallGraph với độ trễ micro-giây, không bị phụ thuộc vào MCP nội bộ.
3. **Kích hoạt Choke Points:**
   - Kích hoạt `GatewayProvider` trong `custos-cli` (kiểm tra Invariant I7 rà soát rò rỉ secret).
   - Mở rộng `custos-daemon` Runtime & `LocalApiDispatcher` hỗ trợ cả Task FSM và Session/Bridge FSM.

---

## 2. Kế Hoạch Công Việc Sprint 2 (Phase 1) Phân Chia Cho 3 Thành Viên

### 2.1. Vi — Founder / AI Systems & Product Intelligence Lead (Dân AI)
- [ ] **S1 Cognitive Router Polish (`crates/runtime/custos-cognitive`):**
  - Tinh chỉnh `RoutingPolicy` và `RoutingSignals`: Phân biệt chính xác giữa `TierZero` (deterministic static analysis), `SystemOne` (cheap qualified route), và `SystemTwo` (deep deliberation).
- [ ] **Repo Intelligence Direct Reader (`crates/runtime/custos-context`):**
  - Viết module Rust `RepoBridge` đọc trực tiếp file SQLite `tools/repo_intelligent/nexus_index.db` (bằng `rusqlite` có sẵn) để nạp AST và CallGraph vào `CodeGraph` in-memory.
  - Tích hợp vào `TokenAwareContextCompiler`: Biên dịch `ContextPack` chuẩn xác với token budget cho LLM.
- [ ] **Coding Agent Pipeline Initialization (`crates/packs/custos-packs-engineering`):**
  - Hiện thực hóa 2 role workers đầu tiên: `engineering.explorer` (quét call graph) và `engineering.planner` (lập change plan).

### 2.2. Truong — Core Platform & Security Lead (Dân SE 1)
- [ ] **Daemon Runtime Pipeline Dispatcher (`crates/app/custos-daemon`):**
  - Bổ sung phương thức `execute_task_pipeline(task_id: &str)` trong `CustosRuntime` để điều phối chuỗi tự động từ lúc nhận task đến khi kết thúc có bằng chứng.
- [ ] **Security Gateway Enforcement (`crates/adapters/custos-adapters-mcp`):**
  - Nối `GatewayTool` vào mọi công cụ thay đổi tệp, bắt buộc kiểm tra `ExecutionPermit` và sinh `Receipt`.
- [ ] **Crash Recovery Invariant Testing (`tests/crash/`):**
  - Bổ sung test case mô phỏng `kill -9` giữa lúc đang biên dịch context hoặc đang gọi provider, kiểm chứng daemon khởi động lại nạp đúng `TaskRevision`.

### 2.3. Vinh — Agent Systems & Coordination Research Engineer (Dân SE 2)
- [ ] **Re-entrant Worker Machine Integration (`crates/runtime/custos-workflow`):**
  - Kết nối `StateMachine` trong `custos-workflow/src/machine.rs` với `SessionManager` và `TaskKernel`.
  - Mỗi bước (turn) sinh ra một `ContinuationPacket` được lưu bền vững vào `custos-persistence`.
- [ ] **Session-to-Task Promotion Pipeline (`crates/core/custos-bridge`):**
  - Kiểm thử và tối ưu phương thức `promote(session_id, contract)`: Chuyển đổi mượt mà từ đoạn hội thoại chat nhanh sang Task kiểm soát chặt chẽ.
- [ ] **E2E Autonomous Loop Test (`tests/e2e/tests/daemon_autonomous_loop.rs`):**
  - Viết bài test E2E hoàn chỉnh kiểm chứng toàn bộ luồng tự động: Local API ➔ TaskKernel ➔ Cognitive S1 ➔ Repo Context ➔ GatewayProvider ➔ EvidenceEngine ➔ Succeeded State.

---

## 3. Tiêu Chí Hoàn Thành (Definition of Done - DoD cho Phase 1)

1. `cargo check --workspace` duy trì 0 warnings, 0 errors.
2. Bài test E2E `daemon_autonomous_loop` chạy thành công từ đầu đến cuối mà không cần can thiệp thủ công.
3. Không có bất kỳ rò rỉ I/O nào trong `custos-domain`.
4. Mọi bằng chứng xác nhận hoàn thành task đều có SHA-256 byte anchors hợp lệ trong `EvidenceBundle`.
