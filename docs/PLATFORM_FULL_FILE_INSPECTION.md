# CUSTOS — BÁCH KHOA TOÀN THƯ TOÀN BỘ CÁC TỆP TIN TRONG PLATFORM/
## Khảo Sát Chi Tiết Toàn Bộ 469 Tệp Mã Nguồn, Chức Năng, Cấu Trúc Dữ Liệu & Hướng Xử Lý

> **Tài liệu kiểm toán và giải phẫu chi tiết toàn bộ tệp tin trong `/Users/mac/Project/AgentHub/Custos/platform`**  
> **Trạng thái:** CANONICAL REPOBASE ENCYCLOPEDIA  
> **Tổng quy mô:** 8 Crates, 469 Tệp, ~105.400 dòng mã nguồn  

---

## MỤC LỤC TỔNG QUAN 8 CRATES

1. [Crate 1: `platform/domain` (Layer 0 — Zero-I/O Domain Entities)](#1-platformdomain-custos-domain) — 38 tệp
2. [Crate 2: `platform/core` (Layer 1 — Authority, Invariants & Gates)](#2-platformcore-custos-core) — 52 tệp
3. [Crate 3: `platform/runtime` (Layer 2 — Cognitive Loop & Orchestration)](#3-platformruntime-custos-runtime) — 87 tệp
4. [Crate 4: `platform/storage` (Layer 2 — Single-Writer SQLite & CAS)](#4-platformstorage-custos-persistence) — 38 tệp + 44 migrations
5. [Crate 5: `platform/bridge` (Layer 2 — Session/Task Promotion)](#5-platformbridge-custos-bridge) — 3 tệp
6. [Crate 6: `platform/ai` (Pillar 1 — Unified AI Subsystem)](#6-platformai-custos-provider) — 85 tệp
7. [Crate 7: `platform/adapters` (Pillar 2 — Host OS Drivers & Protocols)](#7-platformadapters-custos-adapters) — 93 tệp
8. [Crate 8: `platform/components` (Pillar 3 — Extension Host & Lifecycle Engine)](#8-platformcomponents-custos-packs) — 56 tệp

---

## 1. `platform/domain` (`custos-domain`)
> **Tôn chỉ:** 100% Zero-I/O Invariant. Không chứa bất kỳ I/O, database, network, hay filesystem nào.

### Nhóm `src/authority/` (Mô hình thẩm quyền)
1. **`authority/action.rs`**
   - *Chức năng:* Định nghĩa phân loại hành động và ý định hành động của tác nhân.
   - *Structs/Enums:* `ActionKind` (`Read`, `WorkspaceWrite`, `SystemExecute`, `NetworkEgress`), `ActionIntent`.
   - *Quan hệ:* Được `core::authority` sử dụng để cấp quyền; `runtime` dùng để đóng gói đề xuất tool call.
   - *Hướng RFT:* Giữ nguyên trong Base Domain.
2. **`authority/approval.rs`**
   - *Chức năng:* Trạng thái phê duyệt can thiệp của con người (Human-in-the-Loop).
   - *Structs/Enums:* `ApprovalStatus` (`Pending`, `Approved`, `Rejected`), `HumanApprovalPolicy`, `ApprovalRequest`.
   - *Quan hệ:* Kết nối với Cockpit UI qua Local API để hiển thị modal xác nhận cho người dùng.
   - *Hướng RFT:* Giữ nguyên trong Base Domain.
3. **`authority/capability.rs`**
   - *Chức năng:* Khái niệm Capability định danh quyền hạn truy cập tài nguyên.
   - *Structs/Enums:* `CapabilityScope`, `CapabilityDescriptor`, `CapabilityId`.
   - *Quan hệ:* Được sử dụng bởi `core::kernel` và `components::registry`.
   - *Hướng RFT:* Chuẩn hóa `CapabilityId` thành newtype strongly-typed.
4. **`authority/permit.rs`**
   - *Chức năng:* Mô hình giấy phép thực thi dùng một lần duy nhất.
   - *Structs/Enums:* `Permit` (chứa `id`, `task_id`, `action_id`, `capability`, `argument_digest`, `expires_at`, `consumed`).
   - *Quan hệ:* Trái tim của cơ chế bảo mật INV-04; do `core::PermitIssuer` cấp phát và tiêu hủy.
   - *Hướng RFT:* Đổi `action_id` và `capability` từ `String` sang strongly-typed IDs (`ActionId`, `CapabilityId`).
5. **`authority/mod.rs`**: Re-export các thành phần của module `authority`.

### Nhóm `src/task/` (Vòng đời Task, Run, Workflow)
6. **`task/task.rs`**
   - *Chức năng:* Thực thể trung tâm `Task` của Custos.
   - *Structs/Enums:* `Task`, `TaskId`, `TaskStatus` (`Pending`, `Running`, `Completed`, `Failed`, `Cancelled`), `TaskContract`, `TaskPriority`.
   - *Quan hệ:* Được lưu trữ trong `storage::repositories::task`; được điều khiển bởi `core::TaskService`.
   - *Hướng RFT:* Giữ nguyên làm SSOT của hệ thống.
7. **`task/run.rs`**
   - *Chức năng:* Thực thể `Run` đại diện cho một lượt chạy thực thi cụ thể của Task.
   - *Structs/Enums:* `Run`, `RunId`, `RunStatus`, `ExecutionAttempt`, `AttemptStatus`.
   - *Quan hệ:* Theo dõi tiến độ runtime; lưu trong `storage::repositories::run`.
   - *Hướng RFT:* Giữ nguyên trong Base Domain.
8. **`task/workflow.rs`**
   - *Chức năng:* Định nghĩa đồ thị nhiệm vụ DAG.
   - *Structs/Enums:* `WorkflowGraph`, `WorkflowNode`, `NodeDependency`, `StepCondition`.
   - *Quan hệ:* Được `runtime::workflow::dag` phân giải và lên lịch.
   - *Hướng RFT:* Giữ nguyên trong Base Domain.
9. **`task/recipe.rs`**
   - *Chức năng:* Kịch bản thực thi đóng băng bất biến.
   - *Structs/Enums:* `FrozenRecipe`, `RecipeStep`, `RecipeParameter`.
   - *Quan hệ:* Cung cấp template thực thi có sẵn từ các domain packs.
   - *Hướng RFT:* Giữ cấu trúc chung; các recipe chuyên biệt chuyển về từng workbench.
10. **`task/continuation.rs`**
    - *Chức năng:* Gói dữ liệu chuyển giao khi task bị gián đoạn cần tiếp tục (`ContinuationPacket`).
    - *Structs/Enums:* `ContinuationPacket`, `CheckpointState`, `PendingEvidence`.
    - *Quan hệ:* Dùng cho việc crash recovery và resume phiên làm việc.
    - *Hướng RFT:* Giữ nguyên trong Base Domain.
11. **`task/decision.rs`**
    - *Chức năng:* Ghi nhận các quyết định rẽ nhánh của agent trong quá trình suy luận.
    - *Structs/Enums:* `TaskDecision`, `DecisionRationale`, `AlternativeConsidered`.
    - *Quan hệ:* Lưu trữ trong `storage::repositories::decision` phục vụ giải trình (explainability).
    - *Hướng RFT:* Giữ nguyên trong Base Domain.
12. **`task/mod.rs`**: Re-export module task.

### Nhóm `src/evidence/` (Chứng cứ & Thẩm định)
13. **`evidence/evidence.rs`**
    - *Chức năng:* Mô hình chứng cứ hoàn thành nhiệm vụ.
    - *Structs/Enums:* `EvidenceKind` (`ArtifactHash`, `TestPass`, `HumanConsent`, `ExternalValidation`), `ContractEvidence`, `EvidenceStatus`.
    - *Quan hệ:* Cung cấp đầu vào cho `core::CompletionGate` (INV-02).
    - *Hướng RFT:* Giữ nguyên làm nền tảng kiểm chứng.
14. **`evidence/claim.rs`**
    - *Chức năng:* Tuyên bố cần kiểm chứng (`VerificationClaim`).
    - *Structs/Enums:* `VerificationClaim`, `ClaimId`, `ClaimSource`.
    - *Hướng RFT:* Các claim khoa học (`PaperClaim`) chuyển về Research; claim kỹ thuật giữ tại đây.
15. **`evidence/fact.rs`**, **`evidence/execution_record.rs`**, **`evidence/review.rs`**, **`evidence/synthesis.rs`**
    - *Chức năng:* Dữ kiện thực nghiệm, bản ghi kết quả, nhận xét review và tổng hợp bằng chứng.
    - *Hướng RFT:* Chuẩn hóa thành các evidence DTOs cơ bản.
16. **`evidence/mod.rs`**: Re-export module evidence.

### Nhóm `src/workspace/` (Không gian làm việc & Worktrees)
17. **`workspace/workspace.rs`**: `ExecutionWorkspace`, `WorkspaceId`, thư mục làm việc, Git branch liên kết.
18. **`workspace/workspace_files.rs`**: `WorkspaceFileContent`, `WorkspaceFileTree`, `FileMetadata`.
19. **`workspace/artifact.rs`**: Siêu dữ liệu CAS artifact (`ArtifactId`, `Sha256Digest`, `MimeType`, `ByteSize`).
20. **`workspace/terminal.rs`**: Trạng thái phiên PTY ảo (`TerminalSessionId`, `Cols`, `Rows`).
21. **`workspace/repo/mod.rs`**, **`workspace/repo/span.rs`**: `CodeSpan` (tọa độ file, dòng bắt đầu, dòng kết thúc).
22. **`workspace/mod.rs`**: Re-export module workspace.

### Nhóm `src/session/` (Phiên làm việc & Trợ lý)
23. **`session/session.rs`**: `Session`, `SessionId`, `ConversationMessage`, `AuthorRole`.
24. **`session/assistant.rs`**: `AssistantPersona`, `ContextPreferences`.
25. **`session/automation.rs`**: `AutomationTrigger`, `ScheduledJob`.
26. **`session/mod.rs`**: Re-export module session.

### Nhóm `src/context/` (Ngữ cảnh nạp vào mô hình)
27. **`context/context.rs`**: `ContextPack`, `ContextSource`, `ContextBudget`.
28. **`context/memory.rs`**: `MemoryItem`, `MemoryScope` (`Episodic`, `Semantic`).
29. **`context/annotation.rs`**: `ContextAnnotation`, `HighlightRange`.
30. **`context/mod.rs`**: Re-export module context.

### Nhóm `src/shared/` (Tiện ích & Định danh dùng chung)
31. **`shared/ids.rs`**: Định nghĩa toàn bộ Strongly-Typed IDs (`TaskId`, `SessionId`, `RunId`, `WorkerId`).
32. **`shared/error.rs`**: `DomainError` — enum lỗi nghiệp vụ toàn hệ thống (Validation, NotFound, InvariantViolation).
33. **`shared/budget.rs`**: `TokenBudget`, `CostLimitUsd`.
34. **`shared/span.rs`**, **`shared/types.rs`**, **`shared/packages.rs`**, **`shared/packet.rs`**, **`shared/mod.rs`**.

### Nhóm Specialist Domains (Cần bóc tách về Workbenches)
35. **`src/quant/`** (`order.rs`, `orderbook.rs`, `risk.rs`, `backtest.rs`, `factor.rs`, `pbo.rs`, `gate.rs`, `grid.rs`, `lease.rs`, `metrics.rs`, `portal.rs`, `review.rs`, `contracts.rs`, `engine.rs`, `mod.rs`):
    - *Chức năng:* Toàn bộ mô hình dữ liệu tài chính định lượng.
    - *Hướng RFT:* **Chuyển toàn bộ về `workbenches/quant/domain/`** để giải phóng Base Platform.
36. **`src/research/`** (`experiment.rs`, `notebook.rs`, `note.rs`, `mod.rs`):
    - *Chức năng:* Mô hình bài báo khoa học, thí nghiệm nghiên cứu.
    - *Hướng RFT:* **Chuyển về `workbenches/research/domain/`**.
37. **`src/fleet/`** (`compute.rs`, `browser.rs`, `remote_fleet.rs`, `provider_config.rs`, `mod.rs`):
    - *Chức năng:* Quản lý trạng thái fleet máy chủ từ xa.
    - *Hướng RFT:* Chuyển về `resources/` hoặc extension tương ứng.
38. **`src/pipeline/`** (`structure.rs`, `mod.rs`): Cấu trúc pipeline xử lý dữ liệu.

---

## 2. `platform/core` (`custos-core`)
> **Tôn chỉ:** Lõi bất biến kiểm soát thẩm quyền (Authority & Invariant Gatekeeper).

### Nhóm `src/kernel/` (Dịch vụ điều hành trung tâm)
39. **`kernel/service.rs`**
    - *Chức năng:* `TaskService` tiếp nhận và xử lý 4 commands tối cao: `CreateTask`, `AdvanceTask`, `CompleteTask`, `CancelTask`.
    - *Structs:* `TaskService<S: StoragePort>`.
    - *Quan hệ:* Giao tiếp trực tiếp với `storage` để commit state transitions.
40. **`kernel/completion.rs`**
    - *Chức năng:* **`CompletionGate` (Bảo đảm INV-02)**. Chặn đứng hoàn thành task nếu thiếu `EvidenceBundle` chứng minh đầy đủ tiêu chí của `TaskContract`.
    - *Logic:* So khớp `source_version`, cấm self-certification, từ chối chứng cứ của task khác.
41. **`kernel/state_machine.rs`**: Máy trạng thái tuần tự bất biến của Task (`Pending -> Running -> Completed/Failed`).
42. **`kernel/invariants.rs`**: Định nghĩa và kiểm tra các bất biến hệ thống trước mỗi transition.
43. **`kernel/reducer.rs`**: Pure function tính toán trạng thái tiếp theo từ state cũ và event.
44. **`kernel/budget.rs`**: Giám sát giới hạn token và ngân sách tài chính của từng task.
45. **`kernel/observability.rs`**: Thu thập metrics, vết span thực thi.
46. **`kernel/commands.rs`**, **`kernel/events.rs`**: Định nghĩa các lệnh và sự kiện nội bộ của kernel.
47. **`kernel/session_state_machine.rs`**, **`kernel/span_service.rs`**, **`kernel/ports.rs`**, **`kernel/mod.rs`**.

### Nhóm `src/authority/` (Cấp phát & Quản lý Thẩm Quyền)
48. **`authority/permits.rs`**
    - *Chức năng:* **`PermitIssuer` (Bảo đảm INV-04)**.
    - *Logic:* Cấp phát giấy phép `Permit` kèm SHA-256 digest của tham số. Hàm `consume_permit` kiểm tra nguyên tử: còn hạn TTL $\rightarrow$ chưa dùng $\rightarrow$ khớp mã băm $\rightarrow$ tiêu hủy ngay (chống Confused Deputy & Replay Attack).
49. **`authority/policy.rs`**: Chính sách thẩm quyền theo mức rủi ro (`PermitPolicy`, `AuthorityTier`).
50. **`authority/risk.rs`**: Đánh giá mức độ rủi ro của từng lệnh (`Low`, `Medium`, `High`, `Critical`).
51. **`authority/approvals.rs`**: Hàng đợi yêu cầu phê duyệt từ con người cho các lệnh rủi ro cao.
52. **`authority/grants.rs`**: Quản lý các quyền hạn được cấp phát dài hạn.
53. **`authority/audit.rs`**: Ghi vết kiểm toán bất biến cho mọi hành động thực thi.
54. **`authority/mod.rs`**: Re-export module authority.

### Nhóm `src/evidence/` (Pipeline Thẩm Định Chứng Cứ)
55. **`evidence/bundle.rs`**: `EvidenceBundle` cấu trúc đóng gói bằng chứng.
56. **`evidence/pipeline.rs`**: Quy trình xử lý bằng chứng từ thu thập đến xác minh.
57. **`evidence/verifier.rs`**: Trait `ObjectiveVerifier` định nghĩa cách thức kiểm chứng khách quan.
58. **`evidence/mod.rs`**: Re-export module evidence.

### Nhóm `src/contracts/` (Hệ thống Hợp Đồng Cổng Trừu Tượng — Ports)
59-81. **23 tệp contracts:**
    - `kernel.rs`: `KernelPort`.
    - `workflow.rs`: `WorkflowPort`.
    - `storage.rs`: `StoragePort`.
    - `sandbox.rs`: `SandboxPort`.
    - `terminal.rs`: `TerminalPort`.
    - `workspace.rs`: `WorkspacePort`.
    - `harness.rs`: `HarnessPort`.
    - `credentials.rs`: `CredentialVault`.
    - `packages.rs`: `PackagePort`.
    - `skill.rs`: `SkillPort`.
    - `source_acquisition.rs`: `SourceAcquisitionPort`.
    - *Các cổng chuyên biệt cần chuẩn hóa sang Extension Ports:* `compute.rs`, `notebook.rs`, `checkpoint_store.rs`, `judgment.rs`, `quant.rs`, `research_ingress.rs`, `workspace_files.rs`, `worker_wire.rs`, `oi.rs`, `memory.rs`, `mod.rs`.
    - *Hướng RFT:* Phân chia thành `contracts::host` và `contracts::extensions`.

### Nhóm `src/context/` & `src/oi/` & `src/capability/`
82. **`context/mod.rs`**, `anchor.rs`, `compaction.rs`, `dedup.rs`, `redaction.rs`, `structural.rs`: Định nghĩa các luật bất biến về ngữ cảnh (che giấu secret, khử trùng lặp).
83. **`oi/mod.rs`**, `admissibility.rs`, `compiler.rs`, `hard_filters.rs`, `join_policy.rs`, `snapshot_extractor.rs`, `write_set.rs`: Bộ lọc cứng và kiểm tra tính hợp lệ của topology.
84. **`capability/mod.rs`**, `deterministic.rs`, `traits.rs`.
85. **`sandbox_policy/mod.rs`**, `path_sandbox.rs`: Kiểm tra giới hạn đường dẫn truy cập đĩa.
86. **`src/decision.rs`**, **`src/lib.rs`**: Root exports của core.

---

## 3. `platform/runtime` (`custos-runtime`)
> **Tôn chỉ:** Động cơ điều phối vòng lặp nhận thức và luồng thực thi (Cognitive Loop & Execution).

### Nhóm `src/workflow/` (Điều phối thực thi nhiệm vụ)
87. **`workflow/task_runtime/execution.rs` (1.358 lines — Điểm nóng kiến trúc lớn nhất)**
    - *Chức năng:* Trái tim của runtime: đàm phán mode (`model` vs `native`), biên dịch prompt, gọi suy luận, xử lý tool call proposal, xin permit từ kernel, gọi adapter, thu thập bằng chứng và thử nghiệm hoàn thành task.
    - *Hướng RFT:* **Bắt buộc phân rã thành 4 sub-modules:** `mode_negotiator.rs`, `cognitive_turn.rs`, `tool_dispatcher.rs`, `terminal_evaluator.rs`.
88. **`workflow/task_runtime/mod.rs`**: Định nghĩa struct `TaskRuntime`, kết nối các ports.
89. **`workflow/task_runtime/tests.rs`**: Bộ unit test kiểm tra luồng chạy của task runtime.
90. **`workflow/machine.rs`**: Vòng lặp nhận thức `Perception -> Plan -> Execute -> Verify`.
91. **`workflow/dag.rs`**: Bộ phân giải đồ thị phụ thuộc và thứ tự thực thi các bước.
92. **`workflow/outbox.rs`**: Transactional Outbox đảm bảo lệnh ngoại vi chỉ gửi khi SQLite transaction commit thành công.
93. **`workflow/scheduler.rs`**: Lên lịch thực thi các bước sẵn sàng.
94. **`workflow/worker_executor.rs`**: Thực thi worker trong thread/process riêng.
95. **`workflow/dispatcher.rs`**, `graph_runtime.rs`, `lease.rs`, `live.rs`, `revision_loader.rs`, `contracts.rs`, `mod.rs`.

### Nhóm `src/context/` (Trình biên dịch ngữ cảnh RTK & Nén bộ nhớ)
96. **`context/rtk.rs`**: Real-time Knowledge Compiler biên dịch tri thức thời gian thực.
97. **`context/compiler.rs`**: `TokenAwareContextCompiler` co kéo và nén tài liệu vừa vặn với context window.
98. **`context/compaction/`** (10 tệp: `format.rs`, `model.rs`, `orchestrator.rs`, `provider.rs`, `structured.rs`, `summarize.rs`, `templates.rs`, `mod.rs`, prompts): Tự động tóm tắt phân đoạn khi hội thoại chạm trần token.
99. **`context/memory/`** (`mod.rs`, `store.rs`, `traits.rs`): Kho lưu trữ bộ nhớ ngữ cảnh ngắn hạn/dài hạn.
100. **`context/token_counter.rs`**: Đếm token BPE chuẩn xác.
101. **`context/traits.rs`**, **`context/mod.rs`**: Trait trừu tượng và module root.

### Nhóm `src/cognitive/` (Tư duy nhận thức S1/S2 & Kỹ năng nền tảng)
102. **`cognitive/deliberation.rs`**, `arbiter.rs`, `human_gate.rs`: Suy ngẫm S2, phân xử xung đột, cổng dừng xin ý kiến người dùng.
103. **`cognitive/s1/`** (`judge.rs`, `scout.rs`, `micro_executor.rs`, `calibration.rs`, `mod.rs`): Hệ thống tư duy phản xạ nhanh S1 (Fast Heuristics).
104. **`cognitive/skills/`** (Kỹ năng cơ sở):
    - `fs/file_read.rs`, `fs/mod.rs`: Đọc file trong giới hạn an toàn.
    - `http/fetch_url.rs`, `http/mod.rs`: Gọi HTTP GET kiểm soát.
    - `process/shell_exec.rs`, `process/mod.rs`: Chạy subprocess trong sandbox.
    - `search/lexical_search.rs`, `search/mod.rs`: Tìm kiếm từ vựng.
105. **`cognitive/pipeline.rs`**, `prompt_manager.rs`, `provider_selector.rs`, `rdc.rs`, `registry.rs`, `routing.rs`, `signal_extractor.rs`, `config.rs`, `mod.rs`.

### Nhóm `src/compute/` (Quản trị GPU & Tính toán)
106. **`compute/admission.rs`**: `WorkloadAdmissionCoordinator` ngăn chặn xung đột VRAM/GPU giữa Training và Inference. *(Hướng RFT: Bỏ static singleton, chuyển sang Dependency Injection)*.
107. **`compute/training.rs`**, `evaluation.rs`, `placement.rs`, `relay.rs`, `remote.rs`, `mod.rs`.

### Nhóm điều phối Coordinator & Orchestrated Intelligence (OI)
108. **`src/oi/`** (10 tệp: `engine.rs`, `candidate_builder.rs`, `estimator.rs`, `explain.rs`, `replanner.rs`, `selector.rs`, `baseline.rs`, `mod.rs` và 6 topologies: `t0_direct`, `t1_single_worker`, `t2_staged_pipeline`, `t3_read_fanout`, `t4_worktree_integrator`, `t6_repair_loop`).
109. **`src/workspace/`** (`coordinator.rs`, `files.rs`, `mod.rs`): Quản lý trạng thái workspace đang chạy.
110. **`src/terminal/`** (`coordinator.rs`, `buffer.rs`, `mod.rs`): Quản lý buffer và phiên PTY ảo đang chạy.
111. **`src/session/`** (`manager.rs`, `journal.rs`, `mod.rs`): Quản lý các phiên tương tác trong RAM.
112. **`src/notebook/`** (`coordinator.rs`, `mod.rs`): Quản lý vòng đời kernel Python/Jupyter.
113. **`src/agent/`**, `src/clock.rs`, `src/event_bus.rs`, `src/integration/`, `src/packages/`, `src/lib.rs`.
114. **`tests/notebook_binding_golden_test.rs`**: Golden test kiểm tra kết nối notebook.

---

## 4. `platform/storage` (`custos-persistence`)
> **Tôn chỉ:** Hạ tầng lưu trữ bền vững Single-Writer WAL SQLite và Content-Addressed Storage (CAS).

### Nhóm Kết Nối & Động Cơ Migration
115. **`src/connection.rs`**
    - *Chức năng:* Quản lý kết nối SQLite thread-safe với **Single Sovereign Writer Lock** (`Arc<Mutex<Connection>>`) và **Reader Pool 16 kết nối đọc song song** (`Arc<ReaderPool>`).
    - *Cấu hình:* WAL mode, `busy_timeout(5000ms)`, `foreign_keys=ON`, `synchronous=NORMAL`.
116. **`src/migrations.rs`**
    - *Chức năng:* Trình chạy hơn 40 bản migration SQL định nghĩa toàn bộ schema dữ liệu.
    - *Hướng RFT:* Tách chuỗi SQL trong code Rust ra các file `.sql` độc lập.
117. **`migrations/` (44 tệp `.sql` từ `0001_core.sql` đến `0044_universal_resource_registry.sql`)**:
    - Định nghĩa toàn bộ các bảng: `tasks`, `runs`, `sessions`, `workspaces`, `command_ledger`, `outbox`, `providers`, `credentials`, `package_inventory`, `resource_registry`, v.v.

### Nhóm Repositories Truy Vấn Dữ Liệu
118. **`repositories/task.rs`**: CRUD và chuyển trạng thái nhiệm vụ.
119. **`repositories/run.rs`**: CRUD các lượt chạy run và attempt.
120. **`repositories/session.rs`**: Lưu trữ phiên chat và tin nhắn hội thoại.
121. **`repositories/workspace.rs`**: Lưu vết không gian làm việc và Git branch.
122. **`repositories/command_ledger.rs`**: Sổ cái ghi nhận mọi command Local API phục vụ kiểm toán và phục hồi sau sự cố.
123. **`repositories/outbox.rs`**: Hàng đợi các lệnh effect ngoại vi chờ dispatch.
124. **`repositories/packages.rs`**: Bảng kiểm kê trạng thái component (`Installed`, `Enabled`).
125. **`repositories/provider_credentials.rs`**: Lưu trữ khóa bí mật đã được mã hóa.
126. **`repositories/providers/`** (`accounts.rs`, `catalog.rs`, `local_profiles.rs`, `usage.rs`, `mod.rs`): Quản lý tài khoản AI, TTL catalog, hồ sơ model local, và lượng token đã dùng.
127. **`repositories/research/`** (`sources.rs`, `notes.rs`, `claims.rs`, `notebooks.rs`, `reviews.rs`, `synthesis.rs`, `annotations.rs`, `artifacts.rs`, `executions.rs`, `mod.rs`): Lưu trữ dữ liệu nghiên cứu khoa học. *(Hướng RFT: Đóng gói theo extension Research)*.
128. **`repositories/quant.rs`**: Lưu trữ lịch sử giao dịch và kết quả backtest. *(Hướng RFT: Đóng gói theo extension Quant)*.
129. **`repositories/assistant.rs`**, `continuation.rs`, `decision.rs`, `experiment_attempt.rs`, `fleet_automation.rs`, `replan.rs`, `span.rs`, `workflow_revision.rs`, `mod.rs`.

### Nhóm CAS Artifacts & Store Root
130. **`artifacts/filesystem.rs`**: Lưu trữ tệp nhị phân lớn dưới định dạng SHA-256 CAS (`~/data/cas/ab/cd/...`).
131. **`artifacts/mod.rs`**, `artifacts/traits.rs`.
132. **`store/mod.rs`**: `SqliteTaskStore` tập hợp toàn bộ repositories thành một facade duy nhất.
133. **`store/seed.rs`**, `store/tests.rs`, `store/traits.rs`, `src/lib.rs`.

---

## 5. `platform/bridge` (`custos-bridge`)
> **Tôn chỉ:** Tầng chuyển giao phiên làm việc.

134. **`Cargo.toml`**, **`src/lib.rs`**.
135. **`src/port.rs`**: Định nghĩa trait `BridgePort` cho việc thăng hạng phiên làm việc.
136. **`src/service.rs`**: `BridgeService` xử lý logic `promote_session_to_task`, `attach_session_stream`, `steer_task`.
    - *Hướng RFT:* **Chuyển toàn bộ vào `apps/daemon/src/application/sessions/linkage/` và xóa bỏ crate này.**

---

## 6. `platform/ai` (`custos-provider`) — Pillar 1
> **Tôn chỉ:** Phân hệ trí tuệ nhân tạo thống nhất (Unified AI Subsystem).

### Nhóm `src/contracts/` (Hợp Đồng & Bảng Giá SSOT)
137. **`contracts/port.rs`**: Trait chuẩn tối cao `ModelPort` và `ModelProvider`.
138. **`contracts/request.rs`**: `ModelRequest`, `Message`, `ContentPart` (text, image multimodal, tool result).
139. **`contracts/turn.rs`**: `ModelTurn`, `TurnDelta`, `ReasoningDelta` (xử lý reasoning token streaming).
140. **`contracts/events.rs`**: Các sự kiện streaming chuẩn hóa (`TokenDelta`, `ToolProposed`, `Completed`).
141. **`contracts/tool.rs`**: Định nghĩa công cụ `ToolDefinition`, `ToolCall`, `ToolResult`.
142. **`contracts/catalog/pricing.rs`**: **Bảng giá SSOT duy nhất** định giá token đầu vào, đầu ra, reasoning của toàn bộ models.
143. **`contracts/catalog/`** (`models.rs`, `capabilities.rs`, `combo.rs`, `mod.rs`).
144. **`contracts/types/canonical/`** (`catalog.rs`, `model.rs`, `registry.rs`, `name_builder.rs`, `canonical_models.json.zst`): Giải nén và nạp danh mục model chuẩn hóa.
145. **`contracts/types/formats/`** (`openai.rs`, `anthropic.rs`, `google.rs`, `ollama.rs`, `databricks.rs`, `snowflake.rs`, `mod.rs`): Các cấu trúc wire format. *(Hướng RFT: Chuyển về `providers/`)*.
146. **`contracts/types/`** (`thinking.rs`, `context_limit.rs`, `custos_mode.rs`, `permission.rs`, `retry.rs`, `images.rs`, `json.rs`, `errors.rs`, `base.rs`...): Các kiểu dữ liệu phụ trợ.
147. **`contracts/traits.rs`**, `contracts/conformance.rs`, `contracts/mod.rs`.

### Nhóm `src/cloud/` (HTTP Cloud Clients)
148. **`cloud/openai.rs`**: Client OpenAI (GPT-4o, o1, o3-mini).
149. **`cloud/anthropic.rs`**: Client Anthropic (Claude 3.5 Sonnet, Claude 3.7 Sonnet).
150. **`cloud/google.rs`**: Client Google Gemini (Gemini 2.0 Flash, Gemini 1.5 Pro).
151. **`cloud/ollama.rs`**: Client Ollama cục bộ qua chuẩn OpenAI API.
152. **`cloud/google_codeassist/`**: Tích hợp Google Cloud Code Assist.
153. **`cloud/fake/`**: Fake model provider phục vụ kiểm thử đơn vị và testkit.
154. **`cloud/declarative/`**: Hỗ trợ nạp provider bằng file cấu hình JSON/YAML.
155. **`cloud/http_status.rs`**, `cloud/mod.rs`.

### Nhóm `src/local/` (On-Device Inference Engine — Cần Feature Gate)
156. **`local/local_inference/metal_probe.rs`**: Quét phần cứng Apple Silicon Metal và NVIDIA CUDA.
157. **`local/local_inference/gguf_inspector.rs`**: Đọc header GGUF bóc tách tham số và context limit.
158. **`local/local_inference/folder_scanner.rs`**: Quét thư mục tìm kiếm model GGUF trên đĩa.
159. **`local/local_inference/llamacpp/`** (5 tệp: `inference_engine.rs`, `inference_native_tools.rs`, `inference_emulated_tools.rs`, `mod.rs`): Quản lý sidecar llama.cpp với native/emulated tool calling.
160. **`local/local_inference/mlx.rs`**: Hỗ trợ engine Apple MLX.
161. **`local/download_manager/`**: Quản lý tải model từ Hugging Face Hub (hỗ trợ resume, SHA-256).
162. **`local/local_inference/`** (`adapter.rs`, `backend.rs`, `config_resolver.rs`, `management.rs`, `model.rs`, `multimodal.rs`, `native_tool_parsing.rs`, `paths.rs`, `prompt_template.rs`, `provider_utils.rs`, `sidecar.rs`, `thinking_output.rs`, `tool_emulation.rs`, `tool_parsing.rs`, `manifest.json`, `mod.rs`).

### Nhóm `src/auth/` & `src/routing/` (Xác thực & Định tuyến Kinh tế)
163. **`auth/oauth_pkce.rs`**: RFC 7636 OAuth PKCE loopback listener tại `127.0.0.1:1455`.
164. **`auth/credential_vault.rs`**: Tích hợp OS Keychain / File Vault mã hóa.
165. **`auth/mod.rs`**: Re-export auth.
166. **`routing/catalog.rs`**: Quản lý bộ nhớ đệm TTL của model availability.
167. **`routing/fallback.rs`**: Phân loại lỗi HTTP 429 và 402 để kích hoạt fallback cascade.
168. **`routing/probe.rs`**: Thăm dò danh sách model và hạn ngạch quota của tài khoản.
169. **`routing/router/`** (`catalog.rs`, `credentials.rs`, `wire_types.rs`, `tests.rs`, `mod.rs`): `RouterModelProvider` định tuyến thông minh.
170. **`routing/streaming.rs`**, `routing/mod.rs`, `build.rs`, `Cargo.toml`, `src/lib.rs`.

---

## 7. `platform/adapters` (`custos-adapters`) — Pillar 2
> **Tôn chỉ:** Trình điều khiển phần cứng hệ điều hành máy trạm & Giao thức ngoại vi có chủ quyền.

### Nhóm Host OS Infrastructure Drivers
171. **`sandbox/seatbelt/mod.rs`**: macOS Seatbelt (`sandbox-exec`) sinh cấu hình `.sb` khóa chặt quyền ghi trong Worktree.
172. **`sandbox/bubblewrap/mod.rs`**: Linux Bubblewrap (`bwrap`) khởi tạo user namespace cô lập mount/network/process.
173. **`sandbox/developer.rs`**: `SovereignDeveloperAdapter` điều phối thực thi lệnh trong sandbox.
174. **`sandbox/capability_observations.rs`**: Giám sát hành vi I/O phát hiện tác dụng phụ bất thường.
175. **`sandbox/mod.rs`**: Module sandbox root.
176. **`terminal/pty.rs`**: Trình điều khiển Master/Slave PTY ảo thật (POSIX/Windows), xử lý ANSI escape stream và resize.
177. **`terminal/manifest.json`**, `terminal/mod.rs`.
178. **`workspace/provider.rs`**: Thao tác tệp đĩa thật: đọc có giới hạn dung lượng, ghi nguyên tử, duyệt cây thư mục theo `.gitignore`.
179. **`workspace/git_port.rs`**: Cổng gọi CLI Git: `git worktree add/remove`, commit, stash, branch.
180. **`workspace/github.rs`**: Tích hợp GitHub CLI (`gh auth status`), bóc tách device code login.
181. **`workspace/manifest.json`**, `workspace/mod.rs`.
182. **`harness/adapter.rs`**: `ClaudeCodeHarnessAdapter` bọc các CLI agent chạy trong PTY.
183. **`harness/catalog.rs`**, `binary.rs`, `registry.rs`, `spec.rs`, `mock.rs`, `observer.rs`, `mod.rs`.
184. **`harness/stream/`** (`claude.rs`, `codex.rs`, `opencode.rs`, `plain.rs`, `intents.rs`, `process.rs`, `usage.rs`, `mod.rs`): Bộ giải mã dòng văn bản terminal bóc tách tool call, token và chi phí.

### Nhóm Sovereign Communication Protocols (`protocols/`)
185. **`protocols/mod.rs`**: Module tập hợp 3 giao thức ngoại vi có chủ quyền: `mcp`, `a2a`, `roaming`.
186. **`protocols/mcp/security.rs`**: Lớp lọc phòng thủ chống **DNS Rebinding** và **Origin Spoofing** cho loopback endpoints.
187. **`protocols/mcp/federation.rs`**: `McpFederationGateway` gom nhiều MCP server về một mối, gắn tiền tố `server__tool`.
188. **`protocols/mcp/adapters/client.rs`**: Client MCP chuẩn 2026-07-28 đầy đủ (`stdio` và `Streamable HTTP`, multi-round `input_required`).
189. **`protocols/mcp/adapters/gateway_wrap.rs`**: Wrapper kiểm tra `ExecutionPermit` trước khi cho phép gọi tool.
190. **`protocols/mcp/builtins/memory/mod.rs`**: MCP Server bộ nhớ tri thức SQLite nhúng sẵn.
191. **`protocols/mcp/builtins/autovisualiser/`** (`mod.rs` [2.265 lines] và 18 tệp template HTML/JS/CSS): MCP Server tạo biểu đồ tự động (Mermaid, D3, Leaflet, Chart.js). *(Hướng RFT: Tách assets JS/CSS ra thư mục riêng)*.
192. **`protocols/mcp/builtins/mcp_server_runner.rs`**, `mcp/manifest.json`, `mcp/mod.rs`, `mcp/adapters/mod.rs`, `mcp/builtins/mod.rs`.
193. **`protocols/a2a/mod.rs`** & **`protocols/a2a/manifest.json`**: Chuẩn A2A v0.3/v1.0: `AgentCard`, `AgentInterface`, `A2aDispatcher` (JSON-RPC 2.0), `A2aClient`.
194. **`protocols/roaming/`** (12 tệp: `node.rs`, `identity.rs`, `peerbook.rs`, `directory.rs`, `handshake.rs`, `relay.rs`, `trust.rs`, `peer_dispatch.rs`, `card.rs`, `frame.rs`, `error.rs`, `mod.rs`): Mạng P2P Iroh đồng bộ workspace phân tán.

### Nhóm Data & Shared Domain Drivers
195. **`capabilities/documents/source_fetch.rs`** & `manifest.json`: Thu thập file văn bản UTF-8 có kiểm soát size.
196. **`capabilities/mod.rs`**: Re-export cho các caller cũ.
197. **`workbenches/research/notebook/driver.rs`** & `manifest.json`: Trình điều khiển kernel Python/Jupyter qua ZeroMQ/IPC. *(Hướng RFT: Đưa về shared execution `adapters/execution/notebook`)*.
198. **`workbenches/research/compute/`** (`colab.rs`, `kaggle.rs`, `rented_gpu.rs`, `checkpoint.rs`, `checkpoint_store.rs`, `manifest.json`, `mod.rs`): Trình điều khiển mục tiêu GPU từ xa. *(Hướng RFT: Đưa về shared execution `adapters/execution/compute`)*.
199. **`workbenches/quant/market/stream.rs`** & `mod.rs`: Driver truyền dữ liệu thị trường trực tiếp. *(Hướng RFT: Chuyển về extension Quant)*.
200. **`workbenches/mod.rs`**, `Cargo.toml`, `src/lib.rs`.

---

## 8. `platform/components` (`custos-packs`) — Pillar 3
> **Tôn chỉ:** Động cơ quản lý vòng đời Extension Host (Lifecycle Engine).

### Nhóm Động Cơ Vòng Đời (Lifecycle Engine)
201. **`src/installer/mod.rs`**: `LocalPackageTransferAdapter` tiếp nhận tarball (giới hạn 150MiB), verify SHA-256 digest so với manifest, giải nén staging và thăng hạng.
202. **`src/inventory/mod.rs`**: `scan_local_packages`, Package Store Interface, quản lý trạng thái `Installed`, `Enabled`, `Disabled`.
203. **`src/migrations/mod.rs`**: `MigrationDispatcher` tự động quét và chạy file SQL migration đi kèm theo extension.
204. **`src/dynamic_registry/mod.rs`**: Re-export registry cũ. *(Hướng RFT: Gỡ bỏ namespace rỗng này)*.
205. **`src/pack.rs`**, `profile.rs`, `registry.rs`, `verifier.rs`, `benchmark.rs`, `Cargo.toml`, `src/lib.rs`.

### Nhóm 4 Built-in Domain Packs (Cần chuyển dời về `workbenches/`)
206. **`src/assistant/`** (`profile.rs`, `sdk.rs`, `fixtures.rs`, `mod.rs`, `skills/draft/payload_lock.rs`, `skills/identity/contact_resolve.rs`, `skills/subagent/browser_subagent_dispatch.rs`, `verifiers/user_acceptance_oracle.rs`): Pack trợ lý cá nhân. *(Hướng RFT: Chuyển về `workbenches/assistant/`)*.
207. **`src/engineering/`** (`profile.rs`, `recipe.rs`, `sdk.rs`, `fixtures.rs`, `mod.rs`, `skills/ast/ast_search.rs`, `skills/git/git_patch.rs`, `skills/test/cargo_test.rs`, `skills/ui/component_clone.rs`, `verifiers/cargo_test_oracle.rs`): Pack lập trình. *(Hướng RFT: Chuyển về `workbenches/coding/`)*.
208. **`src/quant/`** (`roles.rs`, `jobs.rs`, `mod.rs`, `verifiers/deflated_sharpe.rs`, `verifiers/lookahead_bias.rs`, `verifiers/pbo_cscv.rs`, `tests/quant_golden_test.rs`): Pack tài chính định lượng. *(Hướng RFT: Chuyển về `workbenches/quant/`)*.
209. **`src/research/`** (`profile.rs`, `sdk.rs`, `fixtures.rs`, `mod.rs`, `skills/claims/claim_extract.rs`, `skills/verification/doi_verify.rs`, `skills/web/web_extract.rs`, `verifiers/alce.rs`, `verifiers/citation_coverage_oracle.rs`, `import_formats/`, `markdown_export/`): Pack nghiên cứu khoa học. *(Hướng RFT: Chuyển về `workbenches/research/`)*.

---

## 9. TỔNG HỢP LIỆU PHÁP XỬ LÝ (REMEDIATION DIRECTIVES)

| # | Phân Hệ | Vấn Đề Cốt Lõi | Liệu Pháp Điều Trị Triệt Để |
|---|---|---|---|
| **1** | `platform/ai` | `contracts` bị ô nhiễm bởi `formats/`, reqwest, và native inference bật mặc định. | Tách `ai/contracts` thành pure Zero-I/O module. Thiết lập Cargo feature `local-inference` cho phần inference nặng on-device. |
| **2** | `platform/adapters` | `notebook` và `compute` bị đặt nhầm dưới `workbenches/research`. | Chuyển `notebook` và `compute` thành các driver thực thi dùng chung (`adapters/execution/`). Market feed đưa về extension Quant. |
| **3** | `platform/components` | Vừa làm Extension Host vừa chứa code nghiệp vụ của cả 4 domain packs. | Giữ `components` thuần túy làm Extension Host Engine. Chuyển toàn bộ code 4 domain packs về các thư mục tương ứng trong `workbenches/`. Xóa `dynamic_registry`. |
| **4** | `platform/domain` | Chứa nhiều dữ liệu chuyên biệt của Quant, Research, Fleet. | Base domain chỉ giữ Task, Session, Authority, Evidence, Resources, Context. Chuyển Alpha, Factor, PaperClaim về các workbenches. |
| **5** | `platform/runtime` | Tệp `execution.rs` dài 1.358 dòng gánh toàn bộ cognitive loop. | Phân rã `execution.rs` thành 4 sub-modules: `mode_negotiator`, `cognitive_turn`, `tool_dispatcher`, `terminal_evaluator`. Bỏ static singleton trong `admission.rs`. |
| **6** | `platform/core` | 23 tệp contracts đặt lẫn lộn Host Ports và Extension Ports. | Chuẩn hóa mô hình 2-Tier Ports Taxonomy (`contracts::host` vs `contracts::extensions`). |
| **7** | `platform/storage` | Hơn 1.000 dòng SQL nhúng cứng trong Rust. | Tách các file SQL migration ra thư mục riêng và nạp bằng `include_str!`. |
| **8** | `platform/bridge` | Crate trung gian chỉ chứa 3 tác vụ ứng dụng session linkage. | Chuyển `BridgeService` vào `apps/daemon/src/application/sessions/linkage/` và gỡ bỏ crate `custos-bridge`. |
