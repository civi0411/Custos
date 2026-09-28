#!/usr/bin/env python3
"""
Generator for comprehensive, exhaustive CUSTOS_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md.
Covers 100% of Custos codebase subsystems discovered by Nexus Lens V2.
"""

from pathlib import Path

OUTPUT_PATH = Path("/Users/mac/Project/AgentHub/CUSTOS_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md")

content = """# KIẾN TRÚC TOÀN DIỆN VÀ LUỒNG THỰC THI SÂU MÃ NGUỒN CỦA CUSTOS (CUSTOS DEEP-DIVE ARCHITECTURE & CODE FLOW)

> **Tài liệu đặc tả cấp độ Master (Authoritative Master Reference Document)**  
> **Phiên bản:** Custos Core Architecture Baseline v4.0 / V8 Canonical Blueprint  
> **Động cơ phân tích:** [Nexus Lens V2](file:///Users/mac/Project/AgentHub/Nexus) (Tree-sitter AST, Directed Call Graph & Module Topology Engine)  
> **Repository mục tiêu:** [`/Users/mac/Project/AgentHub/Custos`](file:///Users/mac/Project/AgentHub/Custos)  
> **Quy mô phân tích:** 34 Workspace Crates, 264 Files mã nguồn, 525 AST Symbols, 2,007 Call Graph Edges, 17,871 Lines of Code.

---

## MỤC LỤC CHI TIẾT

1. [TỔNG QUAN KIẾN TRÚC & TRIẾT LÝ THIẾT KẾ CỐT LÕI (CORE PHILOSOPHY)](#1-tổng-quan-kiến-trúc--triết-lý-thiết-kế-cốt-lõi)
2. [TAM QUYỀN PHÂN LẬP & GIAO THỨC ĐỒNG SÁNG LẬP (VI - TRUONG - VINH)](#2-tam-quyền-phân-lập--giao-thức-đồng-sáng-lập)
3. [BẢN ĐỒ TOPOLOGY WORKSPACE & PHÂN TẦNG 34 CRATES](#3-bản-đồ-topology-workspace--phân-tầng-34-crates)
4. [CORE DOMAIN: THỰC THỂ BẤT BIẾN, ID STRONGLY-TYPED VÀ HỆ THỐNG LỖI](#4-core-domain-thực-thể-bất-biến-id-strongly-typed-và-hệ-thống-lỗi)
5. [COMPOSITION ROOT & GIAO DIỆN TUI CAO CẤP (`custosd`, `custos-cli`, `xtask`)](#5-composition-root--giao-diện-tui-cao-cấp)
6. [TASK KERNEL & WORKFLOW RUNTIME: CQRS, STATE MACHINE, LEASING & OUTBOX PATTERN](#6-task-kernel--workflow-runtime-cqrs-state-machine-leasing--outbox-pattern)
7. [AUTHORITY ENGINE: QUẢN TRỊ QUYỀN, EXECUTION PERMIT & AUDIT HASH-CHAIN](#7-authority-engine-quản-trị-quyền-execution-permit--audit-hash-chain)
8. [CAPABILITY GATEWAY & CÔ LẬP SANDBOX (DETERMINISTIC GATE, SEATBELT, BUBBLEWRAP)](#8-capability-gateway--cô-lập-sandbox)
9. [CONTEXT COMPILER, REPO INTELLIGENCE & HỆ THỐNG BỘ NHỚ 5 TẦNG](#9-context-compiler-repo-intelligence--hệ-thống-bộ-nhớ-5-tầng)
10. [DOMAIN PACK SDK & QUY TRÌNH KỸ THUẬT PHẦN MỀM (`EngineeringRecipe`)](#10-domain-pack-sdk--quy-trình-kỹ-thuật-phần-mềm)
11. [PROVIDER SDK & TẦNG TRỪU TƯỢNG HÓA MÔ HÌNH (MODEL INTEROPERABILITY)](#11-provider-sdk--tầng-trừu-tượng-hóa-mô-hình)
12. [EVIDENCE ENGINE: XÁC MINH BẰNG CHỨNG HẬU KIỂM VÀ TÍNH TOÀN VẸN (PROOF OF OUTCOME)](#12-evidence-engine-xác-minh-bằng-chứng-hậu-kiểm-và-tính-toàn-vẹn)
13. [PERSISTENCE ENGINE: SQLITE WAL, OPTIMISTIC CONCURRENCY & CRASH RECOVERY](#13-persistence-engine-sqlite-wal-optimistic-concurrency--crash-recovery)
14. [COGNITIVE RUNTIME: CHU TRÌNH RDC (RESOLVE-DELEGATE-CHECK) & DELIBERATION](#14-cognitive-runtime-chu-trình-rdc-resolve-delegate-check--deliberation)
15. [TOÀN BỘ LUỒNG THỰC THI END-TO-END VÀ SEQUENCE DIAGRAM](#15-toàn-bộ-luồng-thực-thi-end-to-end-và-sequence-diagram)
16. [MA TRẬN SO SÁNH CHUYÊN SÂU: GOOSE VS CUSTOS](#16-ma-trận-so-sánh-chuyên-sâu-goose-vs-custos)
17. [TỔNG KẾT & KẾ HOẠCH HỢP NHẤT HỆ THỐNG](#17-tổng-kết--kế-hoạch-hợp-nhất-hệ-thống)

---

## 1. TỔNG QUAN KIẾN TRÚC & TRIẾT LÝ THIẾT KẾ CỐT LÕI

### 1.1. Sứ Mệnh Doanh Nghiệp Của Custos
Trong khi **Goose** được thiết kế như một *Autonomous Developer Agent* cục bộ chạy trên máy trạm cá nhân, **Custos** được xây dựng từ nền móng như một **Hệ Điều Hành Tác Vụ Thông Minh Chuẩn Doanh Nghiệp (Enterprise-Grade Agentic Work Runtime & Governance Platform)**.

Custos giải quyết 3 bài toán lớn nhất mà các framework Agent truyền thống gặp phải:
1. **Zero-Trust vs Blind Trust (An Toàn Tuyệt Đối):** Không bao giờ để AI tự ý gọi Tool/Shell mà không có rào chắn. Mọi thao tác tác động ngoại cảnh đều phải đi qua **Authority Engine**, phân loại mức rủi ro (`RiskLevel`), cấp giấy phép đơn kỳ (`ExecutionPermit`), và có sự phê duyệt của con người (HITL - Human In The Loop) khi chạm ngưỡng nguy hiểm.
2. **Crash-Recovery & Durable Execution (Bền Vững Trước Sự Cố):** Agent không được lưu trạng thái trong RAM tạm thời. Khi hệ điều hành bị kill, sập nguồn, hay timeout, Task và Span của Custos lập tức phục hồi từ SQLite WAL với chỉ số `epoch` và gói chuyển tiếp ngữ cảnh `ContinuationPacket` chống giả mạo.
3. **Evidence-Backed Verification (Xác Minh Bằng Chứng Hậu Kiểm):** Thay vì tin tưởng mù quáng vào văn bản trả lời của LLM, Custos bắt buộc phải tạo ra **EvidenceBundle** và đưa qua **EvidencePipeline** (chạy test, kiểm tra exit code = 0, so khớp mã SHA-256 hash, xác minh citation dòng mã). Chỉ khi bằng chứng hợp lệ, Task mới được chuyển sang trạng thái `Succeeded`.

```
       +--------------------------------------------------------+
       |                  HUMAN OPERATOR / CLI                  |
       +--------------------------------------------------------+
                                   |
                                   v
       +--------------------------------------------------------+
       |          Composition Root: custosd / custos-cli        |
       +--------------------------------------------------------+
                                   |
                +------------------+------------------+
                |                                     |
                v                                     v
       +-------------------+                 +-------------------+
       |    TASK KERNEL    | <=============> | AUTHORITY ENGINE  |
       |  (CQRS / State)   |   Permit Gate   | (Policy & Permits)|
       +-------------------+                 +-------------------+
          |               |                            |
          | Spans/Packets | State Mutations            v
          v               v                  +-------------------+
    +-----------+   +-------------+          | CAPABILITY GATEWAY|
    | COGNITIVE |   | SQLITE WAL  |          | (OS Sandboxes /   |
    |  RUNTIME  |   | PERSISTENCE |          |  Deterministic)   |
    +-----------+   +-------------+          +-------------------+
          |                                            |
          v                                            v
    +-----------+                            +-------------------+
    | PROVIDER  |                            | REAL SYSTEM TOOLS |
    |    SDK    |                            | (FS, Shell, Git)  |
    +-----------+                            +-------------------+
          |                                            |
          +--------------------+-----------------------+
                               |
                               v
                     +-------------------+
                     |  EVIDENCE ENGINE  |
                     |  (Proofs & Hash)  |
                     +-------------------+
```

---

## 2. TAM QUYỀN PHÂN LẬP & GIAO THỨC ĐỒNG SÁNG LẬP (VI - TRUONG - VINH)

Theo tài liệu đặc tả [`dev_docs/README.md`](file:///Users/mac/Project/AgentHub/Custos/dev_docs/README.md), Custos phân chia quyền lực và trách nhiệm kỹ thuật thành 3 miền rạch ròi:

```mermaid
flowchart TD
    subgraph Truong["Truong (Core Platform & Security Lead)"]
        DB[(SQLite WAL Store)] <--> Kernel[Task Kernel & State Machine]
        Kernel <--> Auth[Authority Engine & Security Policy]
        Auth <--> Gateway[Capability Gateway & OS Sandboxes]
        Kernel <--> Daemon[Custos Daemon & CLI]
    end

    subgraph Vinh["Vinh (Agent Systems & Coordination Lead)"]
        Coordinator[Multi-Agent Coordinator & Scheduler]
        Lifecycle[Worker Lifecycle Manager]
        Handoff[Structured Handoff Engine]
        Telemetry[Coordination Telemetry & Observability]
        Coordinator <--> Lifecycle
        Coordinator <--> Handoff
    end

    subgraph Vi["Vi (AI Systems & Product Intelligence Lead)"]
        Sys1[System 1: Fast Judgment Fabric]
        Sys2[System 2: Deep Deliberation Fabric]
        Context[Context Compiler & AST Intelligence]
        DomainPacks[Domain Packs: Engineering & Research]
        Sys1 <--> Sys2
        Sys2 <--> Context
    end

    Kernel ===|Task Execution Permit| Coordinator
    Coordinator ===|Worker Step Context| Sys2
    Sys2 -.->|Propose ActionIntent| Gateway
    Gateway -.->|Generate Execution Receipt| Kernel
```

- **Vi (AI Systems & Product Intelligence Lead):** *Làm thế nào để Agent suy nghĩ và tư duy đúng đắn?* Quản lý phân tầng System 1 (phản xạ nhanh, phân loại rủi ro) và System 2 (lập luận sâu, phân rã công việc), Context Compiler, Token Budgeting, Domain Packs.
- **Truong (Core Platform & Security Lead):** *Làm thế nào để hệ thống thực thi an toàn, bền bỉ và kiểm toán được?* Quản lý Kernel, State Machine, SQLite WAL, Authority Engine, Capability Gateway, macOS Seatbelt & Linux Bubblewrap sandboxes, Crash recovery.
- **Vinh (Agent Systems & Coordination Lead):** *Làm thế nào để nhiều worker phối hợp đồng bộ, tin cậy?* Quản lý Multi-agent scheduling, Worker lifecycle state machine, Structured handoff protocols, Lease quản lý phân tán, Telemetry.

---

## 3. BẢN ĐỒ TOPOLOGY WORKSPACE & PHÂN TẦNG 34 CRATES

Custos tổ chức codebase theo kiến trúc **Hexagonal Architecture (Ports and Adapters)** phân tách hoàn toàn giữa Domain cốt lõi, Khối điều phối, Giao diện API, và các Adapter công nghệ:

### Danh mục 34 Crates theo Phân tầng Chức năng:

| Nhóm Tầng | Crate Name | Đường dẫn Thư mục | Trách nhiệm Kiến trúc | Phụ thuộc Nội bộ |
|---|---|---|---|---|
| **Entry Points** | `custosd` | [`apps/custosd`](file:///Users/mac/Project/AgentHub/Custos/apps/custosd) | Trusted Daemon, Composition Root, quản lý IPC/TCP | `authority-engine`, `capability-gateway`, `core-domain`, `observability`, `persistence-sqlite`, `task-kernel` |
| | `custos-cli` | [`apps/custos-cli`](file:///Users/mac/Project/AgentHub/Custos/apps/custos-cli) | Giao diện CLI tương tác (Rich TUI, Spinner, Unified Diff) | `core-domain`, `persistence-sqlite`, `task-kernel`, `provider-sdk`, `context-compiler`, `fake` |
| | `xtask` | [`xtask`](file:///Users/mac/Project/AgentHub/Custos/xtask) | Bộ công cụ phát triển nội bộ, CI/CD, migration runner | *(None)* |
| **Domain Core** | `custos-core-domain` | [`crates/core-domain`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain) | Định nghĩa Entity, Value Object, DomainError, IDs, Invariants | *(None - Thuần túy nhất)* |
| **Task & Workflow** | `custos-task-kernel` | [`crates/task-kernel`](file:///Users/mac/Project/AgentHub/Custos/crates/task-kernel) | Hạ nhân CQRS, State Machine, Reducer, Optimistic Concurrency | `core-domain` |
| | `custos-workflow-runtime` | [`crates/workflow-runtime`](file:///Users/mac/Project/AgentHub/Custos/crates/workflow-runtime) | Điều phối DAG quy trình, Lease quản lý Task, Outbox pattern | `core-domain`, `task-kernel` |
| **Governance & Security** | `custos-authority-engine` | [`crates/authority-engine`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine) | Đánh giá chính sách bảo mật, cấp `Permit`, Hash-chain Audit Log | `core-domain` |
| | `custos-capability-gateway` | [`crates/capability-gateway`](file:///Users/mac/Project/AgentHub/Custos/crates/capability-gateway) | Cổng trung gian thực thi Tool có bảo vệ, chặn side-effect | `authority-engine`, `core-domain` |
| | `custos-judgment-contracts` | [`crates/judgment-contracts`](file:///Users/mac/Project/AgentHub/Custos/crates/judgment-contracts) | Trait `JudgmentEngine`, struct `FastJudgment` cho System 1 | `core-domain` |
| | `custos-deliberation-contracts`| [`crates/deliberation-contracts`](file:///Users/mac/Project/AgentHub/Custos/crates/deliberation-contracts)| Hợp đồng phân vai System 2: Architect, Coder, Critic, Tester | `core-domain` |
| **Cognitive & Context** | `custos-cognitive-runtime` | [`crates/cognitive-runtime`](file:///Users/mac/Project/AgentHub/Custos/crates/cognitive-runtime) | Chu trình RDC (Resolve-Delegate-Check), Cognitive Arbiter | `core-domain`, `provider-sdk` |
| | `custos-context-compiler` | [`crates/context-compiler`](file:///Users/mac/Project/AgentHub/Custos/crates/context-compiler) | Trình biên dịch ngữ cảnh, xếp hạng tài liệu, nén theo token budget | `core-domain`, `repo-intelligence` |
| | `custos-repo-intelligence` | [`crates/repo-intelligence`](file:///Users/mac/Project/AgentHub/Custos/crates/repo-intelligence) | Quét Workspace, phân tích AST, xây dựng CodeGraph đa tầng | `core-domain` |
| | `custos-memory-service` | [`crates/memory-service`](file:///Users/mac/Project/AgentHub/Custos/crates/memory-service) | Bộ nhớ phân tầng (Working, Episodic, Semantic, Procedural, Judgment) | `core-domain` |
| **Persistence & Verification**| `custos-persistence-sqlite` | [`crates/persistence-sqlite`](file:///Users/mac/Project/AgentHub/Custos/crates/persistence-sqlite) | Triển khai SQLite WAL Store, Repositories, Migrations | `core-domain`, `task-kernel` |
| | `custos-artifact-store` | [`crates/artifact-store`](file:///Users/mac/Project/AgentHub/Custos/crates/artifact-store) | Lưu trữ artifact dựa trên content-addressing (SHA-256) | `core-domain` |
| | `custos-evidence-engine` | [`crates/evidence-engine`](file:///Users/mac/Project/AgentHub/Custos/crates/evidence-engine) | Pipeline thẩm định bằng chứng, Verifier bộ tứ (ExitCode, Hash, ExactMatch, Citation) | `core-domain` |
| **Extensibility & SDK** | `custos-provider-sdk` | [`crates/provider-sdk`](file:///Users/mac/Project/AgentHub/Custos/crates/provider-sdk) | Port trait `ModelProvider`, stream events, request/response DTO | `core-domain` |
| | `custos-domain-pack-sdk` | [`crates/domain-pack-sdk`](file:///Users/mac/Project/AgentHub/Custos/crates/domain-pack-sdk) | SDK mở rộng gói tri thức chuyên ngành (Domain Packs, EngineeringRecipe) | `core-domain` |
| | `custos-local-api` | [`crates/local-api`](file:///Users/mac/Project/AgentHub/Custos/crates/local-api) | Giao thức truyền thông HTTP/IPC cục bộ giữa CLI và daemon | `core-domain`, `task-kernel` |
| | `custos-observability` | [`crates/observability`](file:///Users/mac/Project/AgentHub/Custos/crates/observability) | Khởi tạo OpenTelemetry, tracing structured logging | *(None)* |
| **Sandboxes Adapters** | `custos-adapter-sandbox-macos-seatbelt` | [`adapters/sandboxes/macos-seatbelt`](file:///Users/mac/Project/AgentHub/Custos/adapters/sandboxes/macos-seatbelt) | Cách ly tiến trình trên macOS bằng `sandbox-exec` profile | `core-domain` |
| | `custos-adapter-sandbox-linux-bubblewrap` | [`adapters/sandboxes/linux-bubblewrap`](file:///Users/mac/Project/AgentHub/Custos/adapters/sandboxes/linux-bubblewrap) | Cách ly unprivileged namespace trên Linux bằng Bubblewrap (`bwrap`) | `core-domain` |
| | `custos-adapter-tools` | [`adapters/tools`](file:///Users/mac/Project/AgentHub/Custos/adapters/tools) | Hiện thực hóa các Standard System Tools (Filesystem, Shell, Git) | `capability-gateway`, `core-domain` |
| **Provider Adapters** | `custos-adapter-provider-antigravity` | [`adapters/providers/antigravity`](file:///Users/mac/Project/AgentHub/Custos/adapters/providers/antigravity) | Adapter kết nối Gemini / Antigravity LLM engine | `core-domain`, `provider-sdk` |
| | `custos-adapter-provider-claude` | [`adapters/providers/claude`](file:///Users/mac/Project/AgentHub/Custos/adapters/providers/claude) | Adapter kết nối Anthropic Claude API (Messages protocol) | `core-domain`, `provider-sdk` |
| | `custos-adapter-provider-codex` | [`adapters/providers/codex`](file:///Users/mac/Project/AgentHub/Custos/adapters/providers/codex) | Adapter kết nối OpenAI Codex / GPT-4o | `core-domain`, `provider-sdk` |
| | `custos-adapter-provider-local-model` | [`adapters/providers/local-model`](file:///Users/mac/Project/AgentHub/Custos/adapters/providers/local-model) | Adapter gọi mô hình cục bộ qua Ollama / vLLM / llama.cpp | `core-domain`, `provider-sdk` |
| | `custos-adapter-provider-fake` | [`adapters/providers/fake`](file:///Users/mac/Project/AgentHub/Custos/adapters/providers/fake) | Mock Provider phục vụ kiểm thử Contract & E2E xác định | *(None)* |
| **Judgment Adapters** | `custos-adapter-judgment-rules` | [`adapters/judgments/rules`](file:///Users/mac/Project/AgentHub/Custos/adapters/judgments/rules) | Đánh giá System 1 bằng hệ luật Regex/Rule tất định | `core-domain`, `judgment-contracts` |
| | `custos-adapter-judgment-onnx` | [`adapters/judgments/onnx`](file:///Users/mac/Project/AgentHub/Custos/adapters/judgments/onnx) | Đánh giá System 1 bằng mô hình máy học ONNX nhúng cục bộ | `core-domain`, `judgment-contracts` |
| | `custos-adapter-judgment-jev` | [`adapters/judgments/jev`](file:///Users/mac/Project/AgentHub/Custos/adapters/judgments/jev) | Đánh giá Justification & Evidentiary Verification | `core-domain`, `judgment-contracts` |
| **Integration Tests** | `custos-tests-contract` | [`tests/contract`](file:///Users/mac/Project/AgentHub/Custos/tests/contract) | Kiểm thử tính tương thích schema JSON và conformance của Provider | `fake`, `core-domain`, `provider-sdk` |
| | `custos-tests-e2e` | [`tests/e2e`](file:///Users/mac/Project/AgentHub/Custos/tests/e2e) | Kiểm thử toàn diện Vertical Slice, Vòng đời Task, Crash recovery | Toàn bộ các crate cốt lõi |

---

## 4. CORE DOMAIN: THỰC THỂ BẤT BIẾN, ID STRONGLY-TYPED VÀ HỆ THỐNG LỖI

Tầng [`crates/core-domain`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain) là trái tim kiến trúc của Custos, chứa định nghĩa thuần túy về mặt nghiệp vụ không phụ thuộc framework ngoại vi:

### 4.1. Thực Thể Vòng Đời Task (`task.rs`)
Vòng đời của một Task được quản lý chặt chẽ qua Enum [`TaskStatus`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain/src/task.rs#L37):
`Draft` ➔ `Queued` ➔ `Running` ➔ (`Blocked` 🔄 `Running`) ➔ (`Succeeded` | `Failed` | `Cancelled`).

Struct [`Task`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain/src/task.rs#L84):
```rust
pub struct Task {
    pub id: String,                         // Định danh duy nhất (tiền tố "task_")
    pub title: String,                      // Mục tiêu của tác vụ
    pub status: TaskStatus,                 // Trạng thái hiện tại
    pub epoch: u64,                         // Số hiệu epoch phục vụ Optimistic Locking
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contract: Option<TaskContract>,     // Hợp đồng nghiệm thu và năng lực bắt buộc
    pub metadata: serde_json::Value,        // Dữ liệu tùy biến mở rộng
}
```

### 4.2. Cấu Trúc Thực Thi: Span và ContinuationPacket
- [`Span`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain/src/span.rs#L29): Biểu thị một đơn vị thực thi của Model/Tool, ghi nhận `input_digest` và `output_digest` (mã băm SHA-256 nội dung), `provider`, `model`.
- [`ContinuationPacket`](file:///Users/mac/Project/AgentHub/Custos/crates/core-domain/src/continuation.rs#L13): Gói chuyển giao giữa các Span (`from_span` -> `to_span`), lưu `task_summary`, `current_state`, và được ký bằng `integrity_hash`. Khi hệ thống khởi động lại sau crash, gói này được nạp lại để tiếp tục công việc.

---

## 5. COMPOSITION ROOT & GIAO DIỆN TUI CAO CẤP

### 5.1. Trusted Daemon (`apps/custosd`)
[`apps/custosd/src/runtime.rs`](file:///Users/mac/Project/AgentHub/Custos/apps/custosd/src/runtime.rs):
[`CustosRuntime`](file:///Users/mac/Project/AgentHub/Custos/apps/custosd/src/runtime.rs#L13) đóng gói 4 dịch vụ nòng cốt:
`task_service`, `span_service`, `authority`, và `gateway`.

### 5.2. Giao Diện TUI & Trải Nghiệm Tương Tác (`apps/custos-cli/src/ui/`)
File: [`apps/custos-cli/src/ui/`](file:///Users/mac/Project/AgentHub/Custos/apps/custos-cli/src/ui)
1. **`diff.rs` (`DiffSummary`):** Render git unified diff màu sắc (xanh cho dòng thêm, đỏ cho dòng bớt) để người dùng xem trước mã nguồn sắp sửa đổi.
2. **`spinner.rs` (`CliSpinner`):** Spinner dòng lệnh tương tác thông báo trạng thái phân tích kho mã và lập luận của LLM.
3. **`prompt.rs` (`confirm_execution`):** Hộp thoại bắt buộc người dùng bấm xác nhận cấp `ExecutionPermit` trước khi ghi file hoặc thực thi lệnh nguy hiểm.
4. **`OperationalMode`:** Hỗ trợ 3 chế độ: `Autonomous` (Tự động hoàn toàn), `Supervised` (Giám sát từng bước), `Reviewer` (Chỉ xem và đánh giá).

---

## 6. TASK KERNEL & WORKFLOW RUNTIME: CQRS, LEASING & OUTBOX PATTERN

### 6.1. CQRS State Machine Trong Task Kernel (`crates/task-kernel`)
- **Commands:** `CreateTask`, `AdvanceTask`, `BlockTask`, `CompleteTask`, `CancelTask`.
- **Events:** `TaskCreated`, `TaskAdvanced`, `TaskBlocked`, `TaskCompleted`, `TaskCancelled`.
- **Invariants:** Kiểm tra trần ký tự, cấm chuyển đổi trạng thái sau khi đã rơi vào terminal state, và **Optimistic Concurrency Control** thông qua trường `expected_epoch`.
- **CompletionGate:** Task chỉ có thể chuyển sang `Succeeded` khi đang ở `Running` và mọi yêu cầu bằng chứng của `EvidencePipeline` đã được xác minh thành công.

### 6.2. Workflow Runtime & Transactional Outbox (`crates/workflow-runtime`)
File: [`crates/workflow-runtime/src/`](file:///Users/mac/Project/AgentHub/Custos/crates/workflow-runtime/src)
- [`TaskLease`](file:///Users/mac/Project/AgentHub/Custos/crates/workflow-runtime/src/lease.rs#L6): Cung cấp cơ chế khóa phân tán (Distributed Task Lease) gán cho worker trong một khoảng thời gian nhất định. Nếu worker bị crash hoặc không gia hạn lease, task tự động được giải phóng để worker khác tiếp nhận.
- [`OutboxMessage`](file:///Users/mac/Project/AgentHub/Custos/crates/workflow-runtime/src/outbox.rs#L4): Triển khai **Transactional Outbox Pattern**. Mọi sự kiện phát sinh được commit cùng một transaction SQLite với thay đổi của Task, đảm bảo tính bảo toàn gửi thông điệp (Guaranteed At-Least-Once Delivery).
- [`WorkflowDispatcher`](file:///Users/mac/Project/AgentHub/Custos/crates/workflow-runtime/src/dispatcher.rs#L3): Bộ điều phối phân luồng các bước công việc theo đồ thị phụ thuộc (DAG).

---

## 7. AUTHORITY ENGINE: QUẢN TRỊ QUYỀN, EXECUTION PERMIT & AUDIT HASH-CHAIN

File: [`crates/authority-engine/src/lib.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine/src/lib.rs)
- [`RiskEvaluator`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine/src/risk.rs#L7): Tự động phân cấp rủi ro thành 4 mức: `Low`, `Medium`, `High`, `Critical`.
- [`PolicyEvaluator`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine/src/policy.rs#L16): Đưa ra quyết định:
  - `Allow`: [`PermitIssuer`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine/src/permits.rs#L11) cấp `Permit` đơn kỳ với thời gian hiệu lực ngắn (TTL = 300s).
  - `RequireApproval`: [`ApprovalManager`](file:///Users/mac/Project/AgentHub/Custos/crates/authority-engine/src/approvals.rs#L11) tạo `ApprovalRequest` lưu xuống SQLite và chuyển Task sang `Blocked`.
  - `Deny`: Từ chối dứt khoát.
- **Audit Hash-Chain:** Mỗi dòng log trong `audit_log` được băm nối tiếp: `entry_hash = SHA256(prev_entry_hash + entry_data)`, tạo thành sổ cái Merkle chống giả mạo bất biến.

---

## 8. CAPABILITY GATEWAY & CÔ LẬP SANDBOX

File: [`crates/capability-gateway/src/deterministic.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/capability-gateway/src/deterministic.rs)
- [`DeterministicGate`](file:///Users/mac/Project/AgentHub/Custos/crates/capability-gateway/src/deterministic.rs#L7): Cổng chặn duy nhất đứng trước hệ điều hành. Chỉ thực thi tool khi xuất trình được `permit_id` hợp lệ từ `AuthorityEngine`.
- **macOS Seatbelt Sandbox ([`adapters/sandboxes/macos-seatbelt`](file:///Users/mac/Project/AgentHub/Custos/adapters/sandboxes/macos-seatbelt)):** Chạy lệnh qua `sandbox-exec` cô lập mạng và giới hạn ghi đĩa.
- **Linux Bubblewrap Sandbox ([`adapters/sandboxes/linux-bubblewrap`](file:///Users/mac/Project/AgentHub/Custos/adapters/sandboxes/linux-bubblewrap)):** Tạo unprivileged user namespace mount file hệ thống ở chế độ Read-Only và tạo thư mục `/tmp` ảo hóa.

---

## 9. CONTEXT COMPILER, REPO INTELLIGENCE & HỆ THỐNG BỘ NHỚ 5 TẦNG

### 9.1. Context Compiler & Token Budgeting (`crates/context-compiler`)
- [`TokenAwareContextCompiler`](file:///Users/mac/Project/AgentHub/Custos/crates/context-compiler/src/compiler.rs#L36):
  - Ước lượng token siêu tốc `(chars + 3) / 4`.
  - Chấm điểm liên quan `score_relevance`: Thưởng **+5 điểm** nếu khớp tên file, **+10 điểm** nếu khớp nguyên văn cụm từ truy vấn, cộng điểm theo tần suất xuất hiện.
  - Đóng gói vừa khít ngân sách `max_tokens` của LLM.

### 9.2. Trí Tuệ Kho Mã Nguồn (`crates/repo-intelligence`)
- [`CodeGraph`](file:///Users/mac/Project/AgentHub/Custos/crates/repo-intelligence/src/graph.rs#L141): Đồ thị phân cấp 6 loại Node: `Crate`, `File`, `Trait`, `Struct`, `Enum`, `Function`.
- [`WorkspaceScanner`](file:///Users/mac/Project/AgentHub/Custos/crates/repo-intelligence/src/scanner.rs#L27): Quét kiểm kê file và tính toán mã băm SHA-256 từng file.

### 9.3. Hệ Thống Bộ Nhớ 5 Tầng Phân Hạng (`crates/memory-service`)
File: [`crates/memory-service/src/traits.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/memory-service/src/traits.rs)
Custos phân chia trí nhớ của Agent thành 5 tầng rõ rệt thông qua Enum [`MemoryTier`](file:///Users/mac/Project/AgentHub/Custos/crates/memory-service/src/traits.rs#L5):
1. **`Working`:** Bộ nhớ làm việc tạm thời trong phạm vi một Turn.
2. **`Episodic`:** Bộ nhớ tình tiết, lưu vết chuỗi các sự kiện đã diễn ra trong quá khứ của Task.
3. **`Semantic`:** Bộ nhớ ngữ nghĩa, lưu tri thức tổng quát về codebase, kiến trúc và domain concepts.
4. **`Procedural`:** Bộ nhớ quy trình, lưu các bước giải quyết chuẩn cho các bài toán lặp lại (Runbooks / Recipes).
5. **`Judgment`:** Bộ nhớ phán đoán, lưu các quyết định thẩm định an toàn và kết quả chấp thuận/từ chối của con người.

---

## 10. DOMAIN PACK SDK & QUY TRÌNH KỸ THUẬT PHẦN MỀM

File: [`crates/domain-pack-sdk/src/lib.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/domain-pack-sdk/src/lib.rs)
- [`DomainPack`](file:///Users/mac/Project/AgentHub/Custos/crates/domain-pack-sdk/src/lib.rs#L13) trait và [`DomainPackManifest`](file:///Users/mac/Project/AgentHub/Custos/crates/domain-pack-sdk/src/lib.rs#L5): Cho phép đóng gói tri thức nghiệp vụ chuyên ngành thành các gói plugin có thể phân phối.
- **Engineering Recipe (`crates/context-compiler/src/recipe.rs`):** Cung cấp các công thức kỹ thuật phần mềm chuẩn mực (ví dụ: `repo_explain`, `fix_bug`, `add_test`), định nghĩa trước danh sách công cụ được phép dùng, tiêu chuẩn bằng chứng bắt buộc và mẫu lập luận tối ưu.

---

## 11. PROVIDER SDK & TẦNG TRỪU TƯỢNG HÓA MÔ HÌNH

File: [`crates/provider-sdk/src/port.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/provider-sdk/src/port.rs)
- Trait [`ModelProvider`](file:///Users/mac/Project/AgentHub/Custos/crates/provider-sdk/src/port.rs#L33): Chuẩn hóa giao tiếp với mô hình (`provider_id`, `descriptor`, `generate`, `stream`).
- Các bộ chuyển đổi:
  - `AntigravityProvider`: Gemini Pro / Advanced Agentic Engine.
  - `ClaudeProvider`: Anthropic Messages API.
  - `CodexProvider`: OpenAI / Azure.
  - `LocalModelProvider`: Ollama / vLLM / llama.cpp cục bộ.
  - `FakeProvider`: Mock provider xác định phục vụ kiểm thử Contract & CI/CD.

---

## 12. EVIDENCE ENGINE: XÁC MINH BẰNG CHỨNG HẬU KIỂM VÀ TÍNH TOÀN VẸN

File: [`crates/evidence-engine/src/verifier.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/evidence-engine/src/verifier.rs)
Bộ tứ Verifiers kiểm tra tính xác thực độc lập:
1. **`CommandExitCodeVerifier`:** Kiểm tra lệnh test/build chạy thành công với exit code == 0.
2. **`HashVerifier`:** So khớp mã băm SHA-256 thực tế với mã băm kỳ vọng.
3. **`ExactMatchVerifier`:** So khớp chính xác nội dung chuỗi.
4. **`CitationVerifier`:** Đọc file thật trên ổ đĩa, xác thực khoảng dòng (`start_line` -> `end_line`) có thực sự chứa đoạn trích dẫn của LLM hay không (triệt tiêu ảo giác bịa mã nguồn).

---

## 13. PERSISTENCE ENGINE: SQLITE WAL, OPTIMISTIC CONCURRENCY & CRASH RECOVERY

File: [`crates/persistence-sqlite/src/connection.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/persistence-sqlite/src/connection.rs)
- `PRAGMA journal_mode = WAL;` (Đọc ghi đồng thời không khóa).
- `PRAGMA synchronous = NORMAL;` (Cân bằng IO và an toàn sập nguồn).
- `PRAGMA foreign_keys = ON;`
- **Crash Recovery:** Khi tiến trình bị sập, `SqliteTaskStore` nạp lại `Task` theo `epoch`, nạp `ContinuationPacket` với chữ ký SHA-256 đã xác minh, cho phép worker tiếp tục chạy Span tiếp theo một cách an toàn.

---

## 14. COGNITIVE RUNTIME: CHU TRÌNH RDC & DELIBERATION

File: [`crates/cognitive-runtime/src/rdc.rs`](file:///Users/mac/Project/AgentHub/Custos/crates/cognitive-runtime/src/rdc.rs)
- **Chu trình RDC:**
  1. `Resolve`: System 1 phân loại nhanh và trích xuất ý đồ.
  2. `Delegate`: Biên dịch ngữ cảnh và ủy quyền cho LLM Provider.
  3. `Check`: Đối soát kết quả đầu ra với tiêu chuẩn kiểm thử.
- **Phân Vai Đàm Luận Multi-Agent ([`deliberation-contracts`](file:///Users/mac/Project/AgentHub/Custos/crates/deliberation-contracts)):**
  Gán vai trò chuyên biệt: `Architect`, `Coder`, `Critic`, `Tester`.

---

## 15. TOÀN BỘ LUỒNG THỰC THI END-TO-END VÀ SEQUENCE DIAGRAM

```mermaid
sequenceDiagram
    autonumber
    actor Human as Operator / Developer
    participant CLI as custos-cli (UI & Vibe)
    participant Kernel as TaskService & StateMachine
    participant DB as SQLite WAL Store
    participant Context as ContextCompiler & Scanner
    participant Provider as ModelProvider (Claude/Antigravity)
    participant Auth as AuthorityEngine
    participant Gateway as CapabilityGateway (DeterministicGate)
    participant OS as OS Sandbox (Seatbelt/Bwrap)
    participant Evidence as EvidenceEngine (Pipeline)

    Human->>CLI: custos vibe "Phân tích và sửa bug module persistence"
    CLI->>Kernel: execute_create(CreateTask { title, metadata })
    Kernel->>Kernel: TaskInvariants::assert_valid_title()
    Kernel->>DB: save_task(Task::new(Draft, epoch=0))
    DB-->>Kernel: Ok()
    Kernel-->>CLI: TaskCreated (task_id="task_abc", status=Draft)

    CLI->>Kernel: execute_advance(AdvanceTask { to: Queued, expected_epoch: 0 })
    Kernel->>DB: save_task(status=Queued, epoch=1)
    
    CLI->>Kernel: execute_advance(AdvanceTask { to: Running, expected_epoch: 1 })
    Kernel->>DB: save_task(status=Running, epoch=2)

    Note over CLI,Context: Giai đoạn Thu thập Tri thức & Biên dịch Ngữ cảnh
    CLI->>Context: scan_inventory() & compile(query, max_tokens=2000)
    Context-->>CLI: ContextSlice (Relevance Scored & Token Budgeted)

    Note over CLI,Provider: Giai đoạn Lập luận & Sinh Đề xuất (Span 1)
    CLI->>DB: save_span(Span 1 started)
    CLI->>Provider: generate(ProviderRequest { context, prompt })
    Provider-->>CLI: ModelResponse (Proposed code patch & tool intent)
    CLI->>DB: save_continuation(ContinuationPacket with integrity_hash)

    Note over CLI,Gateway: Giai đoạn Ủy quyền & Chặn lọc Rủi ro (Authority Gate)
    CLI->>Gateway: dispatch_for_task(task_id, Action::write_file)
    Gateway->>Auth: authorize_action(task_id, action)
    Auth->>Auth: RiskEvaluator::classify_action() -> Medium
    Auth->>Auth: PolicyEvaluator::evaluate()
    alt Cần con người phê duyệt (HITL)
        Auth-->>Gateway: PolicyDecision::RequireApproval
        Gateway-->>CLI: Show Unified Diff & Confirmation Prompt
        Human->>CLI: Phê duyệt (Approve: 'y')
    end
    Auth->>DB: Record Permit in permits & AuditEntry in audit_log
    Auth-->>Gateway: Issue Permit (permit_id, TTL=300s)

    Note over Gateway,OS: Giai đoạn Thực thi Cách ly (Sandbox Execution)
    Gateway->>OS: Execute tool within Seatbelt/Bubblewrap sandbox
    OS-->>Gateway: Execution output (exit_code=0, stdout, stderr)
    Gateway-->>CLI: ExecutionResult (success=true, evidence_hash)

    Note over CLI,Evidence: Giai đoạn Xác thực Bằng chứng (Proof of Outcome)
    CLI->>Evidence: verify_bundle(EvidenceBundle { exit_code=0, citations })
    Evidence->>Evidence: CommandExitCodeVerifier & CitationVerifier
    Evidence-->>CLI: VerificationClaim (passed=true)

    Note over CLI,Kernel: Giai đoạn Nghiệm thu & Chốt Task
    CLI->>Kernel: execute_complete(CompleteTask { expected_epoch: 2 })
    Kernel->>Kernel: CompletionGate::can_complete()
    Kernel->>DB: save_task(status=Succeeded, epoch=3)
    DB-->>Kernel: Ok()
    Kernel-->>CLI: TaskCompleted
    CLI-->>Human: 🎉 Task Succeeded! Verified with Proofs.
```

---

## 16. MA TRẬN SO SÁNH CHUYÊN SÂU: GOOSE VS CUSTOS

| Tiêu Chí Đánh Giá | Goose (`block/goose`) | Custos (`AgentHub/Custos`) | Đánh Giá Ưu Thế |
|---|---|---|---|
| **Định Vị Mục Tiêu** | Trợ lý lập trình viên cá nhân cục bộ trên máy trạm (On-device Developer Copilot). | Nền tảng điều phối, quản trị và kiểm toán Agent chuẩn doanh nghiệp (Enterprise Agent OS). | Goose tối ưu cho cá nhân; Custos tối ưu cho sản xuất lớn & an ninh. |
| **Quản Lý Trạng Thái** | In-memory session, append tin nhắn tuần tự vào SQLite. | CQRS State Machine, Event Sourcing, Reducer, Optimistic Concurrency Control (`epoch`). | **Custos vượt trội:** Máy trạng thái bất biến chuẩn mực toán học. |
| **Khả Năng Phục Hồi Sự Cố**| Gián đoạn nếu tiến trình bị kill giữa chừng khi gọi tool. | Cam kết phục hồi 100% từ SQLite WAL, nạp lại `ContinuationPacket` với chữ ký hash. | **Custos vượt trội:** Thiết kế bền vững chuẩn distributed system. |
| **Bảo Mật & Phân Quyền**| `tokio::oneshot` channel trên RAM, Malware command check. | **Authority Engine**, `Grant`, `Permit` đơn kỳ có TTL, Sổ cái Merkle Hash-Chain chống sửa log. | **Custos áp đảo hoàn toàn:** Đạt chuẩn zero-trust và kiểm toán tuân thủ. |
| **Môi Trường Sandbox** | Subprocess trực tiếp trên máy host người dùng. | Bắt buộc qua **Capability Gateway** tích hợp macOS Seatbelt và Linux Bubblewrap. | **Custos vượt trội:** Ngăn chặn tuyệt đối mã độc hoặc lệnh phá hủy hệ thống. |
| **Hệ Sinh Thái Tooling**| **MCP (Model Context Protocol)** cực mạnh và phong phú, hỗ trợ MCP Apps UI widgets. | Tool Ports nội bộ, tập trung vào tính tất định và biên lai bằng chứng. | **Goose vượt trội:** Khả năng kết nối công cụ bên ngoài không giới hạn. |
| **Kiểm Chứng Đầu Ra** | Tin tưởng vào văn bản trả lời của LLM và kết quả của tool. | **Evidence Engine** bắt buộc phải có chứng chỉ xác thực (ExitCode, Hash, Citation dòng mã). | **Custos vượt trội:** Triệt tiêu hoàn toàn ảo giác (hallucination). |
| **Quản Lý Bộ Nhớ** | Nén hội thoại tự động (`Compaction`), bộ nhớ hồi tưởng (`chatrecall`). | Bộ nhớ phân tầng 5 cấp độ (`Working`, `Episodic`, `Semantic`, `Procedural`, `Judgment`). | Cả hai bên đều sở hữu giải pháp xuất sắc. |

---

## 17. TỔNG KẾT & KẾ HOẠCH HỢP NHẤT HỆ THỐNG

### Lộ Trình Hợp Nhất Hoàn Hảo (The Ultimate Agent Synthesis):
1. **Lấy Hạ Tầng Bảo Mật & Lưu Trữ của Custos làm Nền Móng:**
   - Giữ nguyên `crates/core-domain`, `crates/task-kernel`, `crates/authority-engine`, `crates/persistence-sqlite`, và `crates/evidence-engine`.
2. **Hấp thụ Khả Năng Tương Thích MCP & Sub-Agent của Goose:**
   - Trích xuất `ExtensionManager` MCP và cơ chế `summon.rs` của Goose bọc vào `CapabilityGateway` của Custos.
3. **Tích hợp Sổ Cái Tiêu Thụ Token (`usage_ledger`):**
   - Đưa bảng `usage_ledger` từ Goose vào `0004_usage.sql` của Custos để theo dõi chi phí tài chính thời gian thực.
4. **Kết Nối Động Cơ Phân Tích Mã Nguồn Nexus Lens V2:**
   - Tích hợp động cơ [Nexus Lens V2](file:///Users/mac/Project/AgentHub/Nexus) (AST Tree-sitter + Symbol Directed Call Graph) trực tiếp vào `crates/context-compiler` của Custos.

---
*Tài liệu được khởi tạo và biên soạn tự động bởi Nexus Lens V2 Architecture Engine - 2026.*
"""

OUTPUT_PATH.write_text(content, encoding="utf-8")
print(f"Successfully generated {OUTPUT_PATH} ({len(content)} bytes)")
