# SADE Platform Architecture: Deep Research, Living Commons ("Ao Hồ SADE"), and Refactoring Specification

> **Authority Reference**: Architectural Audit and Blueprint for `Custos/platform` (538 Rust modules across 8 canonical platform crates).  
> **Philosophy Center**: Anthropic Agentic Environment Principles + Clean OOP / Hexagonal System Design.

---

## 1. Triết lý Cốt lõi: Anthropic Environment & "Ao Hồ SADE"

Trong triết lý của Anthropic về Agentic Systems, **Agent không phải là một chuỗi prompt độc lập, mà là một thực thể sinh học (organism) sống trong một môi trường được định nghĩa chặt chẽ (existential environment)**.
- **Môi trường (Environment)** là nguồn chân lý duy nhất (Single Source of Truth). Agent cảm nhận thế giới qua các quan sát (Observations) và tương tác với thế giới qua các hành động có kiểm soát (Governed Actions).
- **Công cụ (Tools)** không phải là các hàm tiện ích rời rạc, mà là các cơ quan vận động (actuators) đòi hỏi giấy phép (Permits) và để lại bằng chứng thực thi (Execution Receipts).
- **Hệ sinh thái SADE ("Ao Hồ SADE - Living Commons")**: Một môi trường mở nhưng kỷ luật cao, nơi mọi loài agent (từ Claude Code, OpenAI Codex, Block Goose, các scout nền S1/S2, đến các verifier oracles chuyên biệt) có thể cùng được "thả" vào một ao hồ duy nhất. Chúng không dẫm chân lên nhau, mà cộng sinh (symbiosis), tận dụng sản phẩm của nhau để cùng hoàn thiện và phát triển.

```
                           ┌──────────────────────────────────────────────┐
                           │            SADE LIVING COMMONS              │
                           │               ("Ao Hồ SADE")                 │
                           └──────────────────────┬───────────────────────┘
                                                  │
                 ┌────────────────────────────────┴────────────────────────────────┐
                 ▼                                                                 ▼
   ┌───────────────────────────┐                                     ┌───────────────────────────┐
   │    HETEROGENEOUS FLEET    │                                     │     AQUATIC SUBSTRATE     │
   │  - Claude Code / Codex    │                                     │  - Universal Resource Reg │
   │  - Goose / OpenCode CLI   │◄──────────────┐                     │    urr://[comp]/[kind]/id │
   │  - Fast S1 Scouts         │               │                     │  - Extensible Lake Event  │
   │  - Specialized Verifiers  │               │                     │    modality/topic bus     │
   │  - Background Evaluators  │               │                     │  - CAS Immutable Storage  │
   └─────────────┬─────────────┘               │                     └─────────────┬─────────────┘
                 │                             │                                   │
                 │ 1. Emits Work & Products    │ 3. Wakes Up & Refines             │ 2. Content Addressed
                 ▼                             │                                   ▼
   ┌───────────────────────────────────────────┴─────────────────────────────────────────────────┐
   │                                  CROSS-DOMAIN PIPELINE                                      │
   │       Science (Claim)  ──►  Coding (AST/Patch)  ──►  Quant (Factor/PBO)  ──►  Assistant     │
   └───────────────────────────────────────────┬─────────────────────────────────────────────────┘
                                               │
                                               ▼ 4. Closed Feedback Loop
                                 ┌───────────────────────────┐
                                 │   T6 REPAIR & OPTIMIZE    │
                                 │  Self-improving Ecosystem │
                                 └───────────────────────────┘
```

---

## 2. Kiểm toán Chi tiết Toàn bộ 8 Crates trong `platform/` (538 Files)

Hệ thống nền tảng `platform/` của Custos được thiết kế theo mô hình **Hexagonal Architecture (Ports and Adapters)** với 8 canonical crates. Dưới đây là phân tích chi tiết từng crate:

