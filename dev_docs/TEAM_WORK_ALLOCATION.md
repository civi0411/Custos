# CUSTOS — BẢN PHÂN CÔNG NHIỆM VỤ & TRÁCH NHIỆM (RACI) THEO RESPONSIBILITY
## (Team Work Allocation & Canonical RACI Ownership Blueprint)

> **Mã tài liệu:** DEV-TEAM-01  
> **Phân loại:** Quy ước vận hành bắt buộc & Hợp đồng phân quyền kỹ thuật  
> **Cơ sở kiến trúc tối cao:** Tuân thủ tuyệt đối [`Custos.md`](../Custos.md) (Lưu ý: `Custos.md` mô tả **ĐÍCH ĐẾN** của kiến trúc, không phải hiện trạng codebase).
> **Hội đồng phê duyệt:** Vĩ (AI/Data Lead), Trường (SE/Storage Lead), Vinh (SE/Runtime Lead)

---

## 1. Triết Lý Thiết Kế Trách Nhiệm (6 Nguyên Tắc Cốt Lõi)

Để giải quyết triệt để vấn đề "thắt cổ chai" và cho phép Vĩ toàn tâm toàn ý (camp) vào mảng AI/Data, trong khi Trường và Vinh (camp) 100% vào Software Engineering (SE), team tuân thủ 6 nguyên tắc sau:

1. **Thiết kế theo Trách nhiệm (Responsibility-based), không theo Crate:** Code có thể di chuyển giữa các crate, nhưng ranh giới trách nhiệm (Ai làm AI, Ai làm Hệ thống) thì tuyệt đối không đổi.
2. **Hợp đồng đi trước (Contract-first):** Đóng băng 6 Ports (Traits) và 4 Schemas (JSON) trước mọi implementation. Mọi người code theo contract, không ai phải chờ ai.
3. **Cắt lát dọc (Vertical slice), không làm theo tầng ngang:** Mỗi giai đoạn (Gate) phải chạy end-to-end qua tất cả các tầng. 
4. **Phát triển theo Cổng nghiệm thu (Gate-driven):** Lộ trình đi từ Gate A $\rightarrow$ Gate OPT.
5. **Mock-first cho AI, InMemory cho SE:** Vĩ cung cấp `MockS1`/`MockS2` trả JSON tĩnh; Trường cung cấp `InMemoryStorage`; Vinh cung cấp `MockSandbox`. Phát triển song song tuyệt đối.
6. **Tự động hóa Documentation Triad:** Mọi thay đổi kiến trúc phải cập nhật đồng thời `Custos.md`, `docs/`, và `codebase-architecture.md` trước khi merge.

---

## 2. Phân Bổ Trọng Lượng & Ranh Giới Kỹ Thuật (40/30/30)

| Thành viên | Tỷ trọng | Lĩnh vực (Camp) | Triết lý |
|---|---|---|---|
| **VĨ (Chief Architect)** | **~40%** | **Cognitive, Contracts & Data** | Thiết kế hợp đồng 6 Ports. Quản lý toàn bộ AI (S1/S2), Prompt, Context Compiler. |
| **TRƯỜNG (SE 1)** | **~30%** | **Storage & OS Sandbox** | Hiện thực hóa lớp lưu trữ bền vững (SQLite WAL, CAS) và cô lập tiến trình OS. |
| **VINH (SE 2)** | **~30%** | **Runtime, UI & Integration** | Quản lý vòng đời tiến trình, UX (CLI/SDK), MCP client và kết nối Adapter ngoài. |

---

## 3. Bản Đặc Tả Trách Nhiệm Chi Tiết

### 3.1. VĨ — Cognitive & Contracts Lead (AI/Data Camp)
**Sở hữu tuyệt đối các thành phần sau:**
- **6 Ports + 4 Schemas:** Thiết kế các Trait và cấu trúc DTO (không viết code implementation của hệ thống con).
- **Context Compiler (8 bước) + Semantic Cache.**
- **Điều phối nhận thức (OI Routing):** Phân luồng S1 (Fast) và S2 (Deep) + Cost Governor.
- **Evidence Engine (Semantic):** Khớp nối bằng chứng. Calibration (Brier, ECE).
- **Trí nhớ (MemoryPort) + Temporal facts.**
- **3 Packs Logic:** Engineering (ACI), Research (FIRE), Assistant.
- **Daemon Bootstrap:** `crates/custos-daemon/src/main.rs` (Composition Root).
- **Đánh giá & Bảo mật:** Thư mục `evals/`, Red-team fixtures, Doc Triad.
*(Vĩ tuyệt đối không ôm: DB Schema, OS Sandbox, CLI, luồng IPC hay Migrations).*

### 3.2. TRƯỜNG — Storage & OS Lead (Systems SE Camp)
**Sở hữu tuyệt đối các thành phần sau:**
- **`custos-persistence` hoàn chỉnh:** Viết SQLite WAL, CAS, Outbox, Migration, FTS5.
- **Bảo chứng giao dịch T1–T5:** Cùng với Reconciliation protocol.
- **Cách ly hệ điều hành (OS Sandbox):** Hiện thực hóa `SandboxDriver` (Seatbelt cho macOS, Bubblewrap cho Linux) tại `custos-adapters`.
- **Kiểm thử chịu lỗi (Crash/Chaos Testing):** Bắn `SIGKILL` giữa T3-EXT.
- **CI/CD Pipeline:** `cargo check`, `clippy`, `deny.toml`.
*(Trường tuyệt đối không ôm: Logic AI, Routing, Prompt, hay Giao diện người dùng).*

### 3.3. VINH — Runtime & Integration Lead (Runtime/UI SE Camp)
**Sở hữu tuyệt đối các thành phần sau:**
- **`custos-runtime`:** Vòng lặp Workflow, Worker lease, CancellationToken, Checkpoint, Resume.
- **`custos-bridge`:** Gắn Session-Task, Idempotency (Tuyệt đối không gọi trực tiếp persistence).
- **`custos-daemon` (API/Listeners):** Local API, Vòng đời socket/pipe.
- **Client & UX (`custos-cli`, `custos-sdk`):** IPC client, TUI, Diff preview, Evidence table.
- **Tích hợp Ngoại vi (`custos-adapters`):** MCP Client (stdio + Streamable HTTP), và triển khai **1 Harness Adapter duy nhất** (ví dụ Claude Code) đi qua Conformance Gate trước khi làm các adapter khác.
*(Vinh tuyệt đối không ôm: AI Semantics, Evidence evaluation, Memory logic, hay Kernel rules).*

---

## 4. Thỏa Thuận Đóng Băng Giao Diện (Interface Freeze)

Trong Sprint 0, team phải hoàn thiện và "đóng băng" 6 Ports (Rust Traits) và 4 Schemas (JSON/Struct). Mọi thay đổi sau đó phải qua RFC.

### 6 Ports (Rust Traits)
| # | Interface | Owner | Nơi Implement chính |
|---|---|---|---|
| 1 | **KernelPort** | Vĩ | `custos-core` |
| 2 | **StoragePort** (`TaskRepo`, `CasStorage`, `OutboxPort`) | Trường | `custos-persistence` |
| 3 | **SandboxPort** (`SandboxDriver`) | Trường | `custos-adapters/sandbox` |
| 4 | **ModelPort** (`ModelProvider`) | Vĩ | `custos-provider` + `custos-adapters` |
| 5 | **WorkflowPort** (`WorkflowExecutionPort`) | Vinh | `custos-runtime` |
| 6 | **MemoryPort** | Vĩ | `custos-core` + `custos-persistence` |

### 4 Schemas cốt lõi
1. **ContextPack:** Input cho AI. (Owner: Vĩ)
2. **ActionIntent + Permit + Receipt:** Chuỗi xác thực quyền hạn. (Owner: Vĩ)
3. **EvidenceRecord:** Bằng chứng nghiệm thu (Fact/Extraction/Semantic). (Owner: Vĩ + Trường)
4. **ContinuationPacket:** Gói khôi phục tiến trình. (Owner: Vĩ)

---

## 5. Lộ Trình Phát Triển (Gate-Driven Roadmap)
*(Dự kiến 40-52 tuần cho 3 người)*

| Gate | Đích đến E2E | Vai trò Vĩ (AI/Data) | Vai trò Trường (Storage/OS) | Vai trò Vinh (Runtime/UI) |
|---|---|---|---|---|
| **A (Local E2E)** | CLI $\rightarrow$ Daemon $\rightarrow$ Local Model | Context Compiler minimal + S1/S2 Mock Stub + Evidence Fact | SQLite WAL + CAS + Task FSM + T1-T5 skeleton | CLI + Daemon + Local API + worktree read-only |
| **B (Workspace)** | PatchBundle + Sandbox test | Semantic verifier + OI route cơ bản | Sandbox Seatbelt/Bwrap + crash test T1-T5 | Workflow loop + cancel + checkpoint |
| **C (Crash)** | SIGKILL giữa T3-EXT | Evidence LTL + Reconciliation logic | Outbox durable + Reconciliation protocol | Recovery UI + audit T3 |
| **D (Taint)** | 100% injection bị chặn | Taint semantics + Context isolation + Secret redaction | (Hỗ trợ review) | MCP fail-closed + red-team fixtures |
| **E (SWE-bench)** | SWE-bench Lite subset pass | OI routing đầy đủ + Engineering Pack | (Hỗ trợ review) | 1 harness adapter (Claude/Codex) |
| **F (Research)** | Tái lập 1 paper + Reproducibility | Research Pack + FIRE + DatasetCard | (Hỗ trợ review) | Experiment runner + Batch API |
| **G (Memory)** | LongMemEval temporal | Temporal facts + MemoryPort + ContinuationPacket | (Hỗ trợ review) | Memory runtime + FTS5 recall |
| **OPT (Cost)** | Giảm 20% Cost/Task | OI economics + cascade + compaction | (Hỗ trợ review) | Usage telemetry T2 |