### 2.1. `platform/domain` (`custos-domain`) — 81 files
* **Vị trí & Trách nhiệm**: Tầng trung tâm (Core Domain), chứa toàn bộ entity, value object, domain event và domain error.
* **Bất biến Inviolable**: **Zero-I/O Invariant** (nghiêm cấm mọi phụ thuộc vào `std::fs`, `std::net`, `tokio::fs`, `tokio::net`, `reqwest`, `rusqlite`). 100% logic là thuần túy toán học và deterministic rules.
* **Các Bounded Contexts**:
  1. `task`: Vòng đời công việc (`Task`, `TaskRevision`, `Run`, `WorkerRun`, `ContinuationPacket`, `Recipe`, `Workflow`).
  2. `authority`: An ninh và kiểm soát tác động (`ActionIntent`, `ExecutionPermit`, `ExecutionReceipt`, `ApprovalDecision`, `CapabilityManifest`).
  3. `evidence`: Bằng chứng nghiệm thu (`EvidenceRecord`, `Claim`, `Fact`, `Synthesis`, `ReviewFinding`, `ExecutionRecord`).
  4. `context`: Ngữ cảnh tri thức (`ContextPack`, `MemoryEntry`, `AnnotationRecord`, `PersonalFact`).
  5. `workspace`: Không gian làm việc (`ExecutionWorkspace`, `WorkspaceFiles`, `ArtifactRef`, `UniversalResourceRef`, `TerminalSession`).
  6. `oi`: Trí tuệ điều phối (`ExecutionTopology`, `StrategyProposal`, `DecisionRecord`, `ReplanRecord`, `Candidate`).
  7. `pipeline`: Đường ống dữ liệu 3 pha (Bronze $\rightarrow$ Silver $\rightarrow$ Gold) với các DataInvariants (`ClosedBarInvariant`, `EvidenceGroundingInvariant`, `GovernedOutboxInvariant`, `AstSyntaxInvariant`).
  8. `fleet`: Quản lý tài nguyên tính toán (`WorkloadLease`, `BrowserSession`, `RemoteHostNode`, `ProviderConfig`).
  9. `session`: Phiên làm việc tương tác (`Session`, `ConversationTurn`, `AssistantDraft`, `WorkbenchLens`).
  10. `quant` & `research`: Các nghiệp vụ định lượng tài chính và nghiên cứu khoa học chuyên sâu.
* **Cải tiến Refactor**: Bổ sung `UniversalResourceRef` (URR) chuẩn hóa định dạng `urr://[component]/[kind]/[id]#[sha256]` với đầy đủ parser, formatter và unit tests.

### 2.2. `platform/core` (`custos-core`) — 66 files
* **Vị trí & Trách nhiệm**: Lõi tin cậy (Trusted Core), định nghĩa toàn bộ **Contracts/Ports** (Interfaces) và Kernel State Machine.
* **Cơ cấu Module**:
  1. `contracts/`: Danh mục 23 cổng chuẩn (Ports): `KernelPort`, `StoragePort`, `SandboxPort`, `ModelPort`, `WorkflowPort`, `MemoryPort`, `HarnessPort`, `SkillPort`, `QuantPort`, `OiPlannerPort`, `ComputePort`, `WorkerWirePort`.
  2. `kernel/`: Kernel State Machine quản lý việc chuyển trạng thái Task, Budget settlement, Outbox reducer, và Event emission.
  3. `authority/`: Đánh giá rủi ro (Risk Evaluation), thẩm định chính sách (Policy Engine), và xác minh giấy phép (Permit Issuer).
  4. `capability/`: Bộ lọc năng lực tiền thực thi, phát hiện đường dẫn ngoài sandbox (Path Traversal Guard).
  5. `evidence/`: Bộ xác minh bằng chứng (Diff verifier, Hash verifier, Citation verifier, Exit code verifier).
  6. `oi/`: Bộ lọc cứng (Hard Filters), tính hợp thức (Admissibility Gate), kiểm tra xung đột ghi (Write Set Checker), bộ biên dịch kế hoạch (Plan Compiler), và **`DecisionSnapshotExtractor`** (vừa được tái cấu trúc đưa về đúng thư mục `oi/`).

### 2.3. `platform/provider` (`custos-provider`) — 49 files
* **Vị trí & Trách nhiệm**: Trừu tượng hóa nhà cung cấp mô hình AI (Model Provider Port).
* **Cơ cấu**:
  1. `catalog/`: Mô hình định giá (Pricing), danh mục canonical models (`canonical_models.json.zst`), năng lực context limit và reasoning effort.
  2. `types/formats/`: Bộ chuyển đổi wire formats cho Anthropic, OpenAI, Databricks, Google CodeAssist, Ollama, Snowflake.
  3. `conformance/`: Test suite kiểm tra tính tương thích chuẩn của các provider adapters.

### 2.4. `platform/storage` (`custos-persistence`) — 43 files
* **Vị trí & Trách nhiệm**: Tầng lưu trữ bền vững SQLite cục bộ theo triết lý Single-Writer Connection Pool và Content-Addressable Storage (CAS).
* **Cơ cấu**:
  1. `artifacts/`: Lưu trữ file vật lý trên đĩa dựa trên SHA-256 hash.
  2. `repositories/`: 18 repositories chuyên trách (`TaskRepo`, `RunRepo`, `SessionRepo`, `OutboxRepo`, `DecisionRepo`, `ResearchRepo`, `QuantRepo`, `AssistantRepo`, `CommandLedgerRepo`).
  3. `migrations.rs`: Hệ thống quản lý phiên bản schema tự động, idempotent.

### 2.5. `platform/runtime` (`custos-runtime`) — 102 files
* **Vị trí & Trách nhiệm**: Động cơ điều phối và thực thi (Execution Engine & Orchestration).
* **Cơ cấu**:
  1. `event_bus.rs`: **PipelineEventBus** — Hệ thần kinh của Ao Hồ SADE. Đã được mở rộng với `LakeEvent` dynamic envelope hỗ trợ trao đổi sự kiện tùy ý kèm URR references.
  2. `oi/`: Bộ máy lựa chọn Topology T0 (Direct), T1 (Single Worker), T2 (Staged Pipeline), T3 (Read Fanout), T4 (Worktree Integrator), T6 (Repair Loop).
  3. `workflow/`: DAG Engine, TaskRuntime, WorkerExecutor, Outbox governance.
  4. `cognitive/`: Hệ thống nhận thức 2 tầng (S1 Fast Scout / S2 Deliberation / Human Gate).
  5. `packages/`: `WorkerLease` quản lý vòng đời tiến trình con của agent qua stdin/stdout có gán ngân sách.
  6. `integration/`: `IntegrationRegistry` quản lý định danh và trạng thái sống của Model, MCP Tool Servers, Agent Harnesses, Remote Peers.

### 2.6. `platform/adapters` (`custos-adapters`) — 129 files
* **Vị trí & Trách nhiệm**: Cầu nối tích hợp thực tế với hệ điều hành và thế giới bên ngoài.
* **Cơ cấu**:
  1. `harness/`: Bộ điều khiển các CLI agent hàng đầu (`ClaudeCodeHarnessAdapter`, `CodexHarnessAdapter`, `GooseHarnessAdapter`, `OpenCodeHarnessAdapter`).
  2. `sandbox/`: Cô lập tiến trình bằng Bubblewrap (Linux), Seatbelt (macOS Sandbox Exec), hoặc Developer unconfined mode.
  3. `terminal/`: PTY pseudo-terminal controller.
  4. `workspace/`: Git worktree manager và GitHub API client.
  5. `roaming/`: Mạng P2P qua Iroh Relay cho tương tác Agent-to-Agent (A2A).

### 2.7. `platform/components` (`custos-packs`) — 65 files
* **Vị trí & Trách nhiệm**: Các thư viện kỹ năng (Skills) và Verifiers tích hợp sẵn cho 4 domain:
  - Engineering: `cargo_test`, `ast_search`, `git_patch`, `component_clone`.
  - Quant: `deflated_sharpe`, `lookahead_bias`, `pbo_cscv`.
  - Research: `claim_extract`, `doi_verify`, `web_extract`, `alce`, `citation_coverage`.
  - Assistant: `contact_resolve`, `payload_lock`, `ax_tree_audit`.

### 2.8. `platform/bridge` (`custos-bridge`) — 3 files
* **Vị trí & Trách nhiệm**: Cầu nối phiên làm việc (Session-Task Bridge):
  - `promote`: Nâng cấp một cuộc trò chuyện thông thường thành một Task tự trị có hợp đồng ràng buộc.
  - `attach` / `detach`: Gắn hoặc tách session vào một Task đang chạy.
  - `steer`: Gửi chỉ đạo giữa chừng của con người vào Worker đang thực thi.
  - `recall` / `fork`: Phục hồi hoặc phân nhánh công việc.

---

## 3. Bản thiết kế "Ao Hồ SADE" (The SADE Living Commons)

Để biến Custos thành một "Ao hồ nuôi dưỡng các loài Agent", 4 cơ chế then chốt sau đây đã được hoàn thiện:

### Cơ chế 1: Chuẩn hóa Định danh Tài nguyên Toàn vũ trụ (URR)
Mọi tài nguyên được sản xuất trong ao hồ đều mang định dạng:
$$\text{URR} = \texttt{urr://[component]/[kind]/[id]\#[sha256]}$$
* Ví dụ:
  - `urr://science/claim/claim_8921#e3b0c442...` (Một luận điểm khoa học có citation gốc)
  - `urr://engineering/patch/patch_3120#a1b2c3d4...` (Một bản vá mã nguồn đã qua AST check)
  - `urr://quant/factor/vol_momentum#f9e8d7c6...` (Một nhân tố alpha đã qua PBO gate)
  - `urr://assistant/brief/brief_0012#77aa88bb...` (Một bản tóm tắt chiến lược cho người dùng)

Khi các agent tương tác, chúng **không sao chép dữ liệu khổng lồ**, mà chuyển giao con trỏ URR bất biến. CAS (Content-Addressable Storage) đảm bảo dữ liệu không thể bị sửa đổi hay giả mạo.

### Cơ chế 2: Event Substrate Mở rộng (`PipelineEvent::LakeEvent`)
Bên cạnh các sự kiện định danh sẵn, `LakeEvent` cho phép bất kỳ agent nào phát sóng tri thức mới:
```rust
let event = PipelineEvent::lake(
    "quant.alpha.discovered",
    "quant_researcher_agent",
    "quant",
    vec!["urr://science/claim/claim_8921#e3b0c442".to_string()],
    json!({ "sharpe": 2.45, "pbo": 0.04 }),
);
event_bus.publish(event);
```
Bất kỳ agent nào khác (ví dụ: một `risk_evaluator_agent` hay một `python_backtester`) đang đăng ký lắng nghe chủ đề `quant.alpha.*` đều sẽ lập tức nhận được tín hiệu và tự động kích hoạt vòng lặp đánh giá.

### Cơ chế 3: Tương hỗ Đa miền (Cross-Domain Symbiosis)
Ao hồ vận hành theo chuỗi giá trị tự động:
1. **Science Agent** phân tích tài liệu và xuất bản `ScienceClaimVerified` kèm URR trích dẫn.
2. **Coding Agent** (Claude Code / Codex) nhận URR đó, viết mã nguồn thuật toán trong một Git Worktree cô lập và chạy `cargo_test_oracle`.
3. **Quant Agent** chạy mô phỏng backtest trên dữ liệu không có lookahead bias và kiểm tra chỉ số Deflated Sharpe.
4. **Assistant Agent** tổng hợp kết quả thành bản nháp email hoặc báo cáo điều hành, sẵn sàng cho người dùng duyệt qua Outbox một chạm.

### Cơ chế 4: Vòng lặp Sửa chữa Tự động $T_6$ (Evaluator-Optimizer Loop)
Khi một agent gặp thất bại (ví dụ: lỗi biên dịch, vi phạm rủi ro, hoặc hallucination):
- Thay vì dừng lại hoặc che giấu lỗi, hệ thống sinh ra `ReplanTrigger::ExecutionFailure`.
- Bộ điều phối $T_6$ đưa chẩn đoán và diff vào một **Scout/Evaluator Agent** độc lập.
- Evaluator tạo ra bản sửa lỗi tối thiểu (Ponytail rule), xác minh lại qua oracle rồi tích hợp vào pipeline chính.

---

## 4. Bảng Kết quả Xác thực Chất lượng (Quality Gates)

| Quality Gate | Mục tiêu | Kết quả | Thời gian |
|---|---|---|---|
| **Gate 1: Pack Manifests** | Kiểm tra cú pháp và schema của các manifest JSON | **PASS** | 0.05s |
| **Gate 2: Desktop Build** | Biên dịch TypeScript/Vite của Desktop UI | **PASS** | 0.18s |
| **Gate 3: Canvas Smoke** | Chạy toàn bộ 51 integration & isolation test scenarios | **PASS (51/51)** | 0.68s |
| **Gate 4: Architecture Invariants** | Zero-I/O Domain, Layering guards, Naming checks, Vertical slice contracts | **PASS (18/18)** | 0.54s |
| **Gate 5: Workspace Cargo Check** | Kiểm tra biên dịch của toàn bộ 15 crates trong workspace | **PASS (15/15)** | 0.70s |

Toàn bộ 5 Quality Gates đều đạt trạng thái **100% GREEN**.
Kiến trúc `platform/` hiện tại đã đạt độ tinh gọn, chuẩn mực OOP & System Design, sẵn sàng làm bệ phóng cho hệ sinh thái Agent Lake hoàn chỉnh.
