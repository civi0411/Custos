# Monorepo Topology, Crate Boundaries & Deep Codebase Map

## Three-root review baseline — 14-11-2026

Toàn bộ kiến trúc Custos đã hoàn thành 5 giai đoạn tái cấu trúc SuperAgent & Orchestration Intelligence (OI) theo Master Plan. Hệ thống hiện tổ chức theo mô hình **3-Root Monorepo Topology** chuẩn mực: `platform/` (hạt nhân dùng chung), `workbenches/` (4 bàn làm việc chuyên biệt: Assistant, Coding, Research, Quant), `apps/` (daemon, cli, desktop native host), cùng `sdk/` (giao thức wire và client).

Source baseline được đồng bộ với 20 Cargo workspace packages. Toàn bộ mã nguồn bao gồm **1,085 tệp Rust (195,224 dòng mã)** và **311 tệp Desktop UI (54,642 dòng mã)**, tổng cộng xấp xỉ **250,000 LOC**. Tất cả 5/5 cổng chất lượng tự động (`cargo xtask gate`) vượt qua 100%.

> **Inviolable Core Invariants:**
> 1. **Zero-I/O Domain Invariant:** `platform/domain` (`custos-domain`) giữ sự thuần khiết tuyệt đối — 0 dependencies I/O (không tokio, không rusqlite, không reqwest, không std::fs).
> 2. **Single-Writer SQLite WAL:** `platform/storage` (`custos-persistence`) quản lý kết nối SQLite duy nhất cho tác vụ ghi, ngăn chặn hoàn toàn starvation và starvation recovery.
> 3. **Zero-Mock in Production:** Mọi mock và fake bị cấm triệt để khỏi mã nguồn sản phẩm, được cô lập dưới `#[cfg(any(test, feature = "test-support"))]`.
> 4. **Process Group & Zombie Elimination:** Toàn bộ subprocess thực thi ngoài hệ thống (`platform/adapters/src/harnesses/stream/process.rs`) bắt buộc tạo process group mới (`setpgid(0)`) và bảo đảm hủy sạch bằng SIGKILL toàn nhóm khi drop.
> 5. **Thin Daemon Transport Adapters:** `apps/daemon` chỉ đóng vai trò Sole Composition Root và mỏng lớp chuyển tiếp (thin transport adapter); toàn bộ nghiệp vụ miền tài chính hay khoa học nằm trọn vẹn trong các workbenches tương ứng.

---

## 0. Quy Trình Đồng Bộ Kiến Trúc & Vận Hành Dành Cho AI Coding Agents

### 0.1 Tam Giác Đồng Bộ Tài Liệu (The Documentation Triad)

Mọi sự thật kiến trúc trong Custos bắt buộc phải tồn tại đồng bộ tại 3 tầng:
1. **`Custos.md` (Root Master Specification):** Định nghĩa **WHY** và **WHAT** (Triết lý, 10 Bất biến hệ thống, State Machines, Trust Zones, Hợp đồng giao thức).
2. **`docs/` (Authoritative Topic-Based Engineering Specifications):** Định nghĩa **HOW** (Đặc tả chi tiết từng chủ đề trong `docs/architecture/`, `docs/reference/`, `docs/development/`).
3. **`docs/development/codebase-architecture.md` (Master Physical Catalog):** Định nghĩa **WHERE** (Bản đồ tệp tin vật lý, số dòng mã, struct/trait xuất khẩu, và ma trận sở hữu RACI).

### 0.2 Quy Trình 5 Bước Bắt Buộc Khi Có Quyết Định Kiến Trúc Mới

1. **Bước 1 — Thống nhất kiến trúc:** Đồng thuận với người vận hành về thiết kế, bất biến và ranh giới.
2. **Bước 2 — Cập nhật `Custos.md`:** Cập nhật chương/mục tương ứng trong tài liệu master.
3. **Bước 3 — Cập nhật `docs/` Topic Document:** Viết hoặc cập nhật tài liệu kỹ thuật chuyên đề.
4. **Bước 4 — Cập nhật `codebase-architecture.md`:** Điền vị trí tệp mới, số dòng, struct/trait xuất khẩu vào bảng catalog.
5. **Bước 5 — Triển khai mã nguồn:** Chỉ viết code sau khi hoàn thành 4 bước trên.

### 0.3 Hướng Dẫn Đọc Dành Cho Coding Agents Trước Khi Viết Code

- **Định vị trước khi sửa:** Tra cứu Mục 1–2 để biết code thuộc package nào trong 3-root monorepo.
- **Tuân thủ ranh giới bất biến:** Tuyệt đối không import chéo phá vỡ cấu trúc phân tầng.
- **Giới hạn số dòng tệp:** Mọi tệp `.rs`, `.ts`, `.tsx`, `.css` phải tuân thủ trần $\le 600$ dòng mã theo quy định của `cargo xtask check-file-line-ceiling`.

---

## 1. Bản Đồ Tổng Thể Workspace Monorepo

```text
Custos/
├── Cargo.toml                       # Root Cargo workspace manifest (20 workspace packages)
├── rust-toolchain.toml              # Phiên bản Rust toolchain cố định
├── deny.toml                        # Cấu hình cargo-deny (bảo mật chuỗi cung ứng, license audit)
│
├── platform/                        # [RING 1 & 2 — SHARED PLATFORM KERNEL & SERVICES]
│   ├── domain/                      # Layer 0: Thực thể miền thuần túy (custos-domain: Zero-I/O, no async)
│   ├── core/                        # Layer 1: Task Kernel, Authority Engine, Completion Gate, Port Traits (custos-core)
│   ├── storage/                     # Layer 1: SQLite WAL persistence, partition engine, Outbox, CAS (custos-persistence)
│   ├── ai/                          # Layer 1: ModelPort contracts, pricing catalog, cloud codecs, local inference (custos-provider)
│   ├── runtime/                     # Layer 2: Agent loop/supervision, fast-path, context, environment, OI brain, workflow (custos-runtime)
│   ├── adapters/                    # Layer 2: OS sandbox, CLI agent adapters, process group guard, MCP/A2A/OAuth (custos-adapters)
│   └── components/                  # Layer 3 Base: Pack registry, module activation, migrations, verifiers (custos-packs)
│
├── workbenches/                     # [RING 3 — OPTIONAL VERTICAL WORKBENCHES]
│   ├── assistant/                   # Assistant workbench: drafts, outbox, domain & repository (custos-workbench-assistant)
│   ├── coding/                      # Coding workbench: recipes, skills, AST/workspace verifiers (custos-workbench-coding)
│   ├── research/                    # Research workbench: claims, synthesis, ALCE/citation verifiers, saga rollback (custos-workbench-research)
│   └── quant/                       # Quant workbench: market/backtest/orders, Deflated Sharpe & Lookahead verifiers (custos-workbench-quant)
│
├── sdk/                             # [WIRE PROTOCOL & CLIENT SDK]
│   ├── protocol/                    # Wire DTOs, Local API envelopes/methods/transport trait (custos-protocol)
│   └── client-rust/                 # Rust client SDK over LocalApiClient & ProfileResolver (custos-sdk)
│
├── apps/                            # [EXECUTABLES & COMPOSITION ROOT]
│   ├── daemon/                      # Sole Composition Root, 4-context Local API, thin transport handlers (custos-daemon)
│   ├── cli/                         # Pure-Rust headless CLI over custos-sdk (custos-app-cli, zero Tauri)
│   └── desktop/native/              # Thin Tauri 2 desktop native host (custos-app-desktop)
│
├── ui/                              # Frontend surfaces (`ui/desktop` product React 19 UI; `ui/cli/bin` Node CLI)
├── tests/                           # Workspace test suites (contract, e2e, testkit, desktop canvas smoke)
└── xtask/                           # Automation scripts, boundary checks & 5-gate release runner
```

---

## 2. Ma Trận Phân Tầng & Workspace Crates

| Layer | Product Crate | Người sở hữu (RACI) | Trách nhiệm kiến trúc cốt lõi | Quy tắc phụ thuộc |
|---|---|:---:|---|---|
| **Layer 0** | `platform/domain` (`custos-domain`) | **Vĩ** | Thực thể miền thuần túy (Task, Session, Evidence, Permit, Notebook, OI models). | **Zero-I/O Tuyệt đối**: Không tokio, rusqlite, reqwest, std::fs. |
| **Layer 0.5** | `sdk/protocol` (`custos-protocol`) | **Vinh** | Wire DTOs, Local API envelopes/methods, `ApiTransport` trait, DTOs cho A2A, profile. | Chỉ phụ thuộc `custos-domain` (không phụ thuộc kernel/workbench/runtime). |
| **Layer 1** | `platform/core` (`custos-core`) | **Vĩ** | Task Kernel (`kernel/trusted.rs`), Authority Engine, Completion Gate, Port Traits (`AgentDriver`). | Phụ thuộc độc quyền vào `custos-domain` (cấm `custos-protocol`, `runtime`, `adapters`, `persistence`). |
| | `platform/storage` (`custos-persistence`) | **Trường** | SQLite WAL mode, schema migrations (0001-0041), Outbox, CAS, `attach_partition`/`detach_partition`. | Hiện thực hóa repository traits; single writer SQLite connection. |
| | `platform/ai` (`custos-provider`) | **Vĩ** | `contracts/`, model pricing catalog, cloud codecs (Anthropic, OpenAI, etc.), lazy `LlamaCppBackend`, routing. | `contracts/` cấm import `cloud/`; không `keyring` (vault/OAuth nằm trong adapters). |
| **Layer 2** | `platform/runtime` (`custos-runtime`) | **Vinh + Vĩ** | 6 pillars: `agent/` (SupervisedAgentRunner, 0-Token FastPathFilter), `context/`, `environment/`, `oi/` (T0-T6 Topologies, Pareto Selector), `session/`, `workflow/`. | Không `reqwest`, không `std::process::Command`, không `custos-persistence` production edge. |
| | `platform/adapters` (`custos-adapters`) | **Trường + Vinh** | `harnesses/` (`CliAgentAdapter`, ProcessDropGuard SIGKILL guard), `connectors/` (mcp, a2a, oauth), `host/` (drivers). | Đóng gói tương tác ngoại vi; cấm phụ thuộc `apps/*` hay `runtime`. |
| **Layer 3** | `platform/components` & `workbenches/*` | **Vĩ** | `custos-packs` flattened và 4 specialist workbenches (`assistant`, `coding`, `research`, `quant`). | Không `reqwest`; phụ thuộc `core`/`runtime`/`adapters`; độc lập phân phối. |
| **Layer 4** | `apps/daemon` (`custos-daemon`) | **Vĩ** | **Sole Composition Root**: `src/{api, auth, application, bootstrap}` với 4 sub-contexts (`kernel`, `env`, `providers`, `workbenches`). | Thin transport handlers; không chứa business logic hay domain math; uỷ quyền drivers cho adapters. |
| | `sdk/client-rust` (`custos-sdk`) | **Vinh** | Client transports (`LocalApiClient`, `ProcessTransport`, `TcpTransport`). | Phụ thuộc `custos-protocol` + `custos-domain` (cấm `custos-core` và `workbenches`). |
| | `apps/{cli,desktop}` (`custos-app-*`) | **Vinh** | `apps/cli` (headless pure-Rust CLI qua `LocalApiClient`) và `apps/desktop/native` (Tauri client). | Giao tiếp qua `custos-sdk` / Local API; `apps/cli` không phụ thuộc `tauri`. |
| **Layer 5** | `ui/desktop` | **Vinh** | Desktop React 19 UI theo Graphite JetBlack theme, 4 workbenches (`copilot`, `engineering`, `research`, `quant`). | Pure client; Zero-mock; tương tác qua daemon client; vượt qua `canvas-smoke.mjs`. |

---

## 3. Bản Đồ Chi Tiết Từng Crate & Từng File Mã Nguồn (Deep File Catalog)

> **Live Physical Catalog:** Bảng danh mục dưới đây phản ánh 100% chính xác cấu trúc tệp tin vật lý hiện hành của toàn bộ workspace monorepo. Mọi đường dẫn tệp đều có liên kết tương đối trực tiếp đến mã nguồn thực tế.

### 3.1. Crate `custos-domain` — Layer 0: Pure Domain Core

- **Đường dẫn thư mục:** `platform/domain`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** Bất biến Zero-I/O tuyệt đối. Không tokio, rusqlite, reqwest, std::fs.
- **Tổng số tệp:** 94 tệp (93 tệp Rust) | **Tổng số dòng mã:** 13,369 dòng Rust.
- **Mô tả chức năng:** Trái tim của hệ thống: Chứa toàn bộ thực thể thuần khiết, định danh (TaskId, SessionId), các bất biến miền, máy trạng thái tác vụ thuần túy, sự kiện và mô hình OI.

#### Danh mục các file bên trong `platform/domain`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/domain/Cargo.toml`](../../platform/domain/Cargo.toml) | 16 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/domain/src/lib.rs`](../../platform/domain/src/lib.rs) | 201 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/domain/src/pipeline/mod.rs`](../../platform/domain/src/pipeline/mod.rs) | 134 | Submodule root: aggregates and re-exports sibling modules | `PipelineStage, DataModality, DataInvariant, PipelineRecord, new_bronze, promote_to_silver` |
| [`platform/domain/src/pipeline/structure.rs`](../../platform/domain/src/pipeline/structure.rs) | 306 | Core architectural component supporting module responsibilities | `InferredDataType, InferredColumn, DetectedStructure, TabularStructureDetector, TabularContractInvariant, detect_delimiter` |
| [`platform/domain/src/context/annotation.rs`](../../platform/domain/src/context/annotation.rs) | 152 | Zero-I/O domain context models, token budgets and context packs | `AnnotationTarget, AnnotationStatus, AnnotationRecord, new_text, new_image_pin, with_side_chat` |
| [`platform/domain/src/context/context.rs`](../../platform/domain/src/context/context.rs) | 216 | Zero-I/O domain context models, token budgets and context packs | `ContextItem, ContextPack, OmittedContextReason, OmittedContextRef, ContextDeliveryState, ContextReceipt` |
| [`platform/domain/src/context/memory.rs`](../../platform/domain/src/context/memory.rs) | 83 | Zero-I/O domain context models, token budgets and context packs | `RecallQuery, MemoryScope, MemoryEntry, PersonalFact, FactProposal, ProposalStatus` |
| [`platform/domain/src/context/mod.rs`](../../platform/domain/src/context/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/evidence/evidence.rs`](../../platform/domain/src/evidence/evidence.rs) | 351 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `EvidenceStatus, EvidenceRequirement, EvidenceRecord, VerificationClaim, CriterionVerificationRecord, OutcomeStatus` |
| [`platform/domain/src/evidence/execution_record.rs`](../../platform/domain/src/evidence/execution_record.rs) | 365 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `ExecutionRecordStatus, ExecutionRecord, CheckpointResumeAssessment, ExecutionArtifact, assess_checkpoint_resume, new` |
| [`platform/domain/src/evidence/fact.rs`](../../platform/domain/src/evidence/fact.rs) | 13 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `Fact` |
| [`platform/domain/src/evidence/mod.rs`](../../platform/domain/src/evidence/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/evidence/review.rs`](../../platform/domain/src/evidence/review.rs) | 180 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `ReviewTargetType, ReviewMethod, ReviewStatus, FindingSeverity, ReviewFinding, ReviewerRecord` |
| [`platform/domain/src/evidence/experiment_attempt/attempt.rs`](../../platform/domain/src/evidence/experiment_attempt/attempt.rs) | 215 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `ExternalJobRef, ExecState, ResourceState, ExecEvent, ResourceEvent, ExecutionAttempt` |
| [`platform/domain/src/evidence/experiment_attempt/mod.rs`](../../platform/domain/src/evidence/experiment_attempt/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/evidence/experiment_attempt/protection.rs`](../../platform/domain/src/evidence/experiment_attempt/protection.rs) | 98 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `InvocationOutcome, Invocation, ProgressTriplet, ProtectionPolicy, ProtectionStatus, validate` |
| [`platform/domain/src/evidence/experiment_attempt/tests.rs`](../../platform/domain/src/evidence/experiment_attempt/tests.rs) | 318 | Zero-I/O evidence models, facts, extractions, claims and verifier results | None |
| [`platform/domain/src/evidence/experiment_attempt/verdict.rs`](../../platform/domain/src/evidence/experiment_attempt/verdict.rs) | 148 | Zero-I/O evidence models, facts, extractions, claims and verifier results | `CheckGroup, CheckStatus, Check, VerificationReport, TargetCaps, ResumeVerdict` |
| [`platform/domain/src/workspace/artifact.rs`](../../platform/domain/src/workspace/artifact.rs) | 394 | Core architectural component supporting module responsibilities | `ArtifactKind, ArtifactRef, UniversalResourceRef, HandoffConsent, ArtifactHandoff, ArtifactLineageNode` |
| [`platform/domain/src/workspace/mod.rs`](../../platform/domain/src/workspace/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/workspace/notes.rs`](../../platform/domain/src/workspace/notes.rs) | 122 | Core architectural component supporting module responsibilities | `NoteRecord, NoteVersionRecord, new, update` |
| [`platform/domain/src/workspace/workspace.rs`](../../platform/domain/src/workspace/workspace.rs) | 509 | Core architectural component supporting module responsibilities | `WorkspaceId, WorkspaceKind, WorkspaceStatus, WorkspaceLineage, DirtyManifest, ExecutionWorkspace` |
| [`platform/domain/src/workspace/workspace_files.rs`](../../platform/domain/src/workspace/workspace_files.rs) | 295 | Core architectural component supporting module responsibilities | `WorkspaceFileEntry, WorkspaceFileTree, WorkspaceFileContent, WriteWorkspaceFileParams, WorkspaceDiffEntry, WorkspaceDiffSummary` |
| [`platform/domain/src/workspace/repo/mod.rs`](../../platform/domain/src/workspace/repo/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/workspace/repo/span.rs`](../../platform/domain/src/workspace/repo/span.rs) | 50 | Core architectural component supporting module responsibilities | `SourceSpan, new, line_count` |
| [`platform/domain/src/provider/config.rs`](../../platform/domain/src/provider/config.rs) | 264 | Core architectural component supporting module responsibilities | `ProviderConfig, ClientApiKeyRecord, OAuthTokenRecord, ProbedModel, ModelPricing, ModelCatalogOption` |
| [`platform/domain/src/provider/mod.rs`](../../platform/domain/src/provider/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/provider/observation.rs`](../../platform/domain/src/provider/observation.rs) | 86 | Core architectural component supporting module responsibilities | `ProviderHealthStatus, ProviderObservationSnapshot, new, with_model, with_status, is_operable` |
| [`platform/domain/src/shared/error.rs`](../../platform/domain/src/shared/error.rs) | 33 | Zero-I/O shared types, entity IDs, timestamps and domain errors | `DomainError` |
| [`platform/domain/src/shared/ids.rs`](../../platform/domain/src/shared/ids.rs) | 284 | Zero-I/O shared types, entity IDs, timestamps and domain errors | `WorkflowNodeId, PaneId, JobId, HarnessId, new_id, new` |
| [`platform/domain/src/shared/mod.rs`](../../platform/domain/src/shared/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/shared/types.rs`](../../platform/domain/src/shared/types.rs) | 12 | Zero-I/O shared types, entity IDs, timestamps and domain errors | None |
| [`platform/domain/src/packages/mod.rs`](../../platform/domain/src/packages/mod.rs) | 182 | Submodule root: aggregates and re-exports sibling modules | `ComponentId, EngineReadiness, ComponentEnablement, DistributionPreset, PackageState, new` |
| [`platform/domain/src/task/continuation.rs`](../../platform/domain/src/task/continuation.rs) | 290 | Zero-I/O task state machine, runs, workers, continuations and revisions | `ContinuationPacket, ContinuationDisplayScope, RequestedModelScope, TransferPrivacyDecision, ContinuationState, ContinuationManifest` |
| [`platform/domain/src/task/mod.rs`](../../platform/domain/src/task/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/task/recipe.rs`](../../platform/domain/src/task/recipe.rs) | 94 | Zero-I/O task state machine, runs, workers, continuations and revisions | `Recipe, RecipeEnvironmentSpec, RecipeInput, new, with_description, with_input` |
| [`platform/domain/src/task/span.rs`](../../platform/domain/src/task/span.rs) | 90 | Zero-I/O task state machine, runs, workers, continuations and revisions | `SpanState, Span, new, complete, fail` |
| [`platform/domain/src/task/task.rs`](../../platform/domain/src/task/task.rs) | 309 | Zero-I/O task state machine, runs, workers, continuations and revisions | `EvidenceKind, ContractEvidence, CriterionSpec, TaskContractRevision, TaskContract, TaskStatus` |
| [`platform/domain/src/task/workflow.rs`](../../platform/domain/src/task/workflow.rs) | 298 | Zero-I/O task state machine, runs, workers, continuations and revisions | `WorkflowStep, WorkflowPlan, WorkflowIR, WorkflowNodeIR, WorkflowRevision, RevisionNode` |
| [`platform/domain/src/task/run/claim.rs`](../../platform/domain/src/task/run/claim.rs) | 84 | Zero-I/O task state machine, runs, workers, continuations and revisions | `ClaimStatus, DispatchClaim, new, with_node_id, mark_dispatched, release` |
| [`platform/domain/src/task/run/command.rs`](../../platform/domain/src/task/run/command.rs) | 153 | Zero-I/O task state machine, runs, workers, continuations and revisions | `StartRunCommand, RunHandle, CancelReceipt, new, with_node_id, with_workflow_revision` |
| [`platform/domain/src/task/run/finding.rs`](../../platform/domain/src/task/run/finding.rs) | 154 | Zero-I/O task state machine, runs, workers, continuations and revisions | `FindingKind, SupervisionSeverity, SupervisionFinding, ErrorGroupSummary, parse, compute_error_signature` |
| [`platform/domain/src/task/run/launch.rs`](../../platform/domain/src/task/run/launch.rs) | 133 | Zero-I/O task state machine, runs, workers, continuations and revisions | `LaunchStatus, LaunchAttempt, allows_retry, new, mark_launched, mark_refused` |
| [`platform/domain/src/task/run/live.rs`](../../platform/domain/src/task/run/live.rs) | 42 | Zero-I/O task state machine, runs, workers, continuations and revisions | `LiveRunEvent, LiveRunSnapshot, LiveRunReplay, ReconciliationSummary` |
| [`platform/domain/src/task/run/mod.rs`](../../platform/domain/src/task/run/mod.rs) | 28 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/task/run/step_span.rs`](../../platform/domain/src/task/run/step_span.rs) | 251 | Zero-I/O task state machine, runs, workers, continuations and revisions | `StepSpanKind, StepSpanStatus, LogSeverity, RunStepSpan, StepError, SupervisionLogEntry` |
| [`platform/domain/src/task/run/tests.rs`](../../platform/domain/src/task/run/tests.rs) | 213 | Zero-I/O task state machine, runs, workers, continuations and revisions | None |
| [`platform/domain/src/task/run/types.rs`](../../platform/domain/src/task/run/types.rs) | 215 | Zero-I/O task state machine, runs, workers, continuations and revisions | `RunStatus, Run, WorkerRun, NodeAttempt, is_terminal, can_transition_to` |
| [`platform/domain/src/task/run/usage.rs`](../../platform/domain/src/task/run/usage.rs) | 101 | Zero-I/O task state machine, runs, workers, continuations and revisions | `UsageMeasurement, UsageExecutorKind, UsageRecord, new, with_model_identity` |
| [`platform/domain/src/task/run/watchdog.rs`](../../platform/domain/src/task/run/watchdog.rs) | 524 | Zero-I/O task state machine, runs, workers, continuations and revisions | `ThresholdPolicy, SupervisionProfile, SupervisionSnapshot, SupervisionRule, LongRunningRule, RepeatedToolRule` |
| [`platform/domain/src/environment/assurance.rs`](../../platform/domain/src/environment/assurance.rs) | 25 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `EnvironmentAssurance, unconstrained` |
| [`platform/domain/src/environment/browser.rs`](../../platform/domain/src/environment/browser.rs) | 254 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `BrowserTabStatus, BrowserConsoleEntry, BrowserNetworkRequest, BrowserPageSnapshot, BrowserTab, BrowserSession` |
| [`platform/domain/src/environment/harness.rs`](../../platform/domain/src/environment/harness.rs) | 170 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `ToolMediationLevel, WorktreeOwnership, CostVisibility, HarnessProfile, HarnessExecutionResult, HarnessMaturity` |
| [`platform/domain/src/environment/mod.rs`](../../platform/domain/src/environment/mod.rs) | 25 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/environment/notebook.rs`](../../platform/domain/src/environment/notebook.rs) | 191 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `NotebookCellType, CellExecutionStatus, KernelStatus, NotebookCell, NotebookDocument, NotebookKernelBinding` |
| [`platform/domain/src/environment/profile.rs`](../../platform/domain/src/environment/profile.rs) | 44 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `EnvironmentProfile, requires_worktree, allows_live_network` |
| [`platform/domain/src/environment/snapshot.rs`](../../platform/domain/src/environment/snapshot.rs) | 30 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `EnvSnapshot, new` |
| [`platform/domain/src/environment/spec.rs`](../../platform/domain/src/environment/spec.rs) | 28 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `EnvironmentSpec, new` |
| [`platform/domain/src/environment/state.rs`](../../platform/domain/src/environment/state.rs) | 33 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `EnvironmentState, is_ready, can_transition_to` |
| [`platform/domain/src/environment/terminal.rs`](../../platform/domain/src/environment/terminal.rs) | 103 | Zero-I/O environment entities for notebook, terminal, workspace and compute | `TerminalSessionStatus, TerminalSession, TerminalOutputChunk, is_active` |
| [`platform/domain/src/fleet/lane.rs`](../../platform/domain/src/fleet/lane.rs) | 189 | Zero-I/O fleet management, compute tiers and pod leasing specs | `ExecutionLane, LanePolicy, priority_rank, can_preempt, is_preemptible, as_str` |
| [`platform/domain/src/fleet/mod.rs`](../../platform/domain/src/fleet/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/fleet/remote_fleet.rs`](../../platform/domain/src/fleet/remote_fleet.rs) | 160 | Zero-I/O fleet management, compute tiers and pod leasing specs | `RemoteHostStatus, SshAuthMethod, RemoteHostNode, RegisterHostParams, FleetExecParams, FleetExecReceipt` |
| [`platform/domain/src/fleet/compute/adaptation.rs`](../../platform/domain/src/fleet/compute/adaptation.rs) | 206 | Zero-I/O fleet management, compute tiers and pod leasing specs | `CompatibilityVerdict, CompatibilityGateEvaluation, HardwareAdaptationPlan, evaluate_gate, compute` |
| [`platform/domain/src/fleet/compute/admission.rs`](../../platform/domain/src/fleet/compute/admission.rs) | 144 | Zero-I/O fleet management, compute tiers and pod leasing specs | `ComputeWorkloadKind, OptionalConsumerPlacement, WorkloadLeaseStatus, WorkloadLease, WorkloadAdmissionRequest, WorkloadAdmissionVerdict` |
| [`platform/domain/src/fleet/compute/backend.rs`](../../platform/domain/src/fleet/compute/backend.rs) | 239 | Zero-I/O fleet management, compute tiers and pod leasing specs | `ComputeBackendKind, AcceleratorSpec, CloudPodLease, CloudPodStatus, DeviceKind, ComputeDeviceRef` |
| [`platform/domain/src/fleet/compute/checkpoint.rs`](../../platform/domain/src/fleet/compute/checkpoint.rs) | 254 | Zero-I/O fleet management, compute tiers and pod leasing specs | `ResumabilityLevel, CheckpointCommitStatus, CheckpointFileEntry, TrainingCheckpointManifest, label, allows_state_continuation` |
| [`platform/domain/src/fleet/compute/mod.rs`](../../platform/domain/src/fleet/compute/mod.rs) | 33 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/fleet/compute/training.rs`](../../platform/domain/src/fleet/compute/training.rs) | 176 | Zero-I/O fleet management, compute tiers and pod leasing specs | `FrozenTrainingRecipe, TrainingStepTelemetry, TrainingExecutionReceipt, ModelEvaluationSpec, ModelEvaluationReport, new` |
| [`platform/domain/src/fleet/compute/tests/adaptation_tests.rs`](../../platform/domain/src/fleet/compute/tests/adaptation_tests.rs) | 93 | Zero-I/O fleet management, compute tiers and pod leasing specs | None |
| [`platform/domain/src/fleet/compute/tests/admission_tests.rs`](../../platform/domain/src/fleet/compute/tests/admission_tests.rs) | 44 | Zero-I/O fleet management, compute tiers and pod leasing specs | None |
| [`platform/domain/src/fleet/compute/tests/checkpoint_tests.rs`](../../platform/domain/src/fleet/compute/tests/checkpoint_tests.rs) | 78 | Zero-I/O fleet management, compute tiers and pod leasing specs | None |
| [`platform/domain/src/fleet/compute/tests/mod.rs`](../../platform/domain/src/fleet/compute/tests/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/authority/action.rs`](../../platform/domain/src/authority/action.rs) | 391 | Zero-I/O domain entity for authority, grants, permits and policy rules | `RiskLevel, ActionLifecycleState, Assurance, Action, EffectStatus, EffectAttempt` |
| [`platform/domain/src/authority/approval.rs`](../../platform/domain/src/authority/approval.rs) | 167 | Zero-I/O domain entity for authority, grants, permits and policy rules | `ApprovalStatus, ApprovalRequest, ApprovalDecision, new, approve, reject` |
| [`platform/domain/src/authority/budget.rs`](../../platform/domain/src/authority/budget.rs) | 203 | Zero-I/O domain entity for authority, grants, permits and policy rules | `Headroom, ReservationToken, Budget, new, reserve, settle` |
| [`platform/domain/src/authority/capability.rs`](../../platform/domain/src/authority/capability.rs) | 209 | Zero-I/O domain entity for authority, grants, permits and policy rules | `CapabilityManifest, CapabilityStatus, CapabilityGroup, CapabilityDescriptor, ResourceActionCapability, is_available` |
| [`platform/domain/src/authority/mod.rs`](../../platform/domain/src/authority/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/authority/outbox.rs`](../../platform/domain/src/authority/outbox.rs) | 97 | Zero-I/O domain entity for authority, grants, permits and policy rules | `OutboxDeliveryState, OutboxActionItem, delivery_state, mark_uncertain` |
| [`platform/domain/src/authority/permit.rs`](../../platform/domain/src/authority/permit.rs) | 292 | Zero-I/O domain entity for authority, grants, permits and policy rules | `Grant, Permit, ReceiptStatus, ExecutionReceipt, StandingGrant, new` |
| [`platform/domain/src/oi/candidate.rs`](../../platform/domain/src/oi/candidate.rs) | 50 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `Candidate, RejectionReason` |
| [`platform/domain/src/oi/evaluation.rs`](../../platform/domain/src/oi/evaluation.rs) | 335 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `TaskExecutionMetrics, ComparisonMarginConfig, OptimizationClaimOutcome, OptimizationEvaluationReport, OptimizationEvaluator, total_economic_cost` |
| [`platform/domain/src/oi/explain.rs`](../../platform/domain/src/oi/explain.rs) | 41 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `CandidateExplanation, ExplainReport` |
| [`platform/domain/src/oi/mod.rs`](../../platform/domain/src/oi/mod.rs) | 29 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/oi/placement.rs`](../../platform/domain/src/oi/placement.rs) | 29 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `NodePlacement, new` |
| [`platform/domain/src/oi/proposal.rs`](../../platform/domain/src/oi/proposal.rs) | 38 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `StrategyProposal, native_baseline` |
| [`platform/domain/src/oi/record.rs`](../../platform/domain/src/oi/record.rs) | 52 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `DecisionRecord, new` |
| [`platform/domain/src/oi/replan.rs`](../../platform/domain/src/oi/replan.rs) | 87 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `Assumption, ReplanTrigger, ReplanBrief, ReplanRecord, new` |
| [`platform/domain/src/oi/snapshot.rs`](../../platform/domain/src/oi/snapshot.rs) | 58 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `DecisionSnapshot, new` |
| [`platform/domain/src/oi/topology.rs`](../../platform/domain/src/oi/topology.rs) | 83 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `ExecutionTopology, is_parallel, requires_isolation` |
| [`platform/domain/src/oi/work_packet.rs`](../../platform/domain/src/oi/work_packet.rs) | 99 | Zero-I/O orchestration intelligence models, topologies, snapshots and candidate proposals | `WorkPacket, WorkerStatus, WorkerResult, new, success, failed` |
| [`platform/domain/src/session/automation.rs`](../../platform/domain/src/session/automation.rs) | 141 | Zero-I/O conversation session, turns and workbench bindings | `HeadlessJobStatus, HeadlessTrigger, HeadlessTaskSpec, HeadlessAutomationJob, CreateHeadlessJobParams, RunHeadlessJobResult` |
| [`platform/domain/src/session/mod.rs`](../../platform/domain/src/session/mod.rs) | 6 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/domain/src/session/session.rs`](../../platform/domain/src/session/session.rs) | 346 | Zero-I/O conversation session, turns and workbench bindings | `SessionId, SessionMode, SessionStatus, Session, SessionJournalEntry, WorkbenchId` |

### 3.2. Crate `custos-core` — Layer 1: Kernel & Invariants

- **Đường dẫn thư mục:** `platform/core`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** Logic nghiệp vụ thuần túy; phụ thuộc độc quyền vào custos-domain (cấm custos-protocol, workbenches, runtime).
- **Tổng số tệp:** 52 tệp (51 tệp Rust) | **Tổng số dòng mã:** 6,830 dòng Rust.
- **Mô tả chức năng:** Bộ hạt nhân điều phối: Task Kernel (trusted.rs), Authority Engine, Completion Gate thẩm định bằng chứng, và Port Traits (AgentDriver, WorkflowPort, ModelPort).

#### Danh mục các file bên trong `platform/core`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/core/Cargo.toml`](../../platform/core/Cargo.toml) | 25 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/core/src/lib.rs`](../../platform/core/src/lib.rs) | 16 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/core/src/evidence/bundle.rs`](../../platform/core/src/evidence/bundle.rs) | 31 | Completion gate, verifier registry, and evidence assessment pipeline | `EvidenceBundle, new` |
| [`platform/core/src/evidence/mod.rs`](../../platform/core/src/evidence/mod.rs) | 341 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/core/src/evidence/pipeline.rs`](../../platform/core/src/evidence/pipeline.rs) | 122 | Completion gate, verifier registry, and evidence assessment pipeline | `RequirementEvaluation, EvidencePipeline, new, with_standard_verifiers, with_source_acquisition, register_verifier` |
| [`platform/core/src/evidence/verifier.rs`](../../platform/core/src/evidence/verifier.rs) | 470 | Completion gate, verifier registry, and evidence assessment pipeline | `Verifier, CommandExitCodeVerifier, HashVerifier, ExactMatchVerifier, CitationVerifier, SemanticSupportEvaluator` |
| [`platform/core/src/contracts/agent.rs`](../../platform/core/src/contracts/agent.rs) | 15 | Canonical Agent contract: defines AgentDriver trait re-exporting AgentRuntimePort, deprecating legacy untyped harness | None |
| [`platform/core/src/contracts/checkpoint_store.rs`](../../platform/core/src/contracts/checkpoint_store.rs) | 87 | Architectural contract port trait defining system abstraction boundaries | `StoreCapabilities, BlobMetadata, PromotionOutcome, CheckpointStorePort` |
| [`platform/core/src/contracts/compute.rs`](../../platform/core/src/contracts/compute.rs) | 307 | Architectural contract port trait defining system abstraction boundaries | `CloudPodRequest, ComputeTelemetry, ComputePort, WorkloadAdmissionPort, TargetCapabilities, PreflightReport` |
| [`platform/core/src/contracts/credentials.rs`](../../platform/core/src/contracts/credentials.rs) | 90 | Architectural contract port trait defining system abstraction boundaries | `CredentialLocation, CredentialVaultError, CredentialVault, CredentialMetadata, CredentialOperation, credential_part_count` |
| [`platform/core/src/contracts/environment.rs`](../../platform/core/src/contracts/environment.rs) | 77 | Architectural contract port trait defining system abstraction boundaries | `GateReport, EnvironmentHandle, EnvironmentPort, ok, failed` |
| [`platform/core/src/contracts/harness.rs`](../../platform/core/src/contracts/harness.rs) | 90 | Architectural contract port trait defining system abstraction boundaries | `AgentRuntimePort, RunObserver` |
| [`platform/core/src/contracts/judgment.rs`](../../platform/core/src/contracts/judgment.rs) | 42 | Architectural contract port trait defining system abstraction boundaries | `JudgmentRequest, JudgmentResult, JudgmentPort` |
| [`platform/core/src/contracts/kernel.rs`](../../platform/core/src/contracts/kernel.rs) | 111 | Architectural contract port trait defining system abstraction boundaries | `KernelPort` |
| [`platform/core/src/contracts/memory.rs`](../../platform/core/src/contracts/memory.rs) | 33 | Architectural contract port trait defining system abstraction boundaries | `MemoryPort` |
| [`platform/core/src/contracts/mod.rs`](../../platform/core/src/contracts/mod.rs) | 114 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/core/src/contracts/notebook.rs`](../../platform/core/src/contracts/notebook.rs) | 125 | Architectural contract port trait defining system abstraction boundaries | `NotebookWorkerResponse, NotebookWorkerSession, NotebookDriverPort, NotebookStoragePort` |
| [`platform/core/src/contracts/oi.rs`](../../platform/core/src/contracts/oi.rs) | 14 | Architectural contract port trait defining system abstraction boundaries | `OiPlannerPort` |
| [`platform/core/src/contracts/pack.rs`](../../platform/core/src/contracts/pack.rs) | 313 | Architectural contract port trait defining system abstraction boundaries | `WorkerRole, WorkbenchJob, DomainPackManifest, PackProfile, VerificationResult, VerificationContext` |
| [`platform/core/src/contracts/packages.rs`](../../platform/core/src/contracts/packages.rs) | 89 | Architectural contract port trait defining system abstraction boundaries | `DownloadProgress, StagedArtifact, InstalledPath, PackageDistributionEnvelope, PackageError, PackageTransferPort` |
| [`platform/core/src/contracts/sandbox.rs`](../../platform/core/src/contracts/sandbox.rs) | 108 | Architectural contract port trait defining system abstraction boundaries | `SandboxCommand, SandboxPort, CommandExecutionResult, CommandRunnerPort, from_intent, verify_permit_binding` |
| [`platform/core/src/contracts/skill.rs`](../../platform/core/src/contracts/skill.rs) | 259 | Architectural contract port trait defining system abstraction boundaries | `TaintLevel, WriteTarget, SkillCapabilityProfile, WorkspaceScope, SkillContext, SkillOutput` |
| [`platform/core/src/contracts/source_acquisition.rs`](../../platform/core/src/contracts/source_acquisition.rs) | 17 | Architectural contract port trait defining system abstraction boundaries | `SourceAcquisitionPort` |
| [`platform/core/src/contracts/storage.rs`](../../platform/core/src/contracts/storage.rs) | 325 | Architectural contract port trait defining system abstraction boundaries | `StoragePort, RunStore, WorkflowStore, RunPort, DecisionPort, WorkflowRevisionPort` |
| [`platform/core/src/contracts/terminal.rs`](../../platform/core/src/contracts/terminal.rs) | 94 | Architectural contract port trait defining system abstraction boundaries | `TerminalPort, TerminalProcessHandle, TerminalOutputConsumer, TerminalDriverPort` |
| [`platform/core/src/contracts/tool_gate.rs`](../../platform/core/src/contracts/tool_gate.rs) | 24 | Architectural contract port trait defining system abstraction boundaries | `ExecutionResult, ToolGate` |
| [`platform/core/src/contracts/workflow.rs`](../../platform/core/src/contracts/workflow.rs) | 191 | Architectural contract port trait defining system abstraction boundaries | `WorkflowPort, SupervisionTraceSink, NoopSupervisionSink, SupervisionOverview, SupervisionRunFilter, SupervisionLogFilter` |
| [`platform/core/src/contracts/workspace.rs`](../../platform/core/src/contracts/workspace.rs) | 104 | Architectural contract port trait defining system abstraction boundaries | `WorkspaceRepository, WorkspaceProvider, RepositoryGitPort` |
| [`platform/core/src/contracts/workspace_files.rs`](../../platform/core/src/contracts/workspace_files.rs) | 79 | Architectural contract port trait defining system abstraction boundaries | `WorkspaceFilesPort` |
| [`platform/core/src/authority/approvals.rs`](../../platform/core/src/authority/approvals.rs) | 51 | Authority engine: grants verification, permit issuance and taint tracking | `ApprovalManager, new, request_approval, get, list_pending, decide` |
| [`platform/core/src/authority/audit.rs`](../../platform/core/src/authority/audit.rs) | 82 | Authority engine: grants verification, permit issuance and taint tracking | `AuditEntry, AuditLog, new, record, len, is_empty` |
| [`platform/core/src/authority/environment_gate.rs`](../../platform/core/src/authority/environment_gate.rs) | 194 | Authority engine: grants verification, permit issuance and taint tracking | `EnvironmentGate, evaluate` |
| [`platform/core/src/authority/grants.rs`](../../platform/core/src/authority/grants.rs) | 89 | Authority engine: grants verification, permit issuance and taint tracking | `GrantStore, new, insert, get, find_active_grant, find_grant_for_action` |
| [`platform/core/src/authority/mod.rs`](../../platform/core/src/authority/mod.rs) | 343 | Submodule root: aggregates and re-exports sibling modules | `AuthorityEngine, new, authorize_action` |
| [`platform/core/src/authority/path_sandbox.rs`](../../platform/core/src/authority/path_sandbox.rs) | 141 | Authority engine: grants verification, permit issuance and taint tracking | `PathSandbox, new, workspace_root, validate_with_observations, resolve_and_contain` |
| [`platform/core/src/authority/permits.rs`](../../platform/core/src/authority/permits.rs) | 249 | Authority engine: grants verification, permit issuance and taint tracking | `PermitStore, PermitIssuer, new, with_store, issue_permit, issue_permit_with_digest` |
| [`platform/core/src/authority/policy.rs`](../../platform/core/src/authority/policy.rs) | 124 | Authority engine: grants verification, permit issuance and taint tracking | `ApprovalKind, ApprovalPolicyVerdict, ApprovalPolicy, PolicyDecision, PolicyEvaluator, DefaultPolicyEvaluator` |
| [`platform/core/src/authority/risk.rs`](../../platform/core/src/authority/risk.rs) | 39 | Authority engine: grants verification, permit issuance and taint tracking | `RiskEvaluator, classify_action, requires_human_approval` |
| [`platform/core/src/kernel/budget.rs`](../../platform/core/src/kernel/budget.rs) | 136 | Task kernel lifecycle management, state transitions and admission control | `BudgetGovernor, new, reserve, settle, refund, headroom` |
| [`platform/core/src/kernel/commands.rs`](../../platform/core/src/kernel/commands.rs) | 65 | Task kernel lifecycle management, state transitions and admission control | `CreateTask, AdvanceTask, BlockTask, CompleteTask, CancelTask, RetrySpec` |
| [`platform/core/src/kernel/completion.rs`](../../platform/core/src/kernel/completion.rs) | 465 | Task kernel lifecycle management, state transitions and admission control | `CompletionGate, can_complete, can_complete_with_evidence, is_verifier_compatible_with_kind, can_cancel` |
| [`platform/core/src/kernel/events.rs`](../../platform/core/src/kernel/events.rs) | 110 | Task kernel lifecycle management, state transitions and admission control | `TaskCreated, TaskAdvanced, TaskBlocked, TaskCompleted, TaskCancelled, TaskSnapshotImported` |
| [`platform/core/src/kernel/invariants.rs`](../../platform/core/src/kernel/invariants.rs) | 56 | Task kernel lifecycle management, state transitions and admission control | `TaskInvariants, assert_valid_title, assert_epoch, assert_not_terminal, assert_valid_transition` |
| [`platform/core/src/kernel/mod.rs`](../../platform/core/src/kernel/mod.rs) | 32 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/core/src/kernel/observability.rs`](../../platform/core/src/kernel/observability.rs) | 22 | Task kernel lifecycle management, state transitions and admission control | `KernelObservability, on_state_change` |
| [`platform/core/src/kernel/ports.rs`](../../platform/core/src/kernel/ports.rs) | 45 | Task kernel lifecycle management, state transitions and admission control | `TaskStore, SessionStore` |
| [`platform/core/src/kernel/reducer.rs`](../../platform/core/src/kernel/reducer.rs) | 195 | Task kernel lifecycle management, state transitions and admission control | `TaskReducer, from_created, replay, apply` |
| [`platform/core/src/kernel/service.rs`](../../platform/core/src/kernel/service.rs) | 387 | Task kernel lifecycle management, state transitions and admission control | `TaskService, new, execute_create, execute_advance, execute_block, execute_complete` |
| [`platform/core/src/kernel/session_state_machine.rs`](../../platform/core/src/kernel/session_state_machine.rs) | 40 | Task kernel lifecycle management, state transitions and admission control | `SessionStateMachine, transition` |
| [`platform/core/src/kernel/span_service.rs`](../../platform/core/src/kernel/span_service.rs) | 60 | Task kernel lifecycle management, state transitions and admission control | `SpanService, new, start_span, complete_span, record_continuation` |
| [`platform/core/src/kernel/state_machine.rs`](../../platform/core/src/kernel/state_machine.rs) | 47 | Task kernel lifecycle management, state transitions and admission control | `TaskStateMachine, valid_next_states, can_transition, ensure_can_transition` |
| [`platform/core/src/kernel/trusted.rs`](../../platform/core/src/kernel/trusted.rs) | 170 | TrustedKernel: core coordinator of Task state machine, authority engine, and evidence completion gate | `TrustedKernel, new, with_source_acquisition, with_parts, authority, evidence_mut` |

### 3.3. Crate `custos-persistence` — Layer 1: Relational Storage & Partition Engine

- **Đường dẫn thư mục:** `platform/storage`
- **Chủ sở hữu chính (Owner):** **Trường**
- **Quy tắc ranh giới:** SQLite WAL mode, single-writer connection, chống WAL starvation. Phân vùng ATTACH/DETACH an toàn chống SQL injection.
- **Tổng số tệp:** 58 tệp (57 tệp Rust) | **Tổng số dòng mã:** 11,788 dòng Rust.
- **Mô tả chức năng:** Lưu trữ quan hệ bền vững: Quản lý SQLite WAL, schema migrations (0001-0041), Transactional Outbox, CAS versioning và cơ chế attach/detach multi-partition database.

#### Danh mục các file bên trong `platform/storage`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/storage/Cargo.toml`](../../platform/storage/Cargo.toml) | 20 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/storage/src/connection.rs`](../../platform/storage/src/connection.rs) | 346 | DbConnection with attach_partition & detach_partition SQL injection guarded SQLite multi-database engine | `DbConnection, ReaderPool, PooledReader, open_in_memory, open, lock` |
| [`platform/storage/src/lib.rs`](../../platform/storage/src/lib.rs) | 11 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/storage/src/migrations.rs`](../../platform/storage/src/migrations.rs) | 141 | Core architectural component supporting module responsibilities | `run_migrations` |
| [`platform/storage/src/artifacts/filesystem.rs`](../../platform/storage/src/artifacts/filesystem.rs) | 189 | CAS storage and content-addressed artifact repository | `FsArtifactStore, new` |
| [`platform/storage/src/artifacts/mod.rs`](../../platform/storage/src/artifacts/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/storage/src/repositories/browser.rs`](../../platform/storage/src/repositories/browser.rs) | 353 | Relational repository implementing transactional persistence via SQLite | `BrowserRepository, new, save_browser_session, get_browser_session, list_browser_sessions, save_browser_tab` |
| [`platform/storage/src/repositories/command_ledger.rs`](../../platform/storage/src/repositories/command_ledger.rs) | 381 | Relational repository implementing transactional persistence via SQLite | `CommandLedgerRepository, new, get_command, record_command, admit, complete` |
| [`platform/storage/src/repositories/continuation.rs`](../../platform/storage/src/repositories/continuation.rs) | 124 | Relational repository implementing transactional persistence via SQLite | `ContinuationRepository, new, save_continuation, get_latest_continuation, get_continuation` |
| [`platform/storage/src/repositories/decision.rs`](../../platform/storage/src/repositories/decision.rs) | 110 | Relational repository implementing transactional persistence via SQLite | `DecisionRepository, new, save_decision, get_decision, list_decisions_for_task` |
| [`platform/storage/src/repositories/experiment_attempt.rs`](../../platform/storage/src/repositories/experiment_attempt.rs) | 418 | Relational repository implementing transactional persistence via SQLite | `ExperimentAttemptRepository, new, save_attempt, get_attempt, list_attempts_for_experiment, record_invocation` |
| [`platform/storage/src/repositories/mod.rs`](../../platform/storage/src/repositories/mod.rs) | 35 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/storage/src/repositories/packages.rs`](../../platform/storage/src/repositories/packages.rs) | 524 | Relational repository implementing transactional persistence via SQLite | `PackageInventoryRow, PackageRepository, new, list_inventory, list_enabled_components, record_activation_intent` |
| [`platform/storage/src/repositories/replan.rs`](../../platform/storage/src/repositories/replan.rs) | 160 | Relational repository implementing transactional persistence via SQLite | `ReplanRepository, new, save_replan, get_replan, list_replans_for_task` |
| [`platform/storage/src/repositories/session.rs`](../../platform/storage/src/repositories/session.rs) | 226 | Relational repository implementing transactional persistence via SQLite | `SessionRepository, new, get_session, list_sessions, save_session, delete_session` |
| [`platform/storage/src/repositories/span.rs`](../../platform/storage/src/repositories/span.rs) | 134 | Relational repository implementing transactional persistence via SQLite | `SpanRepository, new, get_span, save_span, list_spans` |
| [`platform/storage/src/repositories/task.rs`](../../platform/storage/src/repositories/task.rs) | 335 | Relational repository implementing transactional persistence via SQLite | `TaskRepository, new, get_task, save_task, save_task_with_event, save_task_with_event_and_command` |
| [`platform/storage/src/repositories/workflow_revision.rs`](../../platform/storage/src/repositories/workflow_revision.rs) | 276 | Relational repository implementing transactional persistence via SQLite | `WorkflowRevisionRepository, new, save_revision, get_revision, list_revisions_for_task, get_placements_for_revision` |
| [`platform/storage/src/repositories/workspace.rs`](../../platform/storage/src/repositories/workspace.rs) | 314 | Relational repository implementing transactional persistence via SQLite | `WorkspaceRepository, new, save_workspace, get_workspace, list_workspaces, update_workspace_status` |
| [`platform/storage/src/repositories/outbox/effect.rs`](../../platform/storage/src/repositories/outbox/effect.rs) | 196 | Relational repository implementing transactional persistence via SQLite | `effect_status_as_str, record_effect, update_effect_status, get_effect_by_idempotency_key, reconcile_on_startup, reconcile_inflight_to_uncertain` |
| [`platform/storage/src/repositories/outbox/entry.rs`](../../platform/storage/src/repositories/outbox/entry.rs) | 333 | Relational repository implementing transactional persistence via SQLite | `outbox_status_as_str` |
| [`platform/storage/src/repositories/outbox/mod.rs`](../../platform/storage/src/repositories/outbox/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | `OutboxRepository, new` |
| [`platform/storage/src/repositories/outbox/tests.rs`](../../platform/storage/src/repositories/outbox/tests.rs) | 207 | Relational repository implementing transactional persistence via SQLite | None |
| [`platform/storage/src/repositories/providers/catalog.rs`](../../platform/storage/src/repositories/providers/catalog.rs) | 192 | Relational repository implementing transactional persistence via SQLite | `save_catalog_models, catalog_fetched_at, list_catalog_models` |
| [`platform/storage/src/repositories/providers/credentials.rs`](../../platform/storage/src/repositories/providers/credentials.rs) | 327 | Relational repository implementing transactional persistence via SQLite | `credential_vault_id, credential_metadata, credential_accounts, credential_operations, prepare_credential_operation, update_credential_operation` |
| [`platform/storage/src/repositories/providers/local_profiles.rs`](../../platform/storage/src/repositories/providers/local_profiles.rs) | 314 | Relational repository implementing transactional persistence via SQLite | `save_local_folder, list_local_folders, get_local_folder, delete_local_folder, upsert_cached_model, list_cached_models` |
| [`platform/storage/src/repositories/providers/mod.rs`](../../platform/storage/src/repositories/providers/mod.rs) | 343 | Submodule root: aggregates and re-exports sibling modules | `ProviderRepository, new` |
| [`platform/storage/src/repositories/providers/usage.rs`](../../platform/storage/src/repositories/providers/usage.rs) | 15 | Relational repository implementing transactional persistence via SQLite | `provider_event_counts` |
| [`platform/storage/src/repositories/providers/accounts/config.rs`](../../platform/storage/src/repositories/providers/accounts/config.rs) | 239 | Relational repository implementing transactional persistence via SQLite | `list_providers, get_provider, save_provider, delete_provider, list_client_keys, create_client_key` |
| [`platform/storage/src/repositories/providers/accounts/mod.rs`](../../platform/storage/src/repositories/providers/accounts/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/storage/src/repositories/providers/accounts/oauth.rs`](../../platform/storage/src/repositories/providers/accounts/oauth.rs) | 330 | Relational repository implementing transactional persistence via SQLite | `save_oauth_token, save_oauth_connection, get_oauth_token, delete_oauth_token, list_expiring_oauth_tokens, list_oauth_tokens` |
| [`platform/storage/src/repositories/providers/accounts/routing.rs`](../../platform/storage/src/repositories/providers/accounts/routing.rs) | 149 | Relational repository implementing transactional persistence via SQLite | `routing_policy, save_routing_policy, select_account` |
| [`platform/storage/src/repositories/shared_capabilities/annotations.rs`](../../platform/storage/src/repositories/shared_capabilities/annotations.rs) | 190 | Relational repository implementing transactional persistence via SQLite | `save_annotation, list_annotations_for_artifact, list_pending_annotations, update_annotation_status` |
| [`platform/storage/src/repositories/shared_capabilities/artifacts.rs`](../../platform/storage/src/repositories/shared_capabilities/artifacts.rs) | 236 | Relational repository implementing transactional persistence via SQLite | `record_artifact_lineage, list_artifact_lineage, list_all_artifacts, get_artifact_lineage_graph` |
| [`platform/storage/src/repositories/shared_capabilities/mod.rs`](../../platform/storage/src/repositories/shared_capabilities/mod.rs) | 54 | Submodule root: aggregates and re-exports sibling modules | `SharedCapabilitiesRepository, new, from_raw, raw_db, ensure_schema` |
| [`platform/storage/src/repositories/shared_capabilities/notebooks.rs`](../../platform/storage/src/repositories/shared_capabilities/notebooks.rs) | 445 | Relational repository implementing transactional persistence via SQLite | `list_notebook_cells, list_notebook_cells_scoped, save_notebook_cell, save_notebook_cells, replace_notebook_cells, replace_notebook_cells_scoped` |
| [`platform/storage/src/repositories/shared_capabilities/notes.rs`](../../platform/storage/src/repositories/shared_capabilities/notes.rs) | 159 | Relational repository implementing transactional persistence via SQLite | `save_note, save_note_version, get_note, list_notes, list_note_versions` |
| [`platform/storage/src/repositories/shared_capabilities/reviews.rs`](../../platform/storage/src/repositories/shared_capabilities/reviews.rs) | 205 | Relational repository implementing transactional persistence via SQLite | `save_review, get_review, list_reviews, mark_review_stale` |
| [`platform/storage/src/repositories/shared_capabilities/tests.rs`](../../platform/storage/src/repositories/shared_capabilities/tests.rs) | 194 | Relational repository implementing transactional persistence via SQLite | None |
| [`platform/storage/src/repositories/fleet_automation/fleet_executions.rs`](../../platform/storage/src/repositories/fleet_automation/fleet_executions.rs) | 137 | Relational repository implementing transactional persistence via SQLite | `save_fleet_execution, list_fleet_executions` |
| [`platform/storage/src/repositories/fleet_automation/headless_jobs.rs`](../../platform/storage/src/repositories/fleet_automation/headless_jobs.rs) | 191 | Relational repository implementing transactional persistence via SQLite | `save_headless_job, get_headless_job, list_headless_jobs` |
| [`platform/storage/src/repositories/fleet_automation/mod.rs`](../../platform/storage/src/repositories/fleet_automation/mod.rs) | 26 | Submodule root: aggregates and re-exports sibling modules | `FleetAutomationRepository, new` |
| [`platform/storage/src/repositories/fleet_automation/remote_hosts.rs`](../../platform/storage/src/repositories/fleet_automation/remote_hosts.rs) | 198 | Relational repository implementing transactional persistence via SQLite | `save_remote_host, get_remote_host, list_remote_hosts` |
| [`platform/storage/src/repositories/fleet_automation/tests/fleet_automation_tests.rs`](../../platform/storage/src/repositories/fleet_automation/tests/fleet_automation_tests.rs) | 109 | Relational repository implementing transactional persistence via SQLite | None |
| [`platform/storage/src/repositories/fleet_automation/tests/mod.rs`](../../platform/storage/src/repositories/fleet_automation/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/storage/src/repositories/run/claims.rs`](../../platform/storage/src/repositories/run/claims.rs) | 238 | Relational repository implementing transactional persistence via SQLite | `claim_ready, update_claim, get_claim, get_active_claim, list_active_claims, fence_claim` |
| [`platform/storage/src/repositories/run/launches.rs`](../../platform/storage/src/repositories/run/launches.rs) | 93 | Relational repository implementing transactional persistence via SQLite | `save_launch_attempt, get_launch_attempt` |
| [`platform/storage/src/repositories/run/mod.rs`](../../platform/storage/src/repositories/run/mod.rs) | 22 | Submodule root: aggregates and re-exports sibling modules | `RunRepository, new` |
| [`platform/storage/src/repositories/run/runs.rs`](../../platform/storage/src/repositories/run/runs.rs) | 287 | Relational repository implementing transactional persistence via SQLite | `save_run, get_run, list_runs_for_task, list_uncertain_runs, reconcile_runs_on_startup` |
| [`platform/storage/src/repositories/run/telemetry.rs`](../../platform/storage/src/repositories/run/telemetry.rs) | 224 | Relational repository implementing transactional persistence via SQLite | `persist_telemetry_batch, prune_telemetry_before` |
| [`platform/storage/src/repositories/run/telemetry_queries.rs`](../../platform/storage/src/repositories/run/telemetry_queries.rs) | 338 | Relational repository implementing transactional persistence via SQLite | `get_telemetry_run_record, list_telemetry_runs, list_run_spans, list_logs, list_run_errors, list_errors` |
| [`platform/storage/src/repositories/run/workers.rs`](../../platform/storage/src/repositories/run/workers.rs) | 148 | Relational repository implementing transactional persistence via SQLite | `save_worker_run, get_worker_run, list_worker_runs_for_task, list_worker_runs_for_run` |
| [`platform/storage/src/repositories/run/tests/mod.rs`](../../platform/storage/src/repositories/run/tests/mod.rs) | 1 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/storage/src/repositories/run/tests/run_tests.rs`](../../platform/storage/src/repositories/run/tests/run_tests.rs) | 218 | Relational repository implementing transactional persistence via SQLite | None |
| [`platform/storage/src/store/mod.rs`](../../platform/storage/src/store/mod.rs) | 297 | Submodule root: aggregates and re-exports sibling modules | `SqliteTaskStore, new_in_memory, new, db, shared_capabilities, writer_arc` |
| [`platform/storage/src/store/seed.rs`](../../platform/storage/src/store/seed.rs) | 139 | SQLite database connection manager, WAL configuration and migration runner | `seed_canonical_data_if_empty` |
| [`platform/storage/src/store/tests.rs`](../../platform/storage/src/store/tests.rs) | 424 | SQLite database connection manager, WAL configuration and migration runner | None |
| [`platform/storage/src/store/traits.rs`](../../platform/storage/src/store/traits.rs) | 465 | SQLite database connection manager, WAL configuration and migration runner | None |

### 3.4. Crate `custos-provider` — Layer 1: Model Contracts, Pricing & Inference

- **Đường dẫn thư mục:** `platform/ai`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** contracts/ cấm import cloud/; không keyring; local inference khởi tạo lười (lazy LlamaCppBackend) không chiếm RAM/Metal lúc boot.
- **Tổng số tệp:** 344 tệp (343 tệp Rust) | **Tổng số dòng mã:** 63,787 dòng Rust.
- **Mô tả chức năng:** Cung cấp năng lực mô hình: ModelPort, catalog giá và latency, Cloud Codecs (Anthropic, OpenAI, Azure, Gemini, Ollama), Local Inference và Dynamic Routing.

#### Danh mục các file bên trong `platform/ai`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/ai/Cargo.toml`](../../platform/ai/Cargo.toml) | 79 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/ai/build.rs`](../../platform/ai/build.rs) | 14 | Core architectural component supporting module responsibilities | None |
| [`platform/ai/src/lib.rs`](../../platform/ai/src/lib.rs) | 33 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/ai/src/contracts/bridge.rs`](../../platform/ai/src/contracts/bridge.rs) | 295 | ModelPort contracts, message formats and token usage DTOs | `ProviderToModelPort, new, inner` |
| [`platform/ai/src/contracts/bridge_outbound.rs`](../../platform/ai/src/contracts/bridge_outbound.rs) | 165 | ModelPort contracts, message formats and token usage DTOs | `ModelPortToProvider, new, with_context_limit, inner` |
| [`platform/ai/src/contracts/bridge_tests.rs`](../../platform/ai/src/contracts/bridge_tests.rs) | 121 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/conformance.rs`](../../platform/ai/src/contracts/conformance.rs) | 94 | ModelPort contracts, message formats and token usage DTOs | `assert_provider_conformance, assert_legacy_bridge_rejects_lossy_turn` |
| [`platform/ai/src/contracts/events.rs`](../../platform/ai/src/contracts/events.rs) | 109 | ModelPort contracts, message formats and token usage DTOs | `ProviderEventType, ToolCall, TokenUsage, ProviderEvent, chunk, tool_call` |
| [`platform/ai/src/contracts/mod.rs`](../../platform/ai/src/contracts/mod.rs) | 18 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/port.rs`](../../platform/ai/src/contracts/port.rs) | 169 | ModelPort contracts, message formats and token usage DTOs | `ModelProvider` |
| [`platform/ai/src/contracts/request.rs`](../../platform/ai/src/contracts/request.rs) | 153 | ModelPort contracts, message formats and token usage DTOs | `ProviderRequest, ModelResponse, simple, new, with_usage, text` |
| [`platform/ai/src/contracts/tool.rs`](../../platform/ai/src/contracts/tool.rs) | 25 | ModelPort contracts, message formats and token usage DTOs | `ToolResult, ToolDefinition, ToolPort` |
| [`platform/ai/src/contracts/turn.rs`](../../platform/ai/src/contracts/turn.rs) | 102 | ModelPort contracts, message formats and token usage DTOs | `ModelTurnRequest, ModelTurnEvent, TurnDelta` |
| [`platform/ai/src/contracts/types/cache_semantics.rs`](../../platform/ai/src/contracts/types/cache_semantics.rs) | 342 | ModelPort contracts, message formats and token usage DTOs | `CacheSemantics, for_model, uses_explicit_breakpoints, apply_chat_payload_breakpoints` |
| [`platform/ai/src/contracts/types/context_limit.rs`](../../platform/ai/src/contracts/types/context_limit.rs) | 195 | ModelPort contracts, message formats and token usage DTOs | `ContextLimitResolver, new, with_configured_limits, resolve_local, resolve` |
| [`platform/ai/src/contracts/types/custos_mode.rs`](../../platform/ai/src/contracts/types/custos_mode.rs) | 36 | ModelPort contracts, message formats and token usage DTOs | `CustosMode` |
| [`platform/ai/src/contracts/types/documents.rs`](../../platform/ai/src/contracts/types/documents.rs) | 80 | ModelPort contracts, message formats and token usage DTOs | `DocumentFormat, document_media_type_is_supported, convert_document, unsupported_document_text` |
| [`platform/ai/src/contracts/types/errors.rs`](../../platform/ai/src/contracts/types/errors.rs) | 309 | ModelPort contracts, message formats and token usage DTOs | `ProviderError, GoogleErrorCode, stream_decode_error, telemetry_type, is_endpoint_not_found, from_stream_error` |
| [`platform/ai/src/contracts/types/images.rs`](../../platform/ai/src/contracts/types/images.rs) | 545 | ModelPort contracts, message formats and token usage DTOs | `ImageFormat, convert_image, detect_image_path, parse_image_bytes, load_image_file` |
| [`platform/ai/src/contracts/types/json.rs`](../../platform/ai/src/contracts/types/json.rs) | 559 | ModelPort contracts, message formats and token usage DTOs | `safely_parse_json, json_escape_control_chars_in_string, looks_truncated, truncation_error_message, parse_tool_arguments` |
| [`platform/ai/src/contracts/types/mcp_utils.rs`](../../platform/ai/src/contracts/types/mcp_utils.rs) | 109 | ModelPort contracts, message formats and token usage DTOs | `extract_text_from_resource` |
| [`platform/ai/src/contracts/types/mod.rs`](../../platform/ai/src/contracts/types/mod.rs) | 22 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/permission.rs`](../../platform/ai/src/contracts/types/permission.rs) | 23 | ModelPort contracts, message formats and token usage DTOs | `Permission, PrincipalType, PermissionConfirmation` |
| [`platform/ai/src/contracts/types/request_log.rs`](../../platform/ai/src/contracts/types/request_log.rs) | 134 | ModelPort contracts, message formats and token usage DTOs | `LoggerAlreadyInstalled, RequestLogger, RequestLogHandle, LogError, LoggerHandleExt, install_logger` |
| [`platform/ai/src/contracts/types/retry.rs`](../../platform/ai/src/contracts/types/retry.rs) | 359 | ModelPort contracts, message formats and token usage DTOs | `RetryConfig, ProviderRetry, new, transient_only, max_retries, delay_for_attempt` |
| [`platform/ai/src/contracts/types/utils.rs`](../../platform/ai/src/contracts/types/utils.rs) | 29 | ModelPort contracts, message formats and token usage DTOs | `sanitize_unicode_tags, strip_unicode_tags, get_model` |
| [`platform/ai/src/contracts/types/thinking/effort.rs`](../../platform/ai/src/contracts/types/thinking/effort.rs) | 67 | ModelPort contracts, message formats and token usage DTOs | `ThinkingEffort, ThinkingEffortOption, ThinkingEffortCapability, ThinkingEffortSupport` |
| [`platform/ai/src/contracts/types/thinking/filter.rs`](../../platform/ai/src/contracts/types/thinking/filter.rs) | 302 | ModelPort contracts, message formats and token usage DTOs | `FilterOut, ThinkFilter, split_think_blocks, new, push, finish` |
| [`platform/ai/src/contracts/types/thinking/mod.rs`](../../platform/ai/src/contracts/types/thinking/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/thinking/tests.rs`](../../platform/ai/src/contracts/types/thinking/tests.rs) | 372 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/model/builder.rs`](../../platform/ai/src/contracts/types/model/builder.rs) | 189 | ModelPort contracts, message formats and token usage DTOs | `new, with_canonical_vision_support, with_canonical_limits, with_context_limit, with_temperature, with_max_tokens` |
| [`platform/ai/src/contracts/types/model/mod.rs`](../../platform/ai/src/contracts/types/model/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/model/reasoning.rs`](../../platform/ai/src/contracts/types/model/reasoning.rs) | 231 | ModelPort contracts, message formats and token usage DTOs | `is_openai_responses_model, extract_reasoning_effort, supports_xai_reasoning_effort, is_xai_reasoning_model, with_thinking_effort, with_default_thinking_effort` |
| [`platform/ai/src/contracts/types/model/types.rs`](../../platform/ai/src/contracts/types/model/types.rs) | 100 | ModelPort contracts, message formats and token usage DTOs | `ModelConfig, is_custos_internal_request_param` |
| [`platform/ai/src/contracts/types/model/tests/config_and_ttl_tests.rs`](../../platform/ai/src/contracts/types/model/tests/config_and_ttl_tests.rs) | 69 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/model/tests/effort_tests.rs`](../../platform/ai/src/contracts/types/model/tests/effort_tests.rs) | 265 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/model/tests/mod.rs`](../../platform/ai/src/contracts/types/model/tests/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/model/tests/reasoning_model_tests.rs`](../../platform/ai/src/contracts/types/model/tests/reasoning_model_tests.rs) | 98 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/model/tests/vision_and_limits_tests.rs`](../../platform/ai/src/contracts/types/model/tests/vision_and_limits_tests.rs) | 160 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/base/metadata.rs`](../../platform/ai/src/contracts/types/base/metadata.rs) | 298 | ModelPort contracts, message formats and token usage DTOs | `ProviderMetadata, ProviderDeprecation, ConfigKey, ThinkingPreservationFormat, ModelInfo, ProviderDescriptor` |
| [`platform/ai/src/contracts/types/base/mod.rs`](../../platform/ai/src/contracts/types/base/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/base/provider_trait.rs`](../../platform/ai/src/contracts/types/base/provider_trait.rs) | 281 | ModelPort contracts, message formats and token usage DTOs | `Provider, model_info_for_provider_model, known_models_from_registry` |
| [`platform/ai/src/contracts/types/base/stream.rs`](../../platform/ai/src/contracts/types/base/stream.rs) | 109 | ModelPort contracts, message formats and token usage DTOs | `PermissionRouting, collect_stream, stream_from_single_message` |
| [`platform/ai/src/contracts/types/base/tests/mod.rs`](../../platform/ai/src/contracts/types/base/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/base/tests/model_tests.rs`](../../platform/ai/src/contracts/types/base/tests/model_tests.rs) | 132 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/base/tests/stream_tests.rs`](../../platform/ai/src/contracts/types/base/tests/stream_tests.rs) | 255 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/core.rs`](../../platform/ai/src/contracts/types/conversation/core.rs) | 225 | ModelPort contracts, message formats and token usage DTOs | `Conversation, InvalidConversation, new, new_unvalidated, empty, messages` |
| [`platform/ai/src/contracts/types/conversation/fix.rs`](../../platform/ai/src/contracts/types/conversation/fix.rs) | 474 | ModelPort contracts, message formats and token usage DTOs | `EffectiveRole, effective_role, fix_conversation, merge_consecutive_messages, merge_consecutive_messages_for_request, debug_conversation_fix` |
| [`platform/ai/src/contracts/types/conversation/mod.rs`](../../platform/ai/src/contracts/types/conversation/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/conversation/token_usage.rs`](../../platform/ai/src/contracts/types/conversation/token_usage.rs) | 288 | ModelPort contracts, message formats and token usage DTOs | `ProviderUsage, CostSource, ProviderStats, DraftStats, Usage, new` |
| [`platform/ai/src/contracts/types/conversation/tool_request.rs`](../../platform/ai/src/contracts/types/conversation/tool_request.rs) | 288 | ModelPort contracts, message formats and token usage DTOs | `tool_name_parts, to_readable_string, was_executed_externally, generated_title, provider_index, generated_chain_summary` |
| [`platform/ai/src/contracts/types/conversation/tool_result_serde.rs`](../../platform/ai/src/contracts/types/conversation/tool_result_serde.rs) | 194 | ModelPort contracts, message formats and token usage DTOs | `serialize, deserialize` |
| [`platform/ai/src/contracts/types/conversation/tests/basic_and_merge.rs`](../../platform/ai/src/contracts/types/conversation/tests/basic_and_merge.rs) | 394 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/tests/helpers.rs`](../../platform/ai/src/contracts/types/conversation/tests/helpers.rs) | 35 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/tests/mod.rs`](../../platform/ai/src/contracts/types/conversation/tests/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/conversation/tests/shadow_map.rs`](../../platform/ai/src/contracts/types/conversation/tests/shadow_map.rs) | 436 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/tests/thinking_and_push.rs`](../../platform/ai/src/contracts/types/conversation/tests/thinking_and_push.rs) | 415 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/message/content.rs`](../../platform/ai/src/contracts/types/conversation/message/content.rs) | 445 | ModelPort contracts, message formats and token usage DTOs | `ThinkingContentBlock, RedactedThinkingContentBlock, SystemNotificationType, SystemNotificationContent, MessageErrorKind, ErrorContent` |
| [`platform/ai/src/contracts/types/conversation/message/content_accessors.rs`](../../platform/ai/src/contracts/types/conversation/message/content_accessors.rs) | 146 | ModelPort contracts, message formats and token usage DTOs | `thinking, redacted_thinking, system_notification, system_notification_with_data, error, as_error` |
| [`platform/ai/src/contracts/types/conversation/message/metadata.rs`](../../platform/ai/src/contracts/types/conversation/message/metadata.rs) | 215 | ModelPort contracts, message formats and token usage DTOs | `InferenceMetadata, MessageUsage, MessageMetadata, TokenState, from_provider_usage, operation_note` |
| [`platform/ai/src/contracts/types/conversation/message/mod.rs`](../../platform/ai/src/contracts/types/conversation/message/mod.rs) | 446 | Submodule root: aggregates and re-exports sibling modules | `Message, new, debug, agent_visible_content, user_visible_content, user` |
| [`platform/ai/src/contracts/types/conversation/message/tool.rs`](../../platform/ai/src/contracts/types/conversation/message/tool.rs) | 169 | ModelPort contracts, message formats and token usage DTOs | `ToolCallResult, ToolRequest, ToolNameParts, ToolChainSummary, ToolResponse, ToolConfirmationRequest` |
| [`platform/ai/src/contracts/types/conversation/message/tests/document.rs`](../../platform/ai/src/contracts/types/conversation/message/tests/document.rs) | 46 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/message/tests/message_methods.rs`](../../platform/ai/src/contracts/types/conversation/message/tests/message_methods.rs) | 372 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/message/tests/mod.rs`](../../platform/ai/src/contracts/types/conversation/message/tests/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/contracts/types/conversation/message/tests/sanitization.rs`](../../platform/ai/src/contracts/types/conversation/message/tests/sanitization.rs) | 268 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/contracts/types/conversation/message/tests/serde.rs`](../../platform/ai/src/contracts/types/conversation/message/tests/serde.rs) | 372 | ModelPort contracts, message formats and token usage DTOs | None |
| [`platform/ai/src/catalog/capabilities.rs`](../../platform/ai/src/catalog/capabilities.rs) | 156 | Model catalog, pricing rates, capabilities and provider endpoints | `ModelCapabilities, get_model_capabilities` |
| [`platform/ai/src/catalog/combo.rs`](../../platform/ai/src/catalog/combo.rs) | 294 | Model catalog, pricing rates, capabilities and provider endpoints | `ModelTier, RequiredCapability, ComboPreset, ComboRegistry, RouteConstraints, as_str` |
| [`platform/ai/src/catalog/mod.rs`](../../platform/ai/src/catalog/mod.rs) | 23 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/catalog/models.rs`](../../platform/ai/src/catalog/models.rs) | 52 | Model catalog, pricing rates, capabilities and provider endpoints | `ContextLimits, Modalities, ThinkingFormat, ModelDescriptor` |
| [`platform/ai/src/catalog/pricing.rs`](../../platform/ai/src/catalog/pricing.rs) | 419 | Model catalog, pricing rates, capabilities and provider endpoints | `PricingRate, zero, calculate_cost, get_model_pricing, normalize_model_id, lookup_model_pricing` |
| [`platform/ai/src/catalog/service.rs`](../../platform/ai/src/catalog/service.rs) | 560 | Model catalog, pricing rates, capabilities and provider endpoints | `ModelCatalogService, new, get_catalog, merge_probed_models, default_canonical_models` |
| [`platform/ai/src/catalog/canonical/mod.rs`](../../platform/ai/src/catalog/canonical/mod.rs) | 368 | Submodule root: aggregates and re-exports sibling modules | `ModelMapping, new, recommended_models_from_registry, provider_wire_name, maybe_get_canonical_model` |
| [`platform/ai/src/catalog/canonical/model.rs`](../../platform/ai/src/catalog/canonical/model.rs) | 244 | Model catalog, pricing rates, capabilities and provider endpoints | `Modality, CanonicalModalities, Pricing, Limit, ThinkingMode, CanonicalModel` |
| [`platform/ai/src/catalog/canonical/registry.rs`](../../platform/ai/src/catalog/canonical/registry.rs) | 114 | Model catalog, pricing rates, capabilities and provider endpoints | `CanonicalModelRegistry, new, bundled, from_json_str, to_json_string, from_file` |
| [`platform/ai/src/catalog/canonical/name_builder/mapper.rs`](../../platform/ai/src/catalog/canonical/name_builder/mapper.rs) | 122 | Model catalog, pricing rates, capabilities and provider endpoints | `canonical_name, map_provider_name, map_to_canonical_model` |
| [`platform/ai/src/catalog/canonical/name_builder/mod.rs`](../../platform/ai/src/catalog/canonical/name_builder/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/catalog/canonical/name_builder/patterns.rs`](../../platform/ai/src/catalog/canonical/name_builder/patterns.rs) | 205 | Model catalog, pricing rates, capabilities and provider endpoints | `strip_version_suffix` |
| [`platform/ai/src/catalog/canonical/name_builder/tests.rs`](../../platform/ai/src/catalog/canonical/name_builder/tests.rs) | 287 | Model catalog, pricing rates, capabilities and provider endpoints | None |
| [`platform/ai/src/catalog/canonical/catalog/metadata.rs`](../../platform/ai/src/catalog/canonical/catalog/metadata.rs) | 332 | Model catalog, pricing rates, capabilities and provider endpoints | `ProviderSetupCategory, ProviderSetupMethod, ProviderSetupGroup, ProviderSetupField, ProviderSetupCapabilities, ProviderSetupCatalogEntry` |
| [`platform/ai/src/catalog/canonical/catalog/mod.rs`](../../platform/ai/src/catalog/canonical/catalog/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/catalog/canonical/catalog/providers.rs`](../../platform/ai/src/catalog/canonical/catalog/providers.rs) | 212 | Model catalog, pricing rates, capabilities and provider endpoints | `ProviderFormat, ProviderCatalogEntry, ProviderTemplate, ModelTemplate, TemplateModelCapabilities, as_str` |
| [`platform/ai/src/catalog/canonical/catalog/tests.rs`](../../platform/ai/src/catalog/canonical/catalog/tests.rs) | 88 | Model catalog, pricing rates, capabilities and provider endpoints | None |
| [`platform/ai/src/local/mod.rs`](../../platform/ai/src/local/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/mod.rs`](../../platform/ai/src/local/local_inference/mod.rs) | 35 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/tests.rs`](../../platform/ai/src/local/local_inference/tests.rs) | 117 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/backend.rs`](../../platform/ai/src/local/local_inference/runtime/backend.rs) | 52 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/config_resolver.rs`](../../platform/ai/src/local/local_inference/runtime/config_resolver.rs) | 60 | Local model inference backend with lazy resource allocation | `set_string_param_resolver, set_bool_param_resolver, set_model_settings_resolver, set_model_settings_writer, string_param, bool_param` |
| [`platform/ai/src/local/local_inference/runtime/messages.rs`](../../platform/ai/src/local/local_inference/runtime/messages.rs) | 224 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mod.rs`](../../platform/ai/src/local/local_inference/runtime/mod.rs) | 41 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/model.rs`](../../platform/ai/src/local/local_inference/runtime/model.rs) | 138 | Local model inference backend with lazy resource allocation | `SamplingConfig, ToolCallingMode, ChatTemplate, ModelSettings` |
| [`platform/ai/src/local/local_inference/runtime/multimodal.rs`](../../platform/ai/src/local/local_inference/runtime/multimodal.rs) | 338 | Local model inference backend with lazy resource allocation | `ExtractedImage, MultimodalMessages, extract_images_from_messages_json, extract_images_from_messages` |
| [`platform/ai/src/local/local_inference/runtime/native_tool_parsing.rs`](../../platform/ai/src/local/local_inference/runtime/native_tool_parsing.rs) | 343 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/paths.rs`](../../platform/ai/src/local/local_inference/runtime/paths.rs) | 122 | Local model inference backend with lazy resource allocation | `Paths, config_dir, data_dir, state_dir, plugins_dir, agents_dir` |
| [`platform/ai/src/local/local_inference/runtime/preload.rs`](../../platform/ai/src/local/local_inference/runtime/preload.rs) | 273 | Local model inference backend with lazy resource allocation | `preload_model, preload_model_with_profile` |
| [`platform/ai/src/local/local_inference/runtime/prompt_template.rs`](../../platform/ai/src/local/local_inference/runtime/prompt_template.rs) | 43 | Local model inference backend with lazy resource allocation | `render_string, render_template` |
| [`platform/ai/src/local/local_inference/runtime/resolution.rs`](../../platform/ai/src/local/local_inference/runtime/resolution.rs) | 177 | Local model inference backend with lazy resource allocation | `ResolvedModelPaths, local_context_limit, available_inference_memory_bytes` |
| [`platform/ai/src/local/local_inference/runtime/runtime.rs`](../../platform/ai/src/local/local_inference/runtime/runtime.rs) | 366 | Local model inference backend with lazy resource allocation | `InferenceRuntime, builtin_chat_template_names, set_admission_coordinator, get_or_init, with_admission, get_admission_coordinator` |
| [`platform/ai/src/local/local_inference/runtime/thinking_output.rs`](../../platform/ai/src/local/local_inference/runtime/thinking_output.rs) | 83 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/tool_parsing.rs`](../../platform/ai/src/local/local_inference/runtime/tool_parsing.rs) | 53 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/backend.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/backend.rs) | 322 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/hardware.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/hardware.rs) | 81 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_emulated_tools.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_emulated_tools.rs) | 385 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_native_tools.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_native_tools.rs) | 409 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/mod.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/templates.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/templates.rs) | 279 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/tests.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/tests.rs) | 114 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/context.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/context.rs) | 177 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/engine.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/engine.rs) | 335 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/mod.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/sampler.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/sampler.rs) | 48 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/tests.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/tests.rs) | 74 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/trimmer.rs`](../../platform/ai/src/local/local_inference/runtime/llamacpp/inference_engine/trimmer.rs) | 60 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/sidecar/chat.rs`](../../platform/ai/src/local/local_inference/runtime/sidecar/chat.rs) | 151 | Local model inference backend with lazy resource allocation | `stream_sidecar_chat` |
| [`platform/ai/src/local/local_inference/runtime/sidecar/lifecycle.rs`](../../platform/ai/src/local/local_inference/runtime/sidecar/lifecycle.rs) | 350 | Local model inference backend with lazy resource allocation | `start_sidecar, set_sidecar_generation_verified, verify_sidecar_generation, stop_sidecar` |
| [`platform/ai/src/local/local_inference/runtime/sidecar/mod.rs`](../../platform/ai/src/local/local_inference/runtime/sidecar/mod.rs) | 18 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/sidecar/types.rs`](../../platform/ai/src/local/local_inference/runtime/sidecar/types.rs) | 124 | Local model inference backend with lazy resource allocation | `ActiveSidecar, find_llama_server_binary, get_sidecar_port, is_sidecar_healthy, get_loaded_sidecar_instance` |
| [`platform/ai/src/local/local_inference/runtime/mlx/imp_stub.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/imp_stub.rs) | 59 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/mod.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/validation.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/validation.rs) | 85 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/safemlx/backend.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/safemlx/backend.rs) | 344 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/safemlx/generation.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/safemlx/generation.rs) | 367 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/safemlx/mod.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/safemlx/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/safemlx/prompt.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/safemlx/prompt.rs) | 239 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/mlx/safemlx/tests.rs`](../../platform/ai/src/local/local_inference/runtime/mlx/safemlx/tests.rs) | 82 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/tool_emulation/mod.rs`](../../platform/ai/src/local/local_inference/runtime/tool_emulation/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/runtime/tool_emulation/parser.rs`](../../platform/ai/src/local/local_inference/runtime/tool_emulation/parser.rs) | 353 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/tool_emulation/prompt.rs`](../../platform/ai/src/local/local_inference/runtime/tool_emulation/prompt.rs) | 168 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/runtime/tool_emulation/tests.rs`](../../platform/ai/src/local/local_inference/runtime/tool_emulation/tests.rs) | 218 | Local model inference backend with lazy resource allocation | None |
| [`platform/ai/src/local/local_inference/discovery/folder_scanner.rs`](../../platform/ai/src/local/local_inference/discovery/folder_scanner.rs) | 118 | Local model inference backend with lazy resource allocation | `scan_folder_for_models` |
| [`platform/ai/src/local/local_inference/discovery/gguf_inspector.rs`](../../platform/ai/src/local/local_inference/discovery/gguf_inspector.rs) | 459 | Local model inference backend with lazy resource allocation | `GgufType, inspect_gguf_file` |
| [`platform/ai/src/local/local_inference/discovery/metal_probe.rs`](../../platform/ai/src/local/local_inference/discovery/metal_probe.rs) | 137 | Local model inference backend with lazy resource allocation | `probe_local_hardware` |
| [`platform/ai/src/local/local_inference/discovery/mod.rs`](../../platform/ai/src/local/local_inference/discovery/mod.rs) | 6 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/adapter/adapter.rs`](../../platform/ai/src/local/local_inference/adapter/adapter.rs) | 406 | Local model inference backend with lazy resource allocation | `NativeLocalModelAdapter, new, provider` |
| [`platform/ai/src/local/local_inference/adapter/mod.rs`](../../platform/ai/src/local/local_inference/adapter/mod.rs) | 11 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/local/local_inference/adapter/provider.rs`](../../platform/ai/src/local/local_inference/adapter/provider.rs) | 520 | Local model inference backend with lazy resource allocation | `LocalInferenceProvider, new, runtime, from_env` |
| [`platform/ai/src/local/local_inference/adapter/provider_utils.rs`](../../platform/ai/src/local/local_inference/adapter/provider_utils.rs) | 24 | Local model inference backend with lazy resource allocation | `filter_extensions_from_system_prompt` |
| [`platform/ai/src/cloud/mod.rs`](../../platform/ai/src/cloud/mod.rs) | 25 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/google_codeassist/inference.rs`](../../platform/ai/src/cloud/google_codeassist/inference.rs) | 205 | Cloud provider transport codec and streaming protocol client | `stream_codeassist, generate_content, generate_codeassist_content` |
| [`platform/ai/src/cloud/google_codeassist/mod.rs`](../../platform/ai/src/cloud/google_codeassist/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/google_codeassist/models.rs`](../../platform/ai/src/cloud/google_codeassist/models.rs) | 83 | Cloud provider transport codec and streaming protocol client | `supports_tiered_reasoning, probe_models, probe_codeassist_models` |
| [`platform/ai/src/cloud/google_codeassist/project.rs`](../../platform/ai/src/cloud/google_codeassist/project.rs) | 88 | Cloud provider transport codec and streaming protocol client | `discover_project` |
| [`platform/ai/src/cloud/google_codeassist/tests.rs`](../../platform/ai/src/cloud/google_codeassist/tests.rs) | 269 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/google_codeassist/transport.rs`](../../platform/ai/src/cloud/google_codeassist/transport.rs) | 137 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/mod.rs`](../../platform/ai/src/cloud/codecs/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/ollama.rs`](../../platform/ai/src/cloud/codecs/ollama.rs) | 450 | Cloud provider transport codec and streaming protocol client | `parse_xml_tool_calls, response_to_message, response_to_streaming_message_ollama` |
| [`platform/ai/src/cloud/codecs/openrouter/mod.rs`](../../platform/ai/src/cloud/codecs/openrouter/mod.rs) | 374 | Submodule root: aggregates and re-exports sibling modules | `extract_reasoning_details, get_reasoning_details, response_to_message, add_reasoning_details_to_request, apply_reasoning_config` |
| [`platform/ai/src/cloud/codecs/google/messages.rs`](../../platform/ai/src/cloud/codecs/google/messages.rs) | 265 | Cloud provider transport codec and streaming protocol client | `metadata_with_signature, get_thought_signature, format_messages` |
| [`platform/ai/src/cloud/codecs/google/mod.rs`](../../platform/ai/src/cloud/codecs/google/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/google/request.rs`](../../platform/ai/src/cloud/codecs/google/request.rs) | 215 | Cloud provider transport codec and streaming protocol client | `create_request, create_request_with_thinking_budget` |
| [`platform/ai/src/cloud/codecs/google/response.rs`](../../platform/ai/src/cloud/codecs/google/response.rs) | 163 | Cloud provider transport codec and streaming protocol client | `response_to_message, get_usage` |
| [`platform/ai/src/cloud/codecs/google/streaming.rs`](../../platform/ai/src/cloud/codecs/google/streaming.rs) | 143 | Cloud provider transport codec and streaming protocol client | `response_to_streaming_message` |
| [`platform/ai/src/cloud/codecs/google/tools.rs`](../../platform/ai/src/cloud/codecs/google/tools.rs) | 24 | Cloud provider transport codec and streaming protocol client | `format_tools` |
| [`platform/ai/src/cloud/codecs/google/tests/helpers.rs`](../../platform/ai/src/cloud/codecs/google/tests/helpers.rs) | 50 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/messages.rs`](../../platform/ai/src/cloud/codecs/google/tests/messages.rs) | 268 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/mod.rs`](../../platform/ai/src/cloud/codecs/google/tests/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/google/tests/request.rs`](../../platform/ai/src/cloud/codecs/google/tests/request.rs) | 121 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/response.rs`](../../platform/ai/src/cloud/codecs/google/tests/response.rs) | 247 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/streaming.rs`](../../platform/ai/src/cloud/codecs/google/tests/streaming.rs) | 279 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/tools.rs`](../../platform/ai/src/cloud/codecs/google/tests/tools.rs) | 89 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/google/tests/usage.rs`](../../platform/ai/src/cloud/codecs/google/tests/usage.rs) | 64 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/messages.rs`](../../platform/ai/src/cloud/codecs/databricks/messages.rs) | 279 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/mod.rs`](../../platform/ai/src/cloud/codecs/databricks/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/databricks/request.rs`](../../platform/ai/src/cloud/codecs/databricks/request.rs) | 202 | Cloud provider transport codec and streaming protocol client | `create_request, create_request_for_provider` |
| [`platform/ai/src/cloud/codecs/databricks/response.rs`](../../platform/ai/src/cloud/codecs/databricks/response.rs) | 140 | Cloud provider transport codec and streaming protocol client | `response_to_message` |
| [`platform/ai/src/cloud/codecs/databricks/tools.rs`](../../platform/ai/src/cloud/codecs/databricks/tools.rs) | 39 | Cloud provider transport codec and streaming protocol client | `format_tools` |
| [`platform/ai/src/cloud/codecs/databricks/tests/documents.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/documents.rs) | 45 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/tests/messages.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/messages.rs) | 463 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/tests/mod.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/mod.rs) | 25 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/databricks/tests/request.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/request.rs) | 296 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/tests/response.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/response.rs) | 250 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/databricks/tests/tools_and_ordering.rs`](../../platform/ai/src/cloud/codecs/databricks/tests/tools_and_ordering.rs) | 357 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/snowflake/messages.rs`](../../platform/ai/src/cloud/codecs/snowflake/messages.rs) | 133 | Cloud provider transport codec and streaming protocol client | `format_messages, format_tools, format_system` |
| [`platform/ai/src/cloud/codecs/snowflake/mod.rs`](../../platform/ai/src/cloud/codecs/snowflake/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/snowflake/request.rs`](../../platform/ai/src/cloud/codecs/snowflake/request.rs) | 56 | Cloud provider transport codec and streaming protocol client | `create_request` |
| [`platform/ai/src/cloud/codecs/snowflake/response.rs`](../../platform/ai/src/cloud/codecs/snowflake/response.rs) | 115 | Cloud provider transport codec and streaming protocol client | `response_to_message, get_usage` |
| [`platform/ai/src/cloud/codecs/snowflake/streaming.rs`](../../platform/ai/src/cloud/codecs/snowflake/streaming.rs) | 84 | Cloud provider transport codec and streaming protocol client | `parse_streaming_response` |
| [`platform/ai/src/cloud/codecs/snowflake/tests.rs`](../../platform/ai/src/cloud/codecs/snowflake/tests.rs) | 352 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/messages.rs`](../../platform/ai/src/cloud/codecs/anthropic/messages.rs) | 283 | Cloud provider transport codec and streaming protocol client | `args_to_input_value, format_messages, format_messages_with_options` |
| [`platform/ai/src/cloud/codecs/anthropic/mod.rs`](../../platform/ai/src/cloud/codecs/anthropic/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/anthropic/options.rs`](../../platform/ai/src/cloud/codecs/anthropic/options.rs) | 358 | Cloud provider transport codec and streaming protocol client | `AnthropicFormatOptions, native, thinking_block_is_stale, requires_explicit_thinking_disable, model_supports_temperature, thinking_type` |
| [`platform/ai/src/cloud/codecs/anthropic/request.rs`](../../platform/ai/src/cloud/codecs/anthropic/request.rs) | 86 | Cloud provider transport codec and streaming protocol client | `create_request, create_request_for_model` |
| [`platform/ai/src/cloud/codecs/anthropic/response.rs`](../../platform/ai/src/cloud/codecs/anthropic/response.rs) | 182 | Cloud provider transport codec and streaming protocol client | `response_to_message, get_usage, input_transformations, get_additional_data` |
| [`platform/ai/src/cloud/codecs/anthropic/streaming.rs`](../../platform/ai/src/cloud/codecs/anthropic/streaming.rs) | 369 | Cloud provider transport codec and streaming protocol client | `response_to_streaming_message` |
| [`platform/ai/src/cloud/codecs/anthropic/tools.rs`](../../platform/ai/src/cloud/codecs/anthropic/tools.rs) | 61 | Cloud provider transport codec and streaming protocol client | `format_tools, format_system` |
| [`platform/ai/src/cloud/codecs/anthropic/tests/cache_breakpoints.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/cache_breakpoints.rs) | 224 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/documents.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/documents.rs) | 66 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/helpers.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/helpers.rs) | 154 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/messages.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/messages.rs) | 474 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/mod.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/response.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/response.rs) | 169 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/streaming.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/streaming.rs) | 535 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/anthropic/tests/thinking.rs`](../../platform/ai/src/cloud/codecs/anthropic/tests/thinking.rs) | 376 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/mod.rs`](../../platform/ai/src/cloud/codecs/openai_responses/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/openai_responses/request.rs`](../../platform/ai/src/cloud/codecs/openai_responses/request.rs) | 379 | Cloud provider transport codec and streaming protocol client | `create_responses_request, create_responses_request_for_model` |
| [`platform/ai/src/cloud/codecs/openai_responses/response.rs`](../../platform/ai/src/cloud/codecs/openai_responses/response.rs) | 150 | Cloud provider transport codec and streaming protocol client | `responses_api_to_message, get_responses_usage` |
| [`platform/ai/src/cloud/codecs/openai_responses/streaming.rs`](../../platform/ai/src/cloud/codecs/openai_responses/streaming.rs) | 331 | Cloud provider transport codec and streaming protocol client | `responses_api_to_streaming_message` |
| [`platform/ai/src/cloud/codecs/openai_responses/types.rs`](../../platform/ai/src/cloud/codecs/openai_responses/types.rs) | 375 | Cloud provider transport codec and streaming protocol client | `ResponsesApiResponse, SummaryText, ResponseOutputItem, ResponseContentBlock, ResponseReasoningInfo, ResponseIncompleteDetails` |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/documents.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/documents.rs) | 47 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/mod.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/request.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/request.rs) | 304 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/request_images.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/request_images.rs) | 377 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/response.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/response.rs) | 243 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/sanitization.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/sanitization.rs) | 269 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/streaming.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/streaming.rs) | 433 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai_responses/tests/tools.rs`](../../platform/ai/src/cloud/codecs/openai_responses/tests/tools.rs) | 252 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/messages.rs`](../../platform/ai/src/cloud/codecs/openai/messages.rs) | 477 | Cloud provider transport codec and streaming protocol client | `format_messages, format_messages_with_options` |
| [`platform/ai/src/cloud/codecs/openai/mod.rs`](../../platform/ai/src/cloud/codecs/openai/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/openai/options.rs`](../../platform/ai/src/cloud/codecs/openai/options.rs) | 123 | Cloud provider transport codec and streaming protocol client | `OpenAiFormatOptions, is_reserved_request_param_key, xai_reasoning_effort_for_thinking, openai_reasoning_effort_for_thinking, sanitize_function_name, is_valid_function_name` |
| [`platform/ai/src/cloud/codecs/openai/request.rs`](../../platform/ai/src/cloud/codecs/openai/request.rs) | 162 | Cloud provider transport codec and streaming protocol client | `create_request, create_request_with_options, create_request_for_model_with_options` |
| [`platform/ai/src/cloud/codecs/openai/response.rs`](../../platform/ai/src/cloud/codecs/openai/response.rs) | 279 | Cloud provider transport codec and streaming protocol client | `record_response_metadata, response_to_message, get_usage, get_cost` |
| [`platform/ai/src/cloud/codecs/openai/tools.rs`](../../platform/ai/src/cloud/codecs/openai/tools.rs) | 152 | Cloud provider transport codec and streaming protocol client | `format_tools, validate_tool_schemas` |
| [`platform/ai/src/cloud/codecs/openai/tests/documents.rs`](../../platform/ai/src/cloud/codecs/openai/tests/documents.rs) | 67 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/helpers.rs`](../../platform/ai/src/cloud/codecs/openai/tests/helpers.rs) | 88 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/messages.rs`](../../platform/ai/src/cloud/codecs/openai/tests/messages.rs) | 466 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/mod.rs`](../../platform/ai/src/cloud/codecs/openai/tests/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/codecs/openai/tests/reasoning_merging.rs`](../../platform/ai/src/cloud/codecs/openai/tests/reasoning_merging.rs) | 106 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/reasoning_messages.rs`](../../platform/ai/src/cloud/codecs/openai/tests/reasoning_messages.rs) | 514 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/reasoning_streaming.rs`](../../platform/ai/src/cloud/codecs/openai/tests/reasoning_streaming.rs) | 168 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/request.rs`](../../platform/ai/src/cloud/codecs/openai/tests/request.rs) | 320 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/response.rs`](../../platform/ai/src/cloud/codecs/openai/tests/response.rs) | 428 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/streaming.rs`](../../platform/ai/src/cloud/codecs/openai/tests/streaming.rs) | 454 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/streaming_frames.rs`](../../platform/ai/src/cloud/codecs/openai/tests/streaming_frames.rs) | 251 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/streaming_tool_calls.rs`](../../platform/ai/src/cloud/codecs/openai/tests/streaming_tool_calls.rs) | 298 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/tools.rs`](../../platform/ai/src/cloud/codecs/openai/tests/tools.rs) | 252 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/usage.rs`](../../platform/ai/src/cloud/codecs/openai/tests/usage.rs) | 104 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/tests/xai_and_preservation.rs`](../../platform/ai/src/cloud/codecs/openai/tests/xai_and_preservation.rs) | 422 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/streaming/mod.rs`](../../platform/ai/src/cloud/codecs/openai/streaming/mod.rs) | 469 | Submodule root: aggregates and re-exports sibling modules | `response_to_streaming_message` |
| [`platform/ai/src/cloud/codecs/openai/streaming/parser.rs`](../../platform/ai/src/cloud/codecs/openai/streaming/parser.rs) | 145 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/codecs/openai/streaming/types.rs`](../../platform/ai/src/cloud/codecs/openai/streaming/types.rs) | 139 | Cloud provider transport codec and streaming protocol client | `deserialize_null_default_string, describe_json_value` |
| [`platform/ai/src/cloud/transport/mod.rs`](../../platform/ai/src/cloud/transport/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/transport/oauth_tokens.rs`](../../platform/ai/src/cloud/transport/oauth_tokens.rs) | 130 | Cloud provider transport codec and streaming protocol client | `extract_chatgpt_account_id, copilot_session_token` |
| [`platform/ai/src/cloud/transport/openai_compatible.rs`](../../platform/ai/src/cloud/transport/openai_compatible.rs) | 455 | Cloud provider transport codec and streaming protocol client | `OpenAiCompatibleProvider, new, with_supports_streaming, stream_for_model, stream_openai_compat, stream_responses_compat` |
| [`platform/ai/src/cloud/transport/api_client/builder.rs`](../../platform/ai/src/cloud/transport/api_client/builder.rs) | 112 | Cloud provider transport codec and streaming protocol client | `ApiRequestBuilder, header, headers, model_headers, streaming, api_post` |
| [`platform/ai/src/cloud/transport/api_client/client.rs`](../../platform/ai/src/cloud/transport/api_client/client.rs) | 272 | Cloud provider transport codec and streaming protocol client | `new_with_tls, with_timeout_and_tls, host, timeout, default_header, with_headers` |
| [`platform/ai/src/cloud/transport/api_client/mod.rs`](../../platform/ai/src/cloud/transport/api_client/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/transport/api_client/tls.rs`](../../platform/ai/src/cloud/transport/api_client/tls.rs) | 290 | Cloud provider transport codec and streaming protocol client | `TlsCertKeyPair, TlsConfig, new, with_client_cert_and_key, with_ca_cert, is_configured` |
| [`platform/ai/src/cloud/transport/api_client/types.rs`](../../platform/ai/src/cloud/transport/api_client/types.rs) | 95 | Cloud provider transport codec and streaming protocol client | `AuthMethod, AuthProvider, ApiResponse, ApiClient, from_response` |
| [`platform/ai/src/cloud/transport/api_client/tests/client_tests.rs`](../../platform/ai/src/cloud/transport/api_client/tests/client_tests.rs) | 350 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/transport/api_client/tests/mod.rs`](../../platform/ai/src/cloud/transport/api_client/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/transport/http_status/error_mapper.rs`](../../platform/ai/src/cloud/transport/http_status/error_mapper.rs) | 220 | Cloud provider transport codec and streaming protocol client | `sanitize_url, is_context_length_exceeded, is_context_length_exceeded_message, map_http_error_to_provider_error` |
| [`platform/ai/src/cloud/transport/http_status/mod.rs`](../../platform/ai/src/cloud/transport/http_status/mod.rs) | 27 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/transport/http_status/response_handler.rs`](../../platform/ai/src/cloud/transport/http_status/response_handler.rs) | 136 | Cloud provider transport codec and streaming protocol client | `ResponseDeadline, set_response_deadline, send_bounded, read_response_body_with_limit, read_error_body, read_json_response` |
| [`platform/ai/src/cloud/transport/http_status/retry.rs`](../../platform/ai/src/cloud/transport/http_status/retry.rs) | 83 | Cloud provider transport codec and streaming protocol client | `extract_retry_after, duration_from_finite_secs, parse_retry_after_header, parse_http_date` |
| [`platform/ai/src/cloud/transport/http_status/tests.rs`](../../platform/ai/src/cloud/transport/http_status/tests.rs) | 365 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openrouter/gemini_compat.rs`](../../platform/ai/src/cloud/openrouter/gemini_compat.rs) | 273 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openrouter/mod.rs`](../../platform/ai/src/cloud/openrouter/mod.rs) | 22 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/openrouter/provider.rs`](../../platform/ai/src/cloud/openrouter/provider.rs) | 272 | Cloud provider transport codec and streaming protocol client | `OpenRouterProvider, new` |
| [`platform/ai/src/cloud/openrouter/tests/gemini_escape_basic.rs`](../../platform/ai/src/cloud/openrouter/tests/gemini_escape_basic.rs) | 375 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openrouter/tests/gemini_escape_collisions.rs`](../../platform/ai/src/cloud/openrouter/tests/gemini_escape_collisions.rs) | 324 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openrouter/tests/mod.rs`](../../platform/ai/src/cloud/openrouter/tests/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/openrouter/tests/provider_tests.rs`](../../platform/ai/src/cloud/openrouter/tests/provider_tests.rs) | 105 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/google/mod.rs`](../../platform/ai/src/cloud/google/mod.rs) | 215 | Submodule root: aggregates and re-exports sibling modules | `GoogleProvider, new` |
| [`platform/ai/src/cloud/databricks/auth.rs`](../../platform/ai/src/cloud/databricks/auth.rs) | 93 | Cloud provider transport codec and streaming protocol client | `DatabricksAuth, DatabricksAuthProvider, oauth, token` |
| [`platform/ai/src/cloud/databricks/endpoint.rs`](../../platform/ai/src/cloud/databricks/endpoint.rs) | 310 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/databricks/mod.rs`](../../platform/ai/src/cloud/databricks/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/databricks/provider.rs`](../../platform/ai/src/cloud/databricks/provider.rs) | 491 | Cloud provider transport codec and streaming protocol client | `cleanup, new, load_retry_config` |
| [`platform/ai/src/cloud/databricks/types.rs`](../../platform/ai/src/cloud/databricks/types.rs) | 82 | Cloud provider transport codec and streaming protocol client | `DatabricksEndpointInfo, DatabricksUpstreamModel, CachedDatabricksEndpointInfo, EndpointMetadataLookup, DatabricksProvider, applies_to` |
| [`platform/ai/src/cloud/databricks/tests/endpoint_tests.rs`](../../platform/ai/src/cloud/databricks/tests/endpoint_tests.rs) | 271 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/databricks/tests/mod.rs`](../../platform/ai/src/cloud/databricks/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/snowflake/mod.rs`](../../platform/ai/src/cloud/snowflake/mod.rs) | 397 | Submodule root: aggregates and re-exports sibling modules | `SnowflakeAuth, SnowflakeProvider, token, new` |
| [`platform/ai/src/cloud/realtime/browser_live_transport.rs`](../../platform/ai/src/cloud/realtime/browser_live_transport.rs) | 103 | Cloud provider transport codec and streaming protocol client | `BrowserLiveOutbound, BrowserLiveTransport, recv, new, take_outbound, push_incoming` |
| [`platform/ai/src/cloud/realtime/live.rs`](../../platform/ai/src/cloud/realtime/live.rs) | 298 | Cloud provider transport codec and streaming protocol client | `LiveTransport, LiveProtocol, LiveSessionEndReason, LiveSessionEvent, LiveSession, connect` |
| [`platform/ai/src/cloud/realtime/live_transport_websocket.rs`](../../platform/ai/src/cloud/realtime/live_transport_websocket.rs) | 86 | Cloud provider transport codec and streaming protocol client | `WebSocketLiveTransport, connect` |
| [`platform/ai/src/cloud/realtime/mod.rs`](../../platform/ai/src/cloud/realtime/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/realtime/openai_live/client.rs`](../../platform/ai/src/cloud/realtime/openai_live/client.rs) | 295 | Cloud provider transport codec and streaming protocol client | `OpenAiLiveClient, OpenAiLiveExistingSessionConnector, OpenAiLiveWebSocketRequest, OpenAiLiveWebSocketConnector, OpenAiLiveWebRtcConnector, OpenAiLiveWebRtcRequest` |
| [`platform/ai/src/cloud/realtime/openai_live/mod.rs`](../../platform/ai/src/cloud/realtime/openai_live/mod.rs) | 29 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/realtime/openai_live/protocol.rs`](../../platform/ai/src/cloud/realtime/openai_live/protocol.rs) | 276 | Cloud provider transport codec and streaming protocol client | `OpenAiLiveProtocol` |
| [`platform/ai/src/cloud/realtime/openai_live/session.rs`](../../platform/ai/src/cloud/realtime/openai_live/session.rs) | 116 | Cloud provider transport codec and streaming protocol client | `StartingOpenAiLiveSession, ConnectedOpenAiLiveSession, recv, session_id, send, close` |
| [`platform/ai/src/cloud/realtime/openai_live/types.rs`](../../platform/ai/src/cloud/realtime/openai_live/types.rs) | 189 | Cloud provider transport codec and streaming protocol client | `OpenAiLiveSessionConfig, OpenAiLiveMessage, OpenAiLiveMessageRole, OpenAiLiveContextChannel, OpenAiLiveContext, OpenAiLiveSessionId` |
| [`platform/ai/src/cloud/realtime/openai_live/tests/client_tests.rs`](../../platform/ai/src/cloud/realtime/openai_live/tests/client_tests.rs) | 215 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/realtime/openai_live/tests/mod.rs`](../../platform/ai/src/cloud/realtime/openai_live/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/realtime/openai_live/tests/protocol_tests.rs`](../../platform/ai/src/cloud/realtime/openai_live/tests/protocol_tests.rs) | 225 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/azure_foundry/mod.rs`](../../platform/ai/src/cloud/azure_foundry/mod.rs) | 18 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/azure_foundry/provider.rs`](../../platform/ai/src/cloud/azure_foundry/provider.rs) | 418 | Cloud provider transport codec and streaming protocol client | `AzureFoundryProvider, create` |
| [`platform/ai/src/cloud/azure_foundry/types.rs`](../../platform/ai/src/cloud/azure_foundry/types.rs) | 206 | Cloud provider transport codec and streaming protocol client | `EndpointKind, ModelPublisher, endpoint_kind, is_project_endpoint` |
| [`platform/ai/src/cloud/azure_foundry/tests/deployments_and_streaming.rs`](../../platform/ai/src/cloud/azure_foundry/tests/deployments_and_streaming.rs) | 320 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/azure_foundry/tests/discovery_and_limits.rs`](../../platform/ai/src/cloud/azure_foundry/tests/discovery_and_limits.rs) | 149 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/azure_foundry/tests/helpers.rs`](../../platform/ai/src/cloud/azure_foundry/tests/helpers.rs) | 61 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/azure_foundry/tests/mod.rs`](../../platform/ai/src/cloud/azure_foundry/tests/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/azure_foundry/tests/routing_matrix.rs`](../../platform/ai/src/cloud/azure_foundry/tests/routing_matrix.rs) | 276 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/declarative/builder.rs`](../../platform/ai/src/cloud/declarative/builder.rs) | 112 | Cloud provider transport codec and streaming protocol client | `expand_env_vars, resolve_config, deserialize_provider_config, config_from_json, load_custom_providers, from_json` |
| [`platform/ai/src/cloud/declarative/macros.rs`](../../platform/ai/src/cloud/declarative/macros.rs) | 44 | Cloud provider transport codec and streaming protocol client | `create` |
| [`platform/ai/src/cloud/declarative/mod.rs`](../../platform/ai/src/cloud/declarative/mod.rs) | 83 | Submodule root: aggregates and re-exports sibling modules | `fixed_provider_configs, fixed_provider_config_entries` |
| [`platform/ai/src/cloud/declarative/tests.rs`](../../platform/ai/src/cloud/declarative/tests.rs) | 305 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/declarative/types.rs`](../../platform/ai/src/cloud/declarative/types.rs) | 196 | Cloud provider transport codec and streaming protocol client | `EnvVarConfig, ProviderEngine, AuthConfig, DeclarativeProviderConfig, KeyResolver, EnvKeyResolver` |
| [`platform/ai/src/cloud/anthropic/declarative.rs`](../../platform/ai/src/cloud/anthropic/declarative.rs) | 110 | Cloud provider transport codec and streaming protocol client | `from_declarative_config` |
| [`platform/ai/src/cloud/anthropic/mod.rs`](../../platform/ai/src/cloud/anthropic/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/anthropic/provider.rs`](../../platform/ai/src/cloud/anthropic/provider.rs) | 189 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/anthropic/stream.rs`](../../platform/ai/src/cloud/anthropic/stream.rs) | 162 | Cloud provider transport codec and streaming protocol client | `stream_for_model` |
| [`platform/ai/src/cloud/anthropic/tests.rs`](../../platform/ai/src/cloud/anthropic/tests.rs) | 304 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/anthropic/types.rs`](../../platform/ai/src/cloud/anthropic/types.rs) | 125 | Cloud provider transport codec and streaming protocol client | `AnthropicProvider, AnthropicProviderBuilder, new, api_client, map_api_client, try_map_api_client` |
| [`platform/ai/src/cloud/fake/mod.rs`](../../platform/ai/src/cloud/fake/mod.rs) | 141 | Submodule root: aggregates and re-exports sibling modules | `FakeProvider, new, with_default_text, enqueue_response` |
| [`platform/ai/src/cloud/ollama/builder.rs`](../../platform/ai/src/cloud/ollama/builder.rs) | 177 | Cloud provider transport codec and streaming protocol client | `OllamaProviderBuilder, new, api_client, map_api_client, try_map_api_client, name` |
| [`platform/ai/src/cloud/ollama/mod.rs`](../../platform/ai/src/cloud/ollama/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/ollama/options.rs`](../../platform/ai/src/cloud/ollama/options.rs) | 94 | Cloud provider transport codec and streaming protocol client | `OllamaOptions` |
| [`platform/ai/src/cloud/ollama/provider.rs`](../../platform/ai/src/cloud/ollama/provider.rs) | 243 | Cloud provider transport codec and streaming protocol client | `OllamaProvider, new, with_options, fetch_ollama_model_names` |
| [`platform/ai/src/cloud/ollama/streaming.rs`](../../platform/ai/src/cloud/ollama/streaming.rs) | 79 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/ollama/tests.rs`](../../platform/ai/src/cloud/ollama/tests.rs) | 230 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openai/builder.rs`](../../platform/ai/src/cloud/openai/builder.rs) | 296 | Cloud provider transport codec and streaming protocol client | `OpenAiProviderBuilder, ensure_url_scheme, parse_openai_base_url, new, api_client, map_api_client` |
| [`platform/ai/src/cloud/openai/compat.rs`](../../platform/ai/src/cloud/openai/compat.rs) | 122 | Cloud provider transport codec and streaming protocol client | `meta_reasoning_effort, sanitize_request_for_compat, apply_declared_request_params` |
| [`platform/ai/src/cloud/openai/context_limit.rs`](../../platform/ai/src/cloud/openai/context_limit.rs) | 65 | Cloud provider transport codec and streaming protocol client | `CachedContextLimit, value, parse_model_ids, parse_n_ctx_from_models` |
| [`platform/ai/src/cloud/openai/mod.rs`](../../platform/ai/src/cloud/openai/mod.rs) | 29 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/openai/provider.rs`](../../platform/ai/src/cloud/openai/provider.rs) | 542 | Cloud provider transport codec and streaming protocol client | `OpenAiProvider, stream_for_model, new` |
| [`platform/ai/src/cloud/openai/tests/helpers.rs`](../../platform/ai/src/cloud/openai/tests/helpers.rs) | 107 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openai/tests/mod.rs`](../../platform/ai/src/cloud/openai/tests/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/cloud/openai/tests/models_and_context.rs`](../../platform/ai/src/cloud/openai/tests/models_and_context.rs) | 303 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openai/tests/routing_and_config.rs`](../../platform/ai/src/cloud/openai/tests/routing_and_config.rs) | 221 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/cloud/openai/tests/sanitization_and_streaming.rs`](../../platform/ai/src/cloud/openai/tests/sanitization_and_streaming.rs) | 340 | Cloud provider transport codec and streaming protocol client | None |
| [`platform/ai/src/routing/availability.rs`](../../platform/ai/src/routing/availability.rs) | 338 | Dynamic model routing, streaming pipelines and capability probes | `ModelUnavailable, ModelAvailability, snapshot, check, refresh, quota_recovered` |
| [`platform/ai/src/routing/fallback.rs`](../../platform/ai/src/routing/fallback.rs) | 255 | Dynamic model routing, streaming pipelines and capability probes | `ProviderCandidate, FallbackErrorClass, CooldownTracker, FallbackRouter, classify_fallback_error, new` |
| [`platform/ai/src/routing/mod.rs`](../../platform/ai/src/routing/mod.rs) | 10 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/streaming/legacy_decoder.rs`](../../platform/ai/src/routing/streaming/legacy_decoder.rs) | 368 | Dynamic model routing, streaming pipelines and capability probes | `unary_completion` |
| [`platform/ai/src/routing/streaming/mod.rs`](../../platform/ai/src/routing/streaming/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/streaming/stream_observer.rs`](../../platform/ai/src/routing/streaming/stream_observer.rs) | 383 | Dynamic model routing, streaming pipelines and capability probes | `stream_response_observed, stream_response, stream_response_with_idle_timeout, stream_turn_response_observed` |
| [`platform/ai/src/routing/streaming/turn_decoder.rs`](../../platform/ai/src/routing/streaming/turn_decoder.rs) | 459 | Dynamic model routing, streaming pipelines and capability probes | `ModelTurnStreamDecoder, decode` |
| [`platform/ai/src/routing/streaming/types.rs`](../../platform/ai/src/routing/streaming/types.rs) | 44 | Dynamic model routing, streaming pipelines and capability probes | `Wire` |
| [`platform/ai/src/routing/streaming/tests/helpers.rs`](../../platform/ai/src/routing/streaming/tests/helpers.rs) | 32 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/streaming/tests/legacy_tests.rs`](../../platform/ai/src/routing/streaming/tests/legacy_tests.rs) | 131 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/streaming/tests/mod.rs`](../../platform/ai/src/routing/streaming/tests/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/streaming/tests/turn_tests.rs`](../../platform/ai/src/routing/streaming/tests/turn_tests.rs) | 93 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/probe/cloud_probes.rs`](../../platform/ai/src/routing/probe/cloud_probes.rs) | 498 | Cloud provider transport codec and streaming protocol client | `probe_endpoint_models, parse_openai_models, parse_openai_models_for_transport, probe_oauth_models, probe_chatgpt_codex_endpoint, parse_chatgpt_codex_models` |
| [`platform/ai/src/routing/probe/metadata.rs`](../../platform/ai/src/routing/probe/metadata.rs) | 152 | Dynamic model routing, streaming pipelines and capability probes | `codex_user_agent, conversation_model_is_visible, model_is_selectable, advertised_efforts, advertised_default_effort, verified_catalog_key` |
| [`platform/ai/src/routing/probe/mod.rs`](../../platform/ai/src/routing/probe/mod.rs) | 27 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/probe/ollama.rs`](../../platform/ai/src/routing/probe/ollama.rs) | 100 | Dynamic model routing, streaming pipelines and capability probes | `ollama_root, parse_ollama_tags, parse_ollama_show_context` |
| [`platform/ai/src/routing/probe/quota.rs`](../../platform/ai/src/routing/probe/quota.rs) | 283 | Dynamic model routing, streaming pipelines and capability probes | `probe_account_quota, parse_account_quota` |
| [`platform/ai/src/routing/probe/tests/mod.rs`](../../platform/ai/src/routing/probe/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/probe/tests/models_and_parsing.rs`](../../platform/ai/src/routing/probe/tests/models_and_parsing.rs) | 257 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/probe/tests/quota_and_copilot.rs`](../../platform/ai/src/routing/probe/tests/quota_and_copilot.rs) | 75 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/catalog.rs`](../../platform/ai/src/routing/router/catalog.rs) | 141 | Model catalog, pricing rates, capabilities and provider endpoints | `OpenAiCompatibleEndpoint, find_by_service, find_by_model, strip_model_prefix` |
| [`platform/ai/src/routing/router/classification.rs`](../../platform/ai/src/routing/router/classification.rs) | 163 | Dynamic model routing, streaming pipelines and capability probes | `classify_provider, format_provider_error, parse_codex_sse_stream` |
| [`platform/ai/src/routing/router/credentials.rs`](../../platform/ai/src/routing/router/credentials.rs) | 67 | Dynamic model routing, streaming pipelines and capability probes | `CredentialResolver, EnvCredentialResolver` |
| [`platform/ai/src/routing/router/execution_backends.rs`](../../platform/ai/src/routing/router/execution_backends.rs) | 364 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/generate_dispatch.rs`](../../platform/ai/src/routing/router/generate_dispatch.rs) | 291 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/mod.rs`](../../platform/ai/src/routing/router/mod.rs) | 22 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/router/provider.rs`](../../platform/ai/src/routing/router/provider.rs) | 260 | Dynamic model routing, streaming pipelines and capability probes | `RouterModelProvider, new, with_native_local, with_env_fallback, classify_provider` |
| [`platform/ai/src/routing/router/response_handler.rs`](../../platform/ai/src/routing/router/response_handler.rs) | 138 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/stream_dispatch.rs`](../../platform/ai/src/routing/router/stream_dispatch.rs) | 340 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/wire_types.rs`](../../platform/ai/src/routing/router/wire_types.rs) | 89 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/tests/helpers.rs`](../../platform/ai/src/routing/router/tests/helpers.rs) | 35 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/tests/mod.rs`](../../platform/ai/src/routing/router/tests/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/ai/src/routing/router/tests/routing_and_models.rs`](../../platform/ai/src/routing/router/tests/routing_and_models.rs) | 394 | Dynamic model routing, streaming pipelines and capability probes | None |
| [`platform/ai/src/routing/router/tests/streaming_and_failover.rs`](../../platform/ai/src/routing/router/tests/streaming_and_failover.rs) | 433 | Dynamic model routing, streaming pipelines and capability probes | None |

### 3.5. Crate `custos-runtime` — Layer 2: Execution, Agent Supervision & OI Brain

- **Đường dẫn thư mục:** `platform/runtime`
- **Chủ sở hữu chính (Owner):** **Vinh + Vĩ**
- **Quy tắc ranh giới:** Không reqwest, không std::process::Command, không gọi trực tiếp custos-persistence (phải đi qua ports).
- **Tổng số tệp:** 111 tệp (110 tệp Rust) | **Tổng số dòng mã:** 22,164 dòng Rust.
- **Mô tả chức năng:** 6 trụ cột thực thi: Agent Loop (SupervisedAgentRunner, 0-Token FastPathFilter), Context Compiler, Environment Coordinator, OI Brain (T0-T6 Topologies, T5 Lead-Worker, Pareto Selector), Session Bridge và Workflow TaskRuntime.

#### Danh mục các file bên trong `platform/runtime`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/runtime/Cargo.toml`](../../platform/runtime/Cargo.toml) | 44 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/runtime/src/lib.rs`](../../platform/runtime/src/lib.rs) | 33 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/runtime/src/context/compiler.rs`](../../platform/runtime/src/context/compiler.rs) | 438 | Context compiler, prompt construction and token budget manager | `HeadroomPolicy, CompiledSessionHistory, SourceDocument, TokenAwareContextCompiler, new, max_prompt_tokens` |
| [`platform/runtime/src/context/mod.rs`](../../platform/runtime/src/context/mod.rs) | 21 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/context/token_counter.rs`](../../platform/runtime/src/context/token_counter.rs) | 322 | Context compiler, prompt construction and token budget manager | `TokenCounter, new, count_tokens, count_tokens_for_tools, count_chat_tokens, count_everything` |
| [`platform/runtime/src/context/traits.rs`](../../platform/runtime/src/context/traits.rs) | 13 | Context compiler, prompt construction and token budget manager | `ContextSlice, ContextBuilder` |
| [`platform/runtime/src/context/memory/mod.rs`](../../platform/runtime/src/context/memory/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/context/memory/store.rs`](../../platform/runtime/src/context/memory/store.rs) | 319 | Context compiler, prompt construction and token budget manager | `InMemoryMemoryStore, new, remember_scoped, recall_scoped` |
| [`platform/runtime/src/context/memory/traits.rs`](../../platform/runtime/src/context/memory/traits.rs) | 18 | Context compiler, prompt construction and token budget manager | `MemoryTier, MemoryStore` |
| [`platform/runtime/src/context/pack/anchor.rs`](../../platform/runtime/src/context/pack/anchor.rs) | 95 | Context compiler, prompt construction and token budget manager | `AnchorRetriever, retrieve_anchors, target_anchor_function, another_anchor_match` |
| [`platform/runtime/src/context/pack/compaction.rs`](../../platform/runtime/src/context/pack/compaction.rs) | 168 | Context compiler, prompt construction and token budget manager | `ProgressiveCompactor, estimate_tokens, compact` |
| [`platform/runtime/src/context/pack/dedup.rs`](../../platform/runtime/src/context/pack/dedup.rs) | 73 | Context compiler, prompt construction and token budget manager | `HashDeduplicator, deduplicate, common_helper, unique_function` |
| [`platform/runtime/src/context/pack/mod.rs`](../../platform/runtime/src/context/pack/mod.rs) | 393 | Submodule root: aggregates and re-exports sibling modules | `CompileContextRequest, ContextCompiler, new, with_sandbox, with_files_port, with_memory_files` |
| [`platform/runtime/src/context/pack/redaction.rs`](../../platform/runtime/src/context/pack/redaction.rs) | 175 | Context compiler, prompt construction and token budget manager | `SecretRedactor, redact, shannon_entropy, is_high_entropy_token` |
| [`platform/runtime/src/context/pack/structural.rs`](../../platform/runtime/src/context/pack/structural.rs) | 160 | Context compiler, prompt construction and token budget manager | `StructuralExtractor, WorkerConfig, extract_outline, new` |
| [`platform/runtime/src/agent/fast_path.rs`](../../platform/runtime/src/agent/fast_path.rs) | 253 | 0-Token FastPathFilter: sub-millisecond, zero-cost matching for deterministic system tasks (help, ping, status, git) | `FastPathResult, FastPathCommand, FastPathFilter, evaluate, is_fast_path` |
| [`platform/runtime/src/agent/mod.rs`](../../platform/runtime/src/agent/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/agent/runner.rs`](../../platform/runtime/src/agent/runner.rs) | 265 | SupervisedAgentRunner decorator: wraps agent execution with StepSpan telemetry, SupervisionTraceSink, and SupervisedObserver | `SupervisedAgentRunner, new, with_workbench, inner` |
| [`platform/runtime/src/agent/runtime_port/loop_turn.rs`](../../platform/runtime/src/agent/runtime_port/loop_turn.rs) | 331 | Governed agent execution loop, supervision runner and fast path filters | None |
| [`platform/runtime/src/agent/runtime_port/mod.rs`](../../platform/runtime/src/agent/runtime_port/mod.rs) | 88 | Submodule root: aggregates and re-exports sibling modules | `GovernedAgentRuntime, new, with_tool_executor, with_max_iterations, with_tool_output_cap, register_tool` |
| [`platform/runtime/src/agent/runtime_port/tool.rs`](../../platform/runtime/src/agent/runtime_port/tool.rs) | 68 | Governed agent execution loop, supervision runner and fast path filters | `InvocationContext, ToolResult, GovernedToolExecutor` |
| [`platform/runtime/src/agent/runtime_port/writer.rs`](../../platform/runtime/src/agent/runtime_port/writer.rs) | 57 | Governed agent execution loop, supervision runner and fast path filters | `LimitedWriter, truncate_to_char_boundary, new, capped, written, into_inner` |
| [`platform/runtime/src/agent/runtime_port/tests/mod.rs`](../../platform/runtime/src/agent/runtime_port/tests/mod.rs) | 337 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/agent/runtime_port/tests/steering_and_writer.rs`](../../platform/runtime/src/agent/runtime_port/tests/steering_and_writer.rs) | 401 | Governed agent execution loop, supervision runner and fast path filters | None |
| [`platform/runtime/src/agent/s1/calibration.rs`](../../platform/runtime/src/agent/s1/calibration.rs) | 145 | Governed agent execution loop, supervision runner and fast path filters | `ConfidenceCalibrator, calculate_brier_score, calculate_ece, new, with_threshold, should_abstain` |
| [`platform/runtime/src/agent/s1/judge.rs`](../../platform/runtime/src/agent/s1/judge.rs) | 278 | Governed agent execution loop, supervision runner and fast path filters | `S1Judge, new, with_threshold` |
| [`platform/runtime/src/agent/s1/micro_executor.rs`](../../platform/runtime/src/agent/s1/micro_executor.rs) | 340 | Governed agent execution loop, supervision runner and fast path filters | `MicroTask, MicroExecutionReceipt, MicroExecutor, new, with_timeout, execute` |
| [`platform/runtime/src/agent/s1/mod.rs`](../../platform/runtime/src/agent/s1/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/agent/s1/scout.rs`](../../platform/runtime/src/agent/s1/scout.rs) | 177 | Governed agent execution loop, supervision runner and fast path filters | `ProjectEcosystem, ScoutReport, S1Scout, new, write_set, inspect` |
| [`platform/runtime/src/agent/skills/mod.rs`](../../platform/runtime/src/agent/skills/mod.rs) | 150 | Submodule root: aggregates and re-exports sibling modules | `SkillRegistry, new, register, resolve_tool_set, get, contains` |
| [`platform/runtime/src/workflow/contracts.rs`](../../platform/runtime/src/workflow/contracts.rs) | 115 | TaskRuntime workflow execution, DAG evaluation and step supervision | `ExecutionContext, StepObservation, new, from_output` |
| [`platform/runtime/src/workflow/graph_runtime.rs`](../../platform/runtime/src/workflow/graph_runtime.rs) | 465 | TaskRuntime workflow execution, DAG evaluation and step supervision | `GraphExecutionReport, StepExecutor, GraphRuntime, new, graph, execute_all` |
| [`platform/runtime/src/workflow/live.rs`](../../platform/runtime/src/workflow/live.rs) | 424 | TaskRuntime workflow execution, DAG evaluation and step supervision | `LiveRuns, LiveObserver, register, emit, replay, cancel` |
| [`platform/runtime/src/workflow/mod.rs`](../../platform/runtime/src/workflow/mod.rs) | 24 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/workflow/revision_loader.rs`](../../platform/runtime/src/workflow/revision_loader.rs) | 45 | TaskRuntime workflow execution, DAG evaluation and step supervision | `RevisionLoader, load_into_dag` |
| [`platform/runtime/src/workflow/scheduler.rs`](../../platform/runtime/src/workflow/scheduler.rs) | 268 | TaskRuntime workflow execution, DAG evaluation and step supervision | `ScheduledJobId, JobRecurrence, JobStatus, ScheduledJob, WorkflowScheduler, new` |
| [`platform/runtime/src/workflow/dag/mod.rs`](../../platform/runtime/src/workflow/dag/mod.rs) | 402 | Submodule root: aggregates and re-exports sibling modules | `NodeStatus, DagNode, ExecutionWave, DagGraph, is_terminal, new` |
| [`platform/runtime/src/workflow/dag/tests.rs`](../../platform/runtime/src/workflow/dag/tests.rs) | 224 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/dispatcher/mod.rs`](../../platform/runtime/src/workflow/dispatcher/mod.rs) | 340 | Submodule root: aggregates and re-exports sibling modules | `WorkflowDispatcher, new, with_run_store, run_store, restore_from_active_claims, restore_from_store` |
| [`platform/runtime/src/workflow/dispatcher/tests.rs`](../../platform/runtime/src/workflow/dispatcher/tests.rs) | 328 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/worker_executor/mod.rs`](../../platform/runtime/src/workflow/worker_executor/mod.rs) | 326 | Submodule root: aggregates and re-exports sibling modules | `WorkerExecutor, new, with_kernel, with_model, with_harness, with_sandbox` |
| [`platform/runtime/src/workflow/worker_executor/tests/mod.rs`](../../platform/runtime/src/workflow/worker_executor/tests/mod.rs) | 345 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/workflow/worker_executor/tests/outbox.rs`](../../platform/runtime/src/workflow/worker_executor/tests/outbox.rs) | 401 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/mod.rs`](../../platform/runtime/src/workflow/task_runtime/mod.rs) | 407 | Submodule root: aggregates and re-exports sibling modules | `TaskRuntime, new, with_ports, with_journal_manager, with_kernel, with_model` |
| [`platform/runtime/src/workflow/task_runtime/port.rs`](../../platform/runtime/src/workflow/task_runtime/port.rs) | 414 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/tests/approval_conflict.rs`](../../platform/runtime/src/workflow/task_runtime/tests/approval_conflict.rs) | 301 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/tests/background.rs`](../../platform/runtime/src/workflow/task_runtime/tests/background.rs) | 193 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/tests/mod.rs`](../../platform/runtime/src/workflow/task_runtime/tests/mod.rs) | 397 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/workflow/task_runtime/tests/turns.rs`](../../platform/runtime/src/workflow/task_runtime/tests/turns.rs) | 286 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/execution/candidate_race.rs`](../../platform/runtime/src/workflow/task_runtime/execution/candidate_race.rs) | 201 | TaskRuntime workflow execution, DAG evaluation and step supervision | `execute_candidate_race` |
| [`platform/runtime/src/workflow/task_runtime/execution/mod.rs`](../../platform/runtime/src/workflow/task_runtime/execution/mod.rs) | 305 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/workflow/task_runtime/execution/model.rs`](../../platform/runtime/src/workflow/task_runtime/execution/model.rs) | 241 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/execution/native.rs`](../../platform/runtime/src/workflow/task_runtime/execution/native.rs) | 340 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/execution/resume.rs`](../../platform/runtime/src/workflow/task_runtime/execution/resume.rs) | 101 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/task_runtime/execution/runner.rs`](../../platform/runtime/src/workflow/task_runtime/execution/runner.rs) | 369 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/supervision/coordinator.rs`](../../platform/runtime/src/workflow/supervision/coordinator.rs) | 276 | TaskRuntime workflow execution, DAG evaluation and step supervision | `SupervisionCoordinator, new, profiles, ensure_started` |
| [`platform/runtime/src/workflow/supervision/mod.rs`](../../platform/runtime/src/workflow/supervision/mod.rs) | 128 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/workflow/supervision/processor.rs`](../../platform/runtime/src/workflow/supervision/processor.rs) | 383 | TaskRuntime workflow execution, DAG evaluation and step supervision | None |
| [`platform/runtime/src/workflow/supervision/profile_registry.rs`](../../platform/runtime/src/workflow/supervision/profile_registry.rs) | 69 | TaskRuntime workflow execution, DAG evaluation and step supervision | `SupervisionProfileRegistry, new, register_profile, unregister_profile, resolve, list_profiles` |
| [`platform/runtime/src/workflow/supervision/redactor.rs`](../../platform/runtime/src/workflow/supervision/redactor.rs) | 121 | TaskRuntime workflow execution, DAG evaluation and step supervision | `PayloadRedactor, redact_text, redact_json` |
| [`platform/runtime/src/environment/coordinator.rs`](../../platform/runtime/src/environment/coordinator.rs) | 278 | Resource coordination for workspaces, terminals and compute nodes | `EnvironmentCoordinator, new, get_handle, list_handles, terminal_port` |
| [`platform/runtime/src/environment/coordinator_tests.rs`](../../platform/runtime/src/environment/coordinator_tests.rs) | 386 | Resource coordination for workspaces, terminals and compute nodes | None |
| [`platform/runtime/src/environment/mod.rs`](../../platform/runtime/src/environment/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/environment/workspace/coordinator.rs`](../../platform/runtime/src/environment/workspace/coordinator.rs) | 345 | Resource coordination for workspaces, terminals and compute nodes | `CreateWorkspaceRequest, WorkspaceCoordinator, new, create_workspace, get_workspace, list_workspaces` |
| [`platform/runtime/src/environment/workspace/coordinator_tests.rs`](../../platform/runtime/src/environment/workspace/coordinator_tests.rs) | 273 | Resource coordination for workspaces, terminals and compute nodes | None |
| [`platform/runtime/src/environment/workspace/lease.rs`](../../platform/runtime/src/environment/workspace/lease.rs) | 409 | Resource coordination for workspaces, terminals and compute nodes | `WorkspaceLease, NoopGitPort, WorkspaceLeaseManager, lease_id, lease_path, path` |
| [`platform/runtime/src/environment/workspace/mod.rs`](../../platform/runtime/src/environment/workspace/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/environment/terminal/buffer.rs`](../../platform/runtime/src/environment/terminal/buffer.rs) | 214 | Resource coordination for workspaces, terminals and compute nodes | `BoundedTerminalBuffer, new, push, mark_eof, is_eof, current_seq` |
| [`platform/runtime/src/environment/terminal/coordinator.rs`](../../platform/runtime/src/environment/terminal/coordinator.rs) | 465 | Resource coordination for workspaces, terminals and compute nodes | `TerminalCoordinator, new, with_max_buffer_bytes` |
| [`platform/runtime/src/environment/terminal/mod.rs`](../../platform/runtime/src/environment/terminal/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/environment/compute/evaluation.rs`](../../platform/runtime/src/environment/compute/evaluation.rs) | 85 | Resource coordination for workspaces, terminals and compute nodes | `ModelEvaluationRunner, new, execute_evaluation` |
| [`platform/runtime/src/environment/compute/mod.rs`](../../platform/runtime/src/environment/compute/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/environment/compute/placement.rs`](../../platform/runtime/src/environment/compute/placement.rs) | 136 | Resource coordination for workspaces, terminals and compute nodes | `OptionalConsumerPlacementEngine, new, evaluate_placement, can_preempt_background_for_interactive` |
| [`platform/runtime/src/environment/compute/relay.rs`](../../platform/runtime/src/environment/compute/relay.rs) | 132 | Resource coordination for workspaces, terminals and compute nodes | `CheckpointRelayCoordinator, new, prepare_continuation_attempt, promote_checkpoint_with_fencing, handoff_crashed_attempt` |
| [`platform/runtime/src/environment/compute/remote.rs`](../../platform/runtime/src/environment/compute/remote.rs) | 249 | Resource coordination for workspaces, terminals and compute nodes | `RemoteExecutionCoordinator, new, capabilities, preflight, admit_and_submit, observe_and_reconcile` |
| [`platform/runtime/src/environment/compute/training.rs`](../../platform/runtime/src/environment/compute/training.rs) | 205 | Resource coordination for workspaces, terminals and compute nodes | `LocalTrainingRunner, new, execute_training_run, cancel_training_run` |
| [`platform/runtime/src/environment/compute/admission/mod.rs`](../../platform/runtime/src/environment/compute/admission/mod.rs) | 436 | Submodule root: aggregates and re-exports sibling modules | `WorkloadAdmissionCoordinator, new, with_custom_devices, with_policy, set_policy, set_workbench_policy` |
| [`platform/runtime/src/environment/compute/admission/tests.rs`](../../platform/runtime/src/environment/compute/admission/tests.rs) | 185 | Resource coordination for workspaces, terminals and compute nodes | None |
| [`platform/runtime/src/environment/notebook/mod.rs`](../../platform/runtime/src/environment/notebook/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/environment/notebook/coordinator/execution.rs`](../../platform/runtime/src/environment/notebook/coordinator/execution.rs) | 255 | Resource coordination for workspaces, terminals and compute nodes | `execute_cell` |
| [`platform/runtime/src/environment/notebook/coordinator/mod.rs`](../../platform/runtime/src/environment/notebook/coordinator/mod.rs) | 289 | Submodule root: aggregates and re-exports sibling modules | `PythonKernelCoordinator, new, with_admission_coordinator, kernel_key, get_status, get_status_scoped` |
| [`platform/runtime/src/environment/notebook/coordinator/tests.rs`](../../platform/runtime/src/environment/notebook/coordinator/tests.rs) | 261 | Resource coordination for workspaces, terminals and compute nodes | None |
| [`platform/runtime/src/oi/admissibility.rs`](../../platform/runtime/src/oi/admissibility.rs) | 50 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `AdmissibilityResult, AdmissibilityEvaluator, evaluate` |
| [`platform/runtime/src/oi/baseline.rs`](../../platform/runtime/src/oi/baseline.rs) | 78 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `DumbOiPlanner, new, plan` |
| [`platform/runtime/src/oi/benchmark.rs`](../../platform/runtime/src/oi/benchmark.rs) | 163 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `BenchmarkComparison, BenchmarkSuiteReport, OiBenchmarkRunner, evaluate_task, default_fixtures, run_suite_with_fixtures` |
| [`platform/runtime/src/oi/candidate_builder.rs`](../../platform/runtime/src/oi/candidate_builder.rs) | 66 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `CandidateBuilder, build` |
| [`platform/runtime/src/oi/compiler.rs`](../../platform/runtime/src/oi/compiler.rs) | 391 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `PlanCompiler, compile, compile_with_placements` |
| [`platform/runtime/src/oi/engine.rs`](../../platform/runtime/src/oi/engine.rs) | 33 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `OiEngine, new` |
| [`platform/runtime/src/oi/estimator.rs`](../../platform/runtime/src/oi/estimator.rs) | 269 | Estimator with Real Model Pricing Catalog (Claude 3.5 Sonnet/Haiku, GPT-4o/mini, Gemini) & Prompt Cache Hit Affinity | `Estimator, estimate, estimate_with_snapshot` |
| [`platform/runtime/src/oi/explain.rs`](../../platform/runtime/src/oi/explain.rs) | 175 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `ExplainService, explain` |
| [`platform/runtime/src/oi/hard_filters.rs`](../../platform/runtime/src/oi/hard_filters.rs) | 184 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `HardFilters, evaluate` |
| [`platform/runtime/src/oi/join_policy.rs`](../../platform/runtime/src/oi/join_policy.rs) | 185 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `JoinPolicy, JoinEvaluation, JoinEvaluator, evaluate` |
| [`platform/runtime/src/oi/mod.rs`](../../platform/runtime/src/oi/mod.rs) | 32 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/oi/replanner.rs`](../../platform/runtime/src/oi/replanner.rs) | 238 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `Replanner, create_brief, apply_delta` |
| [`platform/runtime/src/oi/selector.rs`](../../platform/runtime/src/oi/selector.rs) | 230 | Pareto Multi-Objective Selector: optimizes utility U = w_q*Q - w_c*C - w_t*T with normalized cost/latency & cache-anchored context | `Selector, select` |
| [`platform/runtime/src/oi/snapshot_extractor.rs`](../../platform/runtime/src/oi/snapshot_extractor.rs) | 97 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `DecisionSnapshotExtractor, extract_snapshot` |
| [`platform/runtime/src/oi/write_set.rs`](../../platform/runtime/src/oi/write_set.rs) | 168 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `WriteSetConflictChecker, check_conflicts` |
| [`platform/runtime/src/oi/topologies/mod.rs`](../../platform/runtime/src/oi/topologies/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/oi/topologies/t0_direct.rs`](../../platform/runtime/src/oi/topologies/t0_direct.rs) | 48 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T0DirectCoordinator, execute` |
| [`platform/runtime/src/oi/topologies/t1_single_worker.rs`](../../platform/runtime/src/oi/topologies/t1_single_worker.rs) | 44 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T1SingleWorkerCoordinator, execute` |
| [`platform/runtime/src/oi/topologies/t2_staged_pipeline.rs`](../../platform/runtime/src/oi/topologies/t2_staged_pipeline.rs) | 89 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T2StagedPipelineCoordinator, execute_pipeline` |
| [`platform/runtime/src/oi/topologies/t3_read_fanout.rs`](../../platform/runtime/src/oi/topologies/t3_read_fanout.rs) | 66 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T3ReadFanOutCoordinator, execute_fan_out` |
| [`platform/runtime/src/oi/topologies/t4_worktree_integrator.rs`](../../platform/runtime/src/oi/topologies/t4_worktree_integrator.rs) | 124 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T4WorktreeCoordinator, prepare_and_verify_leases, integrate_branches` |
| [`platform/runtime/src/oi/topologies/t5_lead_worker.rs`](../../platform/runtime/src/oi/topologies/t5_lead_worker.rs) | 271 | T5 Lead-Worker DAG coordinator: Kahn topological sort, cycle detection, dependency validation & coverage threshold synthesis | `T5LeadWorkerCoordinator, OrderedNode, LeadDecomposition, T5SynthesisResult, lead_decompose, verify_dag_execution` |
| [`platform/runtime/src/oi/topologies/t6_repair_loop.rs`](../../platform/runtime/src/oi/topologies/t6_repair_loop.rs) | 55 | Orchestration Intelligence: multi-objective selection, topologies and estimation | `T6RepairLoopCoordinator, evaluate_iteration` |
| [`platform/runtime/src/session/journal.rs`](../../platform/runtime/src/session/journal.rs) | 35 | Session lifecycle bridge, journal persistence and conversation manager | `SessionJournal, new, append, tool_call_count` |
| [`platform/runtime/src/session/manager.rs`](../../platform/runtime/src/session/manager.rs) | 394 | Session lifecycle bridge, journal persistence and conversation manager | `SessionManager, new, with_store, create_session, get_session, get_journal` |
| [`platform/runtime/src/session/mod.rs`](../../platform/runtime/src/session/mod.rs) | 242 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/session/bridge/mod.rs`](../../platform/runtime/src/session/bridge/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/runtime/src/session/bridge/port.rs`](../../platform/runtime/src/session/bridge/port.rs) | 119 | Session lifecycle bridge, journal persistence and conversation manager | `AttachMode, SteerResult, SteerReceipt, BridgeError, BridgePort, TaskObservation` |
| [`platform/runtime/src/session/bridge/service.rs`](../../platform/runtime/src/session/bridge/service.rs) | 242 | Session lifecycle bridge, journal persistence and conversation manager | `BridgeService, new, with_task_runtime` |

### 3.6. Crate `custos-adapters` — Layer 2: External Adapters, Sandbox & Process Guard

- **Đường dẫn thư mục:** `platform/adapters`
- **Chủ sở hữu chính (Owner):** **Trường + Vinh**
- **Quy tắc ranh giới:** Đóng gói toàn bộ I/O ngoại vi; cấm phụ thuộc apps/* hay runtime. Subprocess bắt buộc setpgid(0) và SIGKILL khi drop chống zombie.
- **Tổng số tệp:** 103 tệp (102 tệp Rust) | **Tổng số dòng mã:** 18,383 dòng Rust.
- **Mô tả chức năng:** Bộ chuyển đổi ngoại vi: CliAgentAdapter, ProcessDropGuard chống zombie process, OS sandboxing (Seatbelt/Bubblewrap), MCP/A2A connectors, OAuth PKCE, Git/Worktree drivers.

#### Danh mục các file bên trong `platform/adapters`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/adapters/Cargo.toml`](../../platform/adapters/Cargo.toml) | 67 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/adapters/src/lib.rs`](../../platform/adapters/src/lib.rs) | 67 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`platform/adapters/src/connectors/mod.rs`](../../platform/adapters/src/connectors/mod.rs) | 17 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/connectors/oauth/copilot.rs`](../../platform/adapters/src/connectors/oauth/copilot.rs) | 161 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `start_copilot_device, complete_copilot_device` |
| [`platform/adapters/src/connectors/oauth/mod.rs`](../../platform/adapters/src/connectors/oauth/mod.rs) | 24 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/connectors/oauth/pkce_manager.rs`](../../platform/adapters/src/connectors/oauth/pkce_manager.rs) | 384 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `OAuthPkceManager, new, generate_pkce_challenge, build_authorization_url, extract_code_from_input, exchange_code` |
| [`platform/adapters/src/connectors/oauth/provider_defaults.rs`](../../platform/adapters/src/connectors/oauth/provider_defaults.rs) | 208 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `oauth_settings, oauth_identity, google_oauth_client, google_oauth_client_with, public_google_oauth_defaults` |
| [`platform/adapters/src/connectors/oauth/types.rs`](../../platform/adapters/src/connectors/oauth/types.rs) | 50 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `PkceChallenge, OAuthAuthorizeUrlResult, resolve_openai_client_id, resolve_openai_scope` |
| [`platform/adapters/src/connectors/oauth/tests/mod.rs`](../../platform/adapters/src/connectors/oauth/tests/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/connectors/oauth/tests/pkce_tests.rs`](../../platform/adapters/src/connectors/oauth/tests/pkce_tests.rs) | 74 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/oauth/tests/provider_tests.rs`](../../platform/adapters/src/connectors/oauth/tests/provider_tests.rs) | 209 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/a2a/mod.rs`](../../platform/adapters/src/connectors/a2a/mod.rs) | 175 | Submodule root: aggregates and re-exports sibling modules | `A2aClient, A2aDispatcher, new, with_timeout, delegate_task, cancel_task` |
| [`platform/adapters/src/connectors/mcp/federation.rs`](../../platform/adapters/src/connectors/mcp/federation.rs) | 352 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `McpToolGate, McpFederationHub, new, set_gate, set_gate_sync, register_server` |
| [`platform/adapters/src/connectors/mcp/mod.rs`](../../platform/adapters/src/connectors/mcp/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/connectors/mcp/security.rs`](../../platform/adapters/src/connectors/mcp/security.rs) | 159 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `is_localhost_host, is_localhost_origin, is_localhost_request, is_dangerous_tool_pattern` |
| [`platform/adapters/src/connectors/mcp/adapters/mod.rs`](../../platform/adapters/src/connectors/mcp/adapters/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/connectors/mcp/adapters/client/context.rs`](../../platform/adapters/src/connectors/mcp/adapters/client/context.rs) | 46 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `ToolCallContext, new, with_working_dir, with_sade_scope` |
| [`platform/adapters/src/connectors/mcp/adapters/client/mod.rs`](../../platform/adapters/src/connectors/mcp/adapters/client/mod.rs) | 531 | Submodule root: aggregates and re-exports sibling modules | `CustosMcpHostInfo, McpClientTrait, StdioProcess, HttpClient, CustosMcpClient, spawn` |
| [`platform/adapters/src/connectors/mcp/adapters/client/tests.rs`](../../platform/adapters/src/connectors/mcp/adapters/client/tests.rs) | 58 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/compute/checkpoint.rs`](../../platform/adapters/src/connectors/compute/checkpoint.rs) | 131 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `verify_local_checkpoint_files` |
| [`platform/adapters/src/connectors/compute/checkpoint_store.rs`](../../platform/adapters/src/connectors/compute/checkpoint_store.rs) | 274 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `CasCheckpointStoreAdapter, new` |
| [`platform/adapters/src/connectors/compute/colab.rs`](../../platform/adapters/src/connectors/compute/colab.rs) | 411 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `ColabTargetAdapter, new, with_colab_bin, is_available, with_mock_mode, set_mock_status` |
| [`platform/adapters/src/connectors/compute/kaggle.rs`](../../platform/adapters/src/connectors/compute/kaggle.rs) | 506 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `KaggleTargetAdapter, new, with_kaggle_bin, is_available, with_credentials, with_mock_mode` |
| [`platform/adapters/src/connectors/compute/mod.rs`](../../platform/adapters/src/connectors/compute/mod.rs) | 93 | Submodule root: aggregates and re-exports sibling modules | `DefaultExecutionTargetResolver, MockExecutionTargetResolver` |
| [`platform/adapters/src/connectors/compute/rented_gpu/mod.rs`](../../platform/adapters/src/connectors/compute/rented_gpu/mod.rs) | 467 | Submodule root: aggregates and re-exports sibling modules | `RentedGpuTargetAdapter, new, with_ssh_key, with_pod_lease, with_mock_mode, is_available` |
| [`platform/adapters/src/connectors/compute/rented_gpu/ops.rs`](../../platform/adapters/src/connectors/compute/rented_gpu/ops.rs) | 125 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/compute/rented_gpu/tests.rs`](../../platform/adapters/src/connectors/compute/rented_gpu/tests.rs) | 84 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/roaming/card.rs`](../../platform/adapters/src/connectors/roaming/card.rs) | 247 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `ConnectionCard, new, fingerprint, encode, decode` |
| [`platform/adapters/src/connectors/roaming/directory.rs`](../../platform/adapters/src/connectors/roaming/directory.rs) | 268 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `Direction, PeerEntry, Directory, new, persistent, persistent_owned` |
| [`platform/adapters/src/connectors/roaming/error.rs`](../../platform/adapters/src/connectors/roaming/error.rs) | 22 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `RoamingError` |
| [`platform/adapters/src/connectors/roaming/frame.rs`](../../platform/adapters/src/connectors/roaming/frame.rs) | 79 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `write_frame, read_frame` |
| [`platform/adapters/src/connectors/roaming/handshake.rs`](../../platform/adapters/src/connectors/roaming/handshake.rs) | 43 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `ClientHello, HostAck, new` |
| [`platform/adapters/src/connectors/roaming/identity.rs`](../../platform/adapters/src/connectors/roaming/identity.rs) | 208 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `RoamingIdentity, from_secret, generate, load_or_create, load, save` |
| [`platform/adapters/src/connectors/roaming/mod.rs`](../../platform/adapters/src/connectors/roaming/mod.rs) | 55 | Submodule root: aggregates and re-exports sibling modules | `parse_endpoint_id` |
| [`platform/adapters/src/connectors/roaming/peer_dispatch.rs`](../../platform/adapters/src/connectors/roaming/peer_dispatch.rs) | 208 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `PeerCapability, PeerDelegationRequest, PeerDelegationStatus, PeerDelegationResult, PeerDelegationRouter, new` |
| [`platform/adapters/src/connectors/roaming/peerbook.rs`](../../platform/adapters/src/connectors/roaming/peerbook.rs) | 241 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `PeerRecord, PeerBook, load, save, remove, rename` |
| [`platform/adapters/src/connectors/roaming/relay.rs`](../../platform/adapters/src/connectors/roaming/relay.rs) | 139 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `RelaySettings, RelayEntry, new, with_auth, advertised_urls, to_relay_mode` |
| [`platform/adapters/src/connectors/roaming/trust.rs`](../../platform/adapters/src/connectors/roaming/trust.rs) | 232 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | `TrustBook, new, accept, revoke_key, is_allowed, allowed_keys` |
| [`platform/adapters/src/connectors/roaming/node/handler.rs`](../../platform/adapters/src/connectors/roaming/node/handler.rs) | 224 | External protocol connectors: MCP, A2A, compute and OAuth PKCE | None |
| [`platform/adapters/src/connectors/roaming/node/mod.rs`](../../platform/adapters/src/connectors/roaming/node/mod.rs) | 480 | Submodule root: aggregates and re-exports sibling modules | `AcpStreamServer, RoamingConfig, RoamingNode, RoamingClientStream, new, with_relay` |
| [`platform/adapters/src/host/binaries.rs`](../../platform/adapters/src/host/binaries.rs) | 116 | Host operating system drivers: terminal, git, sandbox and filesystem | `resolve_agent_browser_binary, resolve_ssh_binary, resolve_gh_binary, resolve_git_binary` |
| [`platform/adapters/src/host/caffeinate.rs`](../../platform/adapters/src/host/caffeinate.rs) | 62 | Host operating system drivers: terminal, git, sandbox and filesystem | `CaffeinateManager, status, set_mode` |
| [`platform/adapters/src/host/integrations.rs`](../../platform/adapters/src/host/integrations.rs) | 127 | Host operating system drivers: terminal, git, sandbox and filesystem | `AgentBrowserOutput, run_agent_browser, run_agent_browser_json, read_screenshot_base64, ping_ssh_host, exec_ssh_command` |
| [`platform/adapters/src/host/mod.rs`](../../platform/adapters/src/host/mod.rs) | 36 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/workspace/folder_picker.rs`](../../platform/adapters/src/host/workspace/folder_picker.rs) | 31 | Host operating system drivers: terminal, git, sandbox and filesystem | `pick_folder` |
| [`platform/adapters/src/host/workspace/git_discovery.rs`](../../platform/adapters/src/host/workspace/git_discovery.rs) | 222 | Host operating system drivers: terminal, git, sandbox and filesystem | `github_login_command, github_auth_metadata, parse_github_remote, resolve_deep_git_paths, list_git_worktrees` |
| [`platform/adapters/src/host/workspace/git_ops.rs`](../../platform/adapters/src/host/workspace/git_ops.rs) | 330 | Host operating system drivers: terminal, git, sandbox and filesystem | `GitMergeOutcome, SubRepositoryInfo, DetectedGitRepository, current_branch, is_working_tree_clean, rev_parse` |
| [`platform/adapters/src/host/workspace/git_port.rs`](../../platform/adapters/src/host/workspace/git_port.rs) | 63 | Host operating system drivers: terminal, git, sandbox and filesystem | `LocalRepositoryGitAdapter, new` |
| [`platform/adapters/src/host/workspace/github.rs`](../../platform/adapters/src/host/workspace/github.rs) | 448 | Host operating system drivers: terminal, git, sandbox and filesystem | `GitHubBinding, validate, account_binding, github_viewer, github_accounts, bind_account` |
| [`platform/adapters/src/host/workspace/mod.rs`](../../platform/adapters/src/host/workspace/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/workspace/provider/mod.rs`](../../platform/adapters/src/host/workspace/provider/mod.rs) | 545 | Submodule root: aggregates and re-exports sibling modules | `LocalWorkspaceProvider, new` |
| [`platform/adapters/src/host/workspace/provider/tests.rs`](../../platform/adapters/src/host/workspace/provider/tests.rs) | 310 | Host operating system drivers: terminal, git, sandbox and filesystem | None |
| [`platform/adapters/src/host/workspace/files/git.rs`](../../platform/adapters/src/host/workspace/files/git.rs) | 372 | Host operating system drivers: terminal, git, sandbox and filesystem | `get_diff, get_file_diff, stage_file, unstage_file, discard_file, commit` |
| [`platform/adapters/src/host/workspace/files/io.rs`](../../platform/adapters/src/host/workspace/files/io.rs) | 287 | Host operating system drivers: terminal, git, sandbox and filesystem | `get_file_tree, read_file, write_file` |
| [`platform/adapters/src/host/workspace/files/mod.rs`](../../platform/adapters/src/host/workspace/files/mod.rs) | 179 | Submodule root: aggregates and re-exports sibling modules | `LocalWorkspaceFilesAdapter, resolve_safe_path, new` |
| [`platform/adapters/src/host/workspace/files/tests.rs`](../../platform/adapters/src/host/workspace/files/tests.rs) | 128 | Host operating system drivers: terminal, git, sandbox and filesystem | None |
| [`platform/adapters/src/host/terminal/mod.rs`](../../platform/adapters/src/host/terminal/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/terminal/pty.rs`](../../platform/adapters/src/host/terminal/pty.rs) | 530 | Host operating system drivers: terminal, git, sandbox and filesystem | `PtyHandle, NativePtyDriver, spawn` |
| [`platform/adapters/src/host/operations/mod.rs`](../../platform/adapters/src/host/operations/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/operations/search/lexical_search.rs`](../../platform/adapters/src/host/operations/search/lexical_search.rs) | 212 | Host operating system drivers: terminal, git, sandbox and filesystem | `LexicalSearchSkill` |
| [`platform/adapters/src/host/operations/search/mod.rs`](../../platform/adapters/src/host/operations/search/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/operations/http/fetch_url.rs`](../../platform/adapters/src/host/operations/http/fetch_url.rs) | 273 | Host operating system drivers: terminal, git, sandbox and filesystem | `FetchUrlSkill, new, validate_ssrf_url, validate_ssrf_safety` |
| [`platform/adapters/src/host/operations/http/mod.rs`](../../platform/adapters/src/host/operations/http/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/operations/fs/file_read.rs`](../../platform/adapters/src/host/operations/fs/file_read.rs) | 212 | Host operating system drivers: terminal, git, sandbox and filesystem | `FileReadSkill` |
| [`platform/adapters/src/host/operations/fs/mod.rs`](../../platform/adapters/src/host/operations/fs/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/operations/process/mod.rs`](../../platform/adapters/src/host/operations/process/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/operations/process/shell_exec.rs`](../../platform/adapters/src/host/operations/process/shell_exec.rs) | 372 | Host operating system drivers: terminal, git, sandbox and filesystem | `ProcessOutput, SandboxedProcess, LocalCommandRunner, stderr_tail, stdout_tail, succeeded` |
| [`platform/adapters/src/host/sandbox/capability_observations.rs`](../../platform/adapters/src/host/sandbox/capability_observations.rs) | 351 | Host operating system drivers: terminal, git, sandbox and filesystem | `CapabilityObservationsAdapter, new` |
| [`platform/adapters/src/host/sandbox/developer.rs`](../../platform/adapters/src/host/sandbox/developer.rs) | 422 | Host operating system drivers: terminal, git, sandbox and filesystem | `SovereignDeveloperAdapter, new, with_timeout` |
| [`platform/adapters/src/host/sandbox/developer_tests.rs`](../../platform/adapters/src/host/sandbox/developer_tests.rs) | 143 | Host operating system drivers: terminal, git, sandbox and filesystem | None |
| [`platform/adapters/src/host/sandbox/mod.rs`](../../platform/adapters/src/host/sandbox/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/sandbox/seatbelt/mod.rs`](../../platform/adapters/src/host/sandbox/seatbelt/mod.rs) | 155 | Submodule root: aggregates and re-exports sibling modules | `SeatbeltSandbox, new, generate_profile, wrap_command` |
| [`platform/adapters/src/host/sandbox/bubblewrap/mod.rs`](../../platform/adapters/src/host/sandbox/bubblewrap/mod.rs) | 141 | Submodule root: aggregates and re-exports sibling modules | `BubblewrapSandbox, new, wrap_command` |
| [`platform/adapters/src/host/packages/mod.rs`](../../platform/adapters/src/host/packages/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/packages/inventory/mod.rs`](../../platform/adapters/src/host/packages/inventory/mod.rs) | 112 | Submodule root: aggregates and re-exports sibling modules | `DiscoveredPackage, scan_local_packages` |
| [`platform/adapters/src/host/packages/installer/mod.rs`](../../platform/adapters/src/host/packages/installer/mod.rs) | 204 | Submodule root: aggregates and re-exports sibling modules | `LocalPackageTransferAdapter, new, with_max_size, sanitize_name` |
| [`platform/adapters/src/host/telemetry/host_metrics.rs`](../../platform/adapters/src/host/telemetry/host_metrics.rs) | 244 | Host operating system drivers: terminal, git, sandbox and filesystem | `ProcessGroupMetric, ListeningPortMetric, SystemTelemetrySnapshot, collect_system_telemetry` |
| [`platform/adapters/src/host/telemetry/mod.rs`](../../platform/adapters/src/host/telemetry/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/telemetry/otlp.rs`](../../platform/adapters/src/host/telemetry/otlp.rs) | 582 | Host operating system drivers: terminal, git, sandbox and filesystem | `normalize_otlp_traces_json, normalize_otlp_logs_json` |
| [`platform/adapters/src/host/documents/mod.rs`](../../platform/adapters/src/host/documents/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/documents/source_fetch.rs`](../../platform/adapters/src/host/documents/source_fetch.rs) | 261 | Host operating system drivers: terminal, git, sandbox and filesystem | `SourceFetchAdapter, new, with_base_dir` |
| [`platform/adapters/src/host/notebook/driver.rs`](../../platform/adapters/src/host/notebook/driver.rs) | 463 | Host operating system drivers: terminal, git, sandbox and filesystem | `LocalNotebookWorkerSession, LocalPythonDriver` |
| [`platform/adapters/src/host/notebook/mod.rs`](../../platform/adapters/src/host/notebook/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/host/worker/mod.rs`](../../platform/adapters/src/host/worker/mod.rs) | 542 | Submodule root: aggregates and re-exports sibling modules | `WorkerLease, spawn, spawn_command, await_observation, cancel` |
| [`platform/adapters/src/host/vault/mod.rs`](../../platform/adapters/src/host/vault/mod.rs) | 326 | Submodule root: aggregates and re-exports sibling modules | `KeyringCredentialVault, FileCredentialVault, new, default_dir` |
| [`platform/adapters/src/harnesses/adapter.rs`](../../platform/adapters/src/harnesses/adapter.rs) | 478 | CliAgentAdapter alias & ProcessDropGuard integration, zero-mock production isolation | `CliHarness, ClaudeCodeHarnessAdapter, OpenCodeHarnessAdapter, from_spec, new, claude_code` |
| [`platform/adapters/src/harnesses/adapter_tests.rs`](../../platform/adapters/src/harnesses/adapter_tests.rs) | 132 | External CLI agent adapters, process management and stream decoders | None |
| [`platform/adapters/src/harnesses/binary.rs`](../../platform/adapters/src/harnesses/binary.rs) | 82 | External CLI agent adapters, process management and stream decoders | `resolve_binary_path, probe_binary_available` |
| [`platform/adapters/src/harnesses/mock.rs`](../../platform/adapters/src/harnesses/mock.rs) | 36 | External CLI agent adapters, process management and stream decoders | `MockHarnessState, new, enabled, push_response, next_response` |
| [`platform/adapters/src/harnesses/mod.rs`](../../platform/adapters/src/harnesses/mod.rs) | 20 | Submodule root: aggregates and re-exports sibling modules | None |
| [`platform/adapters/src/harnesses/observer.rs`](../../platform/adapters/src/harnesses/observer.rs) | 41 | External CLI agent adapters, process management and stream decoders | `NoopObserver, CancellableObserver, new` |
| [`platform/adapters/src/harnesses/registry.rs`](../../platform/adapters/src/harnesses/registry.rs) | 264 | External CLI agent adapters, process management and stream decoders | `HarnessEntry, HarnessRegistry, new, default_with_workspace, register, register_with_launch_cmd` |
| [`platform/adapters/src/harnesses/spec.rs`](../../platform/adapters/src/harnesses/spec.rs) | 173 | External CLI agent adapters, process management and stream decoders | `StreamProtocol, HeadlessArgs, HarnessSpec, build_args, apply` |
| [`platform/adapters/src/harnesses/catalog/mod.rs`](../../platform/adapters/src/harnesses/catalog/mod.rs) | 17 | Submodule root: aggregates and re-exports sibling modules | `find_spec` |
| [`platform/adapters/src/harnesses/catalog/tests.rs`](../../platform/adapters/src/harnesses/catalog/tests.rs) | 80 | External CLI agent adapters, process management and stream decoders | None |
| [`platform/adapters/src/harnesses/stream/claude.rs`](../../platform/adapters/src/harnesses/stream/claude.rs) | 66 | External CLI agent adapters, process management and stream decoders | `normalize_claude` |
| [`platform/adapters/src/harnesses/stream/codex.rs`](../../platform/adapters/src/harnesses/stream/codex.rs) | 61 | External CLI agent adapters, process management and stream decoders | `normalize_codex` |
| [`platform/adapters/src/harnesses/stream/intents.rs`](../../platform/adapters/src/harnesses/stream/intents.rs) | 382 | External CLI agent adapters, process management and stream decoders | `extract_observed_intents` |
| [`platform/adapters/src/harnesses/stream/mod.rs`](../../platform/adapters/src/harnesses/stream/mod.rs) | 200 | Submodule root: aggregates and re-exports sibling modules | `normalize, normalize_by_harness` |
| [`platform/adapters/src/harnesses/stream/opencode.rs`](../../platform/adapters/src/harnesses/stream/opencode.rs) | 47 | External CLI agent adapters, process management and stream decoders | `normalize_opencode` |
| [`platform/adapters/src/harnesses/stream/plain.rs`](../../platform/adapters/src/harnesses/stream/plain.rs) | 10 | External CLI agent adapters, process management and stream decoders | `emit_plain_delta` |
| [`platform/adapters/src/harnesses/stream/process.rs`](../../platform/adapters/src/harnesses/stream/process.rs) | 316 | Subprocess process group (setpgid) isolation and kill-on-drop SIGKILL guard against zombie processes | `run_observed_process_with_protocol, run_observed_process` |
| [`platform/adapters/src/harnesses/stream/usage.rs`](../../platform/adapters/src/harnesses/stream/usage.rs) | 35 | External CLI agent adapters, process management and stream decoders | `emit_usage` |

### 3.7. Crate `custos-packs` — Layer 3 Base: Packs Registry & Verifiers

- **Đường dẫn thư mục:** `platform/components`
- **Chủ sở hữu chính (Owner):** **Vĩ**
- **Quy tắc ranh giới:** Không reqwest, không phụ thuộc cụ thể vào specialist workbench crates. Đóng gói module lifecycle.
- **Tổng số tệp:** 6 tệp (5 tệp Rust) | **Tổng số dòng mã:** 842 dòng Rust.
- **Mô tả chức năng:** Hạ tầng phân hệ mở rộng: Quản lý đăng ký component, module activation lifecycle, migrations của pack và trình thẩm định tích hợp.

#### Danh mục các file bên trong `platform/components`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`platform/components/Cargo.toml`](../../platform/components/Cargo.toml) | 30 | Cargo manifest: package metadata and dependency configuration | None |
| [`platform/components/src/lib.rs`](../../platform/components/src/lib.rs) | 249 | Crate root module: exports canonical public APIs and submodule declarations | `create_default_pack_registry, get_manifest_for_pack, resolve_pack_by_id, create_default_skill_registry, create_pack_registry_from_inventory, create_skill_registry_from_inventory` |
| [`platform/components/src/module.rs`](../../platform/components/src/module.rs) | 124 | Core architectural component supporting module responsibilities | `ModuleRegistry, new, register, get, has, list` |
| [`platform/components/src/registry.rs`](../../platform/components/src/registry.rs) | 282 | Core architectural component supporting module responsibilities | `DomainPackRegistry, new, register, get_pack, get_pack_for_lens, get_role` |
| [`platform/components/src/verifier.rs`](../../platform/components/src/verifier.rs) | 64 | Core architectural component supporting module responsibilities | `PackVerifierAdapter` |
| [`platform/components/src/migrations/mod.rs`](../../platform/components/src/migrations/mod.rs) | 123 | Submodule root: aggregates and re-exports sibling modules | `MigrationScript, WorkbenchMigrationRecord, MigrationDispatcher, compute_checksum, discover_migrations, validate_scoped_tables` |

### 3.8. Crate `custos-workbench-assistant` — Layer 3: Assistant Workbench

- **Đường dẫn thư mục:** `workbenches/assistant`
- **Chủ sở hữu chính (Owner):** **Vinh**
- **Quy tắc ranh giới:** Chuyên biệt hóa miền trợ lý; không rò rỉ logic sang kernel hay các workbench khác.
- **Tổng số tệp:** 19 tệp (18 tệp Rust) | **Tổng số dòng mã:** 1,728 dòng Rust.
- **Mô tả chức năng:** Bàn làm việc Trợ lý: Quản lý bản thảo (drafts), outbox, lịch trình cá nhân, danh bạ và kỹ năng tương tác trợ lý.

#### Danh mục các file bên trong `workbenches/assistant`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`workbenches/assistant/Cargo.toml`](../../workbenches/assistant/Cargo.toml) | 27 | Cargo manifest: package metadata and dependency configuration | None |
| [`workbenches/assistant/src/fixtures.rs`](../../workbenches/assistant/src/fixtures.rs) | 26 | Assistant workbench component: drafts, outbox and agenda management | `assistant_agenda_fixture, assistant_draft_reply_fixture` |
| [`workbenches/assistant/src/lib.rs`](../../workbenches/assistant/src/lib.rs) | 292 | Crate root module: exports canonical public APIs and submodule declarations | `AssistantPackDescriptor, AssistantPack, AssistantModule, get_descriptor, new` |
| [`workbenches/assistant/src/profile.rs`](../../workbenches/assistant/src/profile.rs) | 21 | Assistant workbench component: drafts, outbox and agenda management | `assistant_profile` |
| [`workbenches/assistant/src/repository.rs`](../../workbenches/assistant/src/repository.rs) | 240 | Assistant workbench component: drafts, outbox and agenda management | `DbConnection, AssistantRepository, new, open, open_in_memory, lock` |
| [`workbenches/assistant/src/supervision.rs`](../../workbenches/assistant/src/supervision.rs) | 20 | Assistant workbench component: drafts, outbox and agenda management | `assistant_supervision_profile` |
| [`workbenches/assistant/src/verifiers/mod.rs`](../../workbenches/assistant/src/verifiers/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/assistant/src/verifiers/user_acceptance_oracle.rs`](../../workbenches/assistant/src/verifiers/user_acceptance_oracle.rs) | 289 | Assistant workbench component: drafts, outbox and agenda management | `UserAcceptanceOracle, new, with_grant_store` |
| [`workbenches/assistant/src/skills/mod.rs`](../../workbenches/assistant/src/skills/mod.rs) | 58 | Submodule root: aggregates and re-exports sibling modules | `register_assistant_skills` |
| [`workbenches/assistant/src/skills/draft/mod.rs`](../../workbenches/assistant/src/skills/draft/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/assistant/src/skills/draft/payload_lock.rs`](../../workbenches/assistant/src/skills/draft/payload_lock.rs) | 103 | Assistant workbench component: drafts, outbox and agenda management | `PayloadLockSkill` |
| [`workbenches/assistant/src/skills/identity/contact_resolve.rs`](../../workbenches/assistant/src/skills/identity/contact_resolve.rs) | 136 | Assistant workbench component: drafts, outbox and agenda management | `ContactResolveSkill` |
| [`workbenches/assistant/src/skills/identity/mod.rs`](../../workbenches/assistant/src/skills/identity/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/assistant/src/skills/subagent/ax_tree_audit.rs`](../../workbenches/assistant/src/skills/subagent/ax_tree_audit.rs) | 192 | Assistant workbench component: drafts, outbox and agenda management | `AxTreeAuditSkill, AxNodeInput, AxBounds` |
| [`workbenches/assistant/src/skills/subagent/browser_subagent_dispatch.rs`](../../workbenches/assistant/src/skills/subagent/browser_subagent_dispatch.rs) | 139 | Assistant workbench component: drafts, outbox and agenda management | `BrowserSubagentDispatchSkill` |
| [`workbenches/assistant/src/skills/subagent/mod.rs`](../../workbenches/assistant/src/skills/subagent/mod.rs) | 10 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/assistant/src/domain/draft.rs`](../../workbenches/assistant/src/domain/draft.rs) | 117 | Assistant workbench component: drafts, outbox and agenda management | `AssistantDraft, AssistantDraftPort, new, with_task_id, with_session_id, with_recipient` |
| [`workbenches/assistant/src/domain/mod.rs`](../../workbenches/assistant/src/domain/mod.rs) | 6 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/assistant/src/domain/pipeline_invariant.rs`](../../workbenches/assistant/src/domain/pipeline_invariant.rs) | 68 | Assistant workbench component: drafts, outbox and agenda management | `GovernedOutboxInvariant, new` |

### 3.9. Crate `custos-workbench-coding` — Layer 3: Coding Workbench

- **Đường dẫn thư mục:** `workbenches/coding`
- **Chủ sở hữu chính (Owner):** **Vĩ + Vinh**
- **Quy tắc ranh giới:** Cô lập trong worktree Git; AST và test verifiers không được tạo mutation ngoài sandbox.
- **Tổng số tệp:** 19 tệp (18 tệp Rust) | **Tổng số dòng mã:** 1,747 dòng Rust.
- **Mô tả chức năng:** Bàn làm việc Lập trình: Kỹ năng code (UI, AST, Test, Git), công thức lập trình (Engineering Recipes), và các bộ thẩm định workspace/AST.

#### Danh mục các file bên trong `workbenches/coding`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`workbenches/coding/Cargo.toml`](../../workbenches/coding/Cargo.toml) | 26 | Cargo manifest: package metadata and dependency configuration | None |
| [`workbenches/coding/src/fixtures.rs`](../../workbenches/coding/src/fixtures.rs) | 44 | Coding workbench component: code recipes, AST analysis and workspace verification | `engineering_repair_fixture, engineering_refactor_fixture` |
| [`workbenches/coding/src/lib.rs`](../../workbenches/coding/src/lib.rs) | 333 | Crate root module: exports canonical public APIs and submodule declarations | `EngineeringPackDescriptor, EngineeringPack, CodingModule, get_descriptor, new` |
| [`workbenches/coding/src/pipeline_invariant.rs`](../../workbenches/coding/src/pipeline_invariant.rs) | 92 | Coding workbench component: code recipes, AST analysis and workspace verification | `AstSyntaxInvariant, new` |
| [`workbenches/coding/src/profile.rs`](../../workbenches/coding/src/profile.rs) | 22 | Coding workbench component: code recipes, AST analysis and workspace verification | `engineering_profile` |
| [`workbenches/coding/src/recipe.rs`](../../workbenches/coding/src/recipe.rs) | 240 | Coding workbench component: code recipes, AST analysis and workspace verification | `EngineeringRecipeInput, DiagnosticOutcome, PatchOutcome, VerifyOutcome, RecipeFailure, EngineeringRecipeOutcome` |
| [`workbenches/coding/src/supervision.rs`](../../workbenches/coding/src/supervision.rs) | 20 | Coding workbench component: code recipes, AST analysis and workspace verification | `coding_supervision_profile` |
| [`workbenches/coding/src/verifiers/cargo_test_oracle.rs`](../../workbenches/coding/src/verifiers/cargo_test_oracle.rs) | 111 | Coding workbench component: code recipes, AST analysis and workspace verification | `CargoTestOracle, new, with_timeout` |
| [`workbenches/coding/src/verifiers/mod.rs`](../../workbenches/coding/src/verifiers/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/coding/src/skills/mod.rs`](../../workbenches/coding/src/skills/mod.rs) | 90 | Submodule root: aggregates and re-exports sibling modules | `SkillParameter, SkillMetadata, SkillExecutionResult, EngineeringSkill, register_engineering_skills` |
| [`workbenches/coding/src/skills/ui/component_clone.rs`](../../workbenches/coding/src/skills/ui/component_clone.rs) | 208 | Coding workbench component: code recipes, AST analysis and workspace verification | `ComponentCloneSkill, ExtractedTokens` |
| [`workbenches/coding/src/skills/ui/local_ui_diagnostic.rs`](../../workbenches/coding/src/skills/ui/local_ui_diagnostic.rs) | 135 | Coding workbench component: code recipes, AST analysis and workspace verification | `LocalUiDiagnosticSkill` |
| [`workbenches/coding/src/skills/ui/mod.rs`](../../workbenches/coding/src/skills/ui/mod.rs) | 10 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/coding/src/skills/test/cargo_test.rs`](../../workbenches/coding/src/skills/test/cargo_test.rs) | 106 | Coding workbench component: code recipes, AST analysis and workspace verification | `CargoTestSkill` |
| [`workbenches/coding/src/skills/test/mod.rs`](../../workbenches/coding/src/skills/test/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/coding/src/skills/ast/ast_search.rs`](../../workbenches/coding/src/skills/ast/ast_search.rs) | 189 | Coding workbench component: code recipes, AST analysis and workspace verification | `AstSearchSkill` |
| [`workbenches/coding/src/skills/ast/mod.rs`](../../workbenches/coding/src/skills/ast/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/coding/src/skills/git/git_patch.rs`](../../workbenches/coding/src/skills/git/git_patch.rs) | 134 | Coding workbench component: code recipes, AST analysis and workspace verification | `GitPatchSkill` |
| [`workbenches/coding/src/skills/git/mod.rs`](../../workbenches/coding/src/skills/git/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |

### 3.10. Crate `custos-workbench-research` — Layer 3: Research Workbench

- **Đường dẫn thư mục:** `workbenches/research`
- **Chủ sở hữu chính (Owner):** **Trường**
- **Quy tắc ranh giới:** Bảo đảm tính trung thực khoa học: Citation không đồng nghĩa Support; rollback saga khi tạo task batch thất bại.
- **Tổng số tệp:** 34 tệp (33 tệp Rust) | **Tổng số dòng mã:** 4,184 dòng Rust.
- **Mô tả chức năng:** Bàn làm việc Nghiên cứu: Quản lý tuyên bố khoa học (Claims), trích dẫn tài liệu, tổng hợp bằng chứng, ALCE verifiers và review handoff saga.

#### Danh mục các file bên trong `workbenches/research`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`workbenches/research/Cargo.toml`](../../workbenches/research/Cargo.toml) | 28 | Cargo manifest: package metadata and dependency configuration | None |
| [`workbenches/research/src/fixtures.rs`](../../workbenches/research/src/fixtures.rs) | 32 | Research workbench component: claim extraction, citation verification and ALCE scoring | `research_synthesis_fixture, research_survey_fixture` |
| [`workbenches/research/src/jobs.rs`](../../workbenches/research/src/jobs.rs) | 106 | Research workbench component: claim extraction, citation verification and ALCE scoring | `research_jobs` |
| [`workbenches/research/src/lib.rs`](../../workbenches/research/src/lib.rs) | 81 | Crate root module: exports canonical public APIs and submodule declarations | `ResearchModule, new` |
| [`workbenches/research/src/pack.rs`](../../workbenches/research/src/pack.rs) | 142 | Research workbench component: claim extraction, citation verification and ALCE scoring | `ResearchPackDescriptor, ResearchPack, get_descriptor, new` |
| [`workbenches/research/src/profile.rs`](../../workbenches/research/src/profile.rs) | 23 | Research workbench component: claim extraction, citation verification and ALCE scoring | `research_profile` |
| [`workbenches/research/src/roles.rs`](../../workbenches/research/src/roles.rs) | 48 | Research workbench component: claim extraction, citation verification and ALCE scoring | `research_roles` |
| [`workbenches/research/src/supervision.rs`](../../workbenches/research/src/supervision.rs) | 24 | Research workbench component: claim extraction, citation verification and ALCE scoring | `research_supervision_profile` |
| [`workbenches/research/src/repository/claims.rs`](../../workbenches/research/src/repository/claims.rs) | 174 | Research workbench component: claim extraction, citation verification and ALCE scoring | `save_claim, list_claims` |
| [`workbenches/research/src/repository/executions.rs`](../../workbenches/research/src/repository/executions.rs) | 499 | Research workbench component: claim extraction, citation verification and ALCE scoring | `save_run, list_runs, save_recipe, get_recipe, list_recipes, save_execution_record` |
| [`workbenches/research/src/repository/mod.rs`](../../workbenches/research/src/repository/mod.rs) | 97 | Submodule root: aggregates and re-exports sibling modules | `DbConnection, ResearchRepository, new, open, open_in_memory, lock` |
| [`workbenches/research/src/repository/sources.rs`](../../workbenches/research/src/repository/sources.rs) | 167 | Research workbench component: claim extraction, citation verification and ALCE scoring | `save_source, ingest_source_and_claim_atomic, list_sources, save_anchor, list_anchors_for_source` |
| [`workbenches/research/src/repository/synthesis.rs`](../../workbenches/research/src/repository/synthesis.rs) | 142 | Research workbench component: claim extraction, citation verification and ALCE scoring | `save_synthesis_proposal, get_synthesis_proposal, list_synthesis_proposals` |
| [`workbenches/research/src/repository/tests_checkpoint.rs`](../../workbenches/research/src/repository/tests_checkpoint.rs) | 107 | Research workbench component: claim extraction, citation verification and ALCE scoring | None |
| [`workbenches/research/src/repository/tests_lifecycle.rs`](../../workbenches/research/src/repository/tests_lifecycle.rs) | 215 | Research workbench component: claim extraction, citation verification and ALCE scoring | None |
| [`workbenches/research/src/contracts/mod.rs`](../../workbenches/research/src/contracts/mod.rs) | 3 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/contracts/research_ingress.rs`](../../workbenches/research/src/contracts/research_ingress.rs) | 115 | Research workbench component: claim extraction, citation verification and ALCE scoring | `source_draft, source_draft_with_bytes, passage_draft, claim_draft, experiment_draft` |
| [`workbenches/research/src/verifiers/alce.rs`](../../workbenches/research/src/verifiers/alce.rs) | 237 | Research workbench component: claim extraction, citation verification and ALCE scoring | `AlceEntailmentEngine, new, with_threshold, evaluate_entailment, assess_claim` |
| [`workbenches/research/src/verifiers/citation_coverage_oracle.rs`](../../workbenches/research/src/verifiers/citation_coverage_oracle.rs) | 496 | Research workbench component: claim extraction, citation verification and ALCE scoring | `CitationCoverageOracle, new, with_threshold` |
| [`workbenches/research/src/verifiers/mod.rs`](../../workbenches/research/src/verifiers/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/skills/mod.rs`](../../workbenches/research/src/skills/mod.rs) | 49 | Submodule root: aggregates and re-exports sibling modules | `register_research_skills` |
| [`workbenches/research/src/skills/web/mod.rs`](../../workbenches/research/src/skills/web/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/skills/web/web_extract.rs`](../../workbenches/research/src/skills/web/web_extract.rs) | 172 | Research workbench component: claim extraction, citation verification and ALCE scoring | `WebExtractSkill, new` |
| [`workbenches/research/src/skills/verification/doi_verify.rs`](../../workbenches/research/src/skills/verification/doi_verify.rs) | 153 | Research workbench component: claim extraction, citation verification and ALCE scoring | `DoiVerifySkill, new, is_valid_doi_format` |
| [`workbenches/research/src/skills/verification/mod.rs`](../../workbenches/research/src/skills/verification/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/skills/claims/claim_extract.rs`](../../workbenches/research/src/skills/claims/claim_extract.rs) | 165 | Research workbench component: claim extraction, citation verification and ALCE scoring | `ClaimExtractSkill` |
| [`workbenches/research/src/skills/claims/mod.rs`](../../workbenches/research/src/skills/claims/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/domain/claim.rs`](../../workbenches/research/src/domain/claim.rs) | 476 | Research workbench component: claim extraction, citation verification and ALCE scoring | `SourceRecord, PassageAnchor, PassageAnchorProposal, EvidenceRelation, ClaimEvidenceLink, ResearchClaim` |
| [`workbenches/research/src/domain/mod.rs`](../../workbenches/research/src/domain/mod.rs) | 9 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/domain/note.rs`](../../workbenches/research/src/domain/note.rs) | 3 | Research workbench component: claim extraction, citation verification and ALCE scoring | None |
| [`workbenches/research/src/domain/pipeline_invariant.rs`](../../workbenches/research/src/domain/pipeline_invariant.rs) | 82 | Research workbench component: claim extraction, citation verification and ALCE scoring | `EvidenceGroundingInvariant, new` |
| [`workbenches/research/src/domain/synthesis.rs`](../../workbenches/research/src/domain/synthesis.rs) | 6 | Research workbench component: claim extraction, citation verification and ALCE scoring | None |
| [`workbenches/research/src/services/mod.rs`](../../workbenches/research/src/services/mod.rs) | 6 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/research/src/services/synthesis.rs`](../../workbenches/research/src/services/synthesis.rs) | 340 | Research workbench component: claim extraction, citation verification and ALCE scoring | `PreparedCodingHandoff, SynthesisService, build_and_save_proposal, prepare_claim_investigation, prepare_coding_handoff, mark_proposal_completed` |

### 3.11. Crate `custos-workbench-quant` — Layer 3: Quant Workbench

- **Đường dẫn thư mục:** `workbenches/quant`
- **Chủ sở hữu chính (Owner):** **Trường + Vĩ**
- **Quy tắc ranh giới:** Zero-Mock cho dữ liệu thị trường thực; toàn bộ toán tài chính nằm trong workbench, daemon chỉ là thin transport adapter.
- **Tổng số tệp:** 36 tệp (35 tệp Rust) | **Tổng số dòng mã:** 7,199 dòng Rust.
- **Mô tả chức năng:** Bàn làm việc Định lượng: Mô hình tài chính, backtest engine, sổ lệnh thị trường, thẩm định Deflated Sharpe và Lookahead Bias.

#### Danh mục các file bên trong `workbenches/quant`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`workbenches/quant/Cargo.toml`](../../workbenches/quant/Cargo.toml) | 24 | Cargo manifest: package metadata and dependency configuration | None |
| [`workbenches/quant/src/contracts.rs`](../../workbenches/quant/src/contracts.rs) | 59 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `QuantPort` |
| [`workbenches/quant/src/jobs.rs`](../../workbenches/quant/src/jobs.rs) | 56 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `quant_jobs` |
| [`workbenches/quant/src/lib.rs`](../../workbenches/quant/src/lib.rs) | 56 | Crate root module: exports canonical public APIs and submodule declarations | `QuantModule, new` |
| [`workbenches/quant/src/pack.rs`](../../workbenches/quant/src/pack.rs) | 169 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `QuantPack, new` |
| [`workbenches/quant/src/repository.rs`](../../workbenches/quant/src/repository.rs) | 393 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `QuantRepository, new, ensure_tables` |
| [`workbenches/quant/src/roles.rs`](../../workbenches/quant/src/roles.rs) | 59 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `quant_roles` |
| [`workbenches/quant/src/supervision.rs`](../../workbenches/quant/src/supervision.rs) | 24 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `quant_supervision_profile` |
| [`workbenches/quant/src/verifiers/deflated_sharpe.rs`](../../workbenches/quant/src/verifiers/deflated_sharpe.rs) | 73 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `DeflatedSharpeVerifier` |
| [`workbenches/quant/src/verifiers/lookahead_bias.rs`](../../workbenches/quant/src/verifiers/lookahead_bias.rs) | 55 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `LookaheadBiasChecker` |
| [`workbenches/quant/src/verifiers/mod.rs`](../../workbenches/quant/src/verifiers/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/quant/src/verifiers/pbo_cscv.rs`](../../workbenches/quant/src/verifiers/pbo_cscv.rs) | 161 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `PboOverfittingVerifier` |
| [`workbenches/quant/src/execution/mod.rs`](../../workbenches/quant/src/execution/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/quant/src/execution/stream.rs`](../../workbenches/quant/src/execution/stream.rs) | 253 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `MarketStreamError, StreamState, SubscriptionKey, MarketStreamCoordinator, new, state` |
| [`workbenches/quant/src/skills/mod.rs`](../../workbenches/quant/src/skills/mod.rs) | 215 | Submodule root: aggregates and re-exports sibling modules | `QuantLookaheadAuditSkill, QuantDsrCalculateSkill, QuantPboEvaluateSkill, register_quant_skills` |
| [`workbenches/quant/src/domain/backtest.rs`](../../workbenches/quant/src/domain/backtest.rs) | 243 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `MonthlyReturn, BacktestTearsheet, new, with_deflated_sharpe, with_extended_metrics, with_institutional_metrics` |
| [`workbenches/quant/src/domain/contracts.rs`](../../workbenches/quant/src/domain/contracts.rs) | 387 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `OrderSide, MarketType, ExecutionStatus, OrderIntent, FillSnapshot, PositionSnapshot` |
| [`workbenches/quant/src/domain/engine.rs`](../../workbenches/quant/src/domain/engine.rs) | 298 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `OhlcvBar, BacktestConfig, SignalAction, StrategyRule, DualMovingAverageRule, NativeBacktestEngine` |
| [`workbenches/quant/src/domain/factor.rs`](../../workbenches/quant/src/domain/factor.rs) | 229 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `AlphaFactor, FactorClassification, BenjaminiHochberg, new, with_metrics, with_tags` |
| [`workbenches/quant/src/domain/gate.rs`](../../workbenches/quant/src/domain/gate.rs) | 272 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `DecisionContext, GateOutcome, PreTradeDecisionGate, new, with_risk_state, order_notional` |
| [`workbenches/quant/src/domain/grid.rs`](../../workbenches/quant/src/domain/grid.rs) | 306 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `GridMode, GridCell, GridConfig, GridOrderAction, GridStatusSummary, NativeGridEngine` |
| [`workbenches/quant/src/domain/lease.rs`](../../workbenches/quant/src/domain/lease.rs) | 288 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `LeaseTier, LeaseRecord, LeaseGuard, StrategyCommandType, StrategyCommandStatus, StrategyCommand` |
| [`workbenches/quant/src/domain/metrics.rs`](../../workbenches/quant/src/domain/metrics.rs) | 207 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `InstitutionalMetrics, calculate_downside_deviation, calculate_sortino_ratio, calculate_cvar, calculate_omega_ratio, calculate_dual_beta` |
| [`workbenches/quant/src/domain/mod.rs`](../../workbenches/quant/src/domain/mod.rs) | 447 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/quant/src/domain/order.rs`](../../workbenches/quant/src/domain/order.rs) | 258 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `OrderDirection, OrderType, OrderTicketStatus, OrderTicket, new, with_limit_price` |
| [`workbenches/quant/src/domain/orderbook.rs`](../../workbenches/quant/src/domain/orderbook.rs) | 181 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `OrderBookLevel, OrderBookSnapshot, new, from_levels, synthesize` |
| [`workbenches/quant/src/domain/pbo.rs`](../../workbenches/quant/src/domain/pbo.rs) | 217 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `PboResult, calculate_pbo_cscv` |
| [`workbenches/quant/src/domain/pipeline_invariant.rs`](../../workbenches/quant/src/domain/pipeline_invariant.rs) | 54 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `ClosedBarInvariant, new` |
| [`workbenches/quant/src/domain/portal.rs`](../../workbenches/quant/src/domain/portal.rs) | 327 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `BarTimeframe, NativeDataPortal, BollingerBandsResult, MacdResult, from_str_loose, is_bar_closed` |
| [`workbenches/quant/src/domain/review.rs`](../../workbenches/quant/src/domain/review.rs) | 493 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `TradeFillRecord, HoldingCohort, ExitReasonCohort, DiagnosticCheck, PostTradeReviewReport, PostTradeAnalyzer` |
| [`workbenches/quant/src/domain/risk.rs`](../../workbenches/quant/src/domain/risk.rs) | 278 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `RiskBreach, RiskPermitConfig, ProtectionSpec, ProtectionState, new, with_drawdown_and_dsr_limits` |
| [`workbenches/quant/src/services/backtest.rs`](../../workbenches/quant/src/services/backtest.rs) | 259 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `BacktestService, submit_backtest, get_tearsheet, evaluate_grid, compute_indicators, compute_pbo` |
| [`workbenches/quant/src/services/dto.rs`](../../workbenches/quant/src/services/dto.rs) | 393 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `SubmitBacktestRequest, GetTearsheetRequest, GridEvaluateRequest, ComputeIndicatorsRequest, ComputePboRequest, OrderSpecDto` |
| [`workbenches/quant/src/services/market.rs`](../../workbenches/quant/src/services/market.rs) | 254 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `MarketService, discover_factors, ingest_market_bars, get_market_bars, evaluate_gate, compute_review` |
| [`workbenches/quant/src/services/mod.rs`](../../workbenches/quant/src/services/mod.rs) | 16 | Submodule root: aggregates and re-exports sibling modules | None |
| [`workbenches/quant/src/services/orders.rs`](../../workbenches/quant/src/services/orders.rs) | 204 | Quant workbench component: financial modeling, backtesting and risk audit verifiers | `OrderExecutionService, create_orders, list_orders, approve_order, kill_switch, get_orderbook` |

### 3.12. Crate `custos-protocol` — Layer 0.5: Wire Protocol & Envelopes

- **Đường dẫn thư mục:** `sdk/protocol`
- **Chủ sở hữu chính (Owner):** **Vinh**
- **Quy tắc ranh giới:** Chỉ phụ thuộc custos-domain. Transport-agnostic.
- **Tổng số tệp:** 31 tệp (30 tệp Rust) | **Tổng số dòng mã:** 4,822 dòng Rust.
- **Mô tả chức năng:** Giao thức truyền thông: DTOs, Local API request/response envelopes, ApiTransport trait, wire formats cho A2A, profile và research.

#### Danh mục các file bên trong `sdk/protocol`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`sdk/protocol/Cargo.toml`](../../sdk/protocol/Cargo.toml) | 27 | Cargo manifest: package metadata and dependency configuration | None |
| [`sdk/protocol/src/a2a.rs`](../../sdk/protocol/src/a2a.rs) | 176 | Wire protocol specification, Local API envelopes and custom request DTOs | `AgentCard, AgentInterface, AgentCapabilities, AgentAuthRequirement, A2aTaskState, A2aJsonRpcRequest` |
| [`sdk/protocol/src/harness.rs`](../../sdk/protocol/src/harness.rs) | 4 | Wire protocol specification, Local API envelopes and custom request DTOs | None |
| [`sdk/protocol/src/lib.rs`](../../sdk/protocol/src/lib.rs) | 15 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`sdk/protocol/src/profile.rs`](../../sdk/protocol/src/profile.rs) | 158 | Wire protocol specification, Local API envelopes and custom request DTOs | `DaemonLifecycleState, ProfileResolver, from_env, for_profile, with_dir, profile_id` |
| [`sdk/protocol/src/worker.rs`](../../sdk/protocol/src/worker.rs) | 176 | Wire protocol specification, Local API envelopes and custom request DTOs | `WorkerExecutionKind, WorkerOperation, WorkerBudget, WorkerUsage, WorkerObservation, WorkerHandshake` |
| [`sdk/protocol/src/research/mod.rs`](../../sdk/protocol/src/research/mod.rs) | 28 | Submodule root: aggregates and re-exports sibling modules | `ClaimGroundingLevel` |
| [`sdk/protocol/src/research/notes.rs`](../../sdk/protocol/src/research/notes.rs) | 3 | Wire protocol specification, Local API envelopes and custom request DTOs | None |
| [`sdk/protocol/src/research/synthesis.rs`](../../sdk/protocol/src/research/synthesis.rs) | 213 | Wire protocol specification, Local API envelopes and custom request DTOs | `SynthesisProposalStatus, ClaimHandoffSummary, RecipeHandoffSummary, ResearchSynthesisProposal, SaveSynthesisProposalParams, HandoffToCodingParams` |
| [`sdk/protocol/src/wire_types/custom_notifications.rs`](../../sdk/protocol/src/wire_types/custom_notifications.rs) | 285 | Wire protocol specification, Local API envelopes and custom request DTOs | `GooseSessionNotification, GooseSessionUpdate, LiveVoiceInteractionEndedUpdate, LiveVoiceInteractionOutcome, ProviderDeviceCodeNotification, SessionUsageUpdate` |
| [`sdk/protocol/src/wire_types/mod.rs`](../../sdk/protocol/src/wire_types/mod.rs) | 7 | Submodule root: aggregates and re-exports sibling modules | None |
| [`sdk/protocol/src/wire_types/custom_requests/apps.rs`](../../sdk/protocol/src/wire_types/custom_requests/apps.rs) | 59 | Wire protocol specification, Local API envelopes and custom request DTOs | `AppsListRequest, AppsListResponse, AppsExportRequest, AppsExportResponse, AppsImportRequest, AppsImportResponse` |
| [`sdk/protocol/src/wire_types/custom_requests/common.rs`](../../sdk/protocol/src/wire_types/custom_requests/common.rs) | 25 | Wire protocol specification, Local API envelopes and custom request DTOs | `CustomMethodSchema, EmptyResponse` |
| [`sdk/protocol/src/wire_types/custom_requests/config.rs`](../../sdk/protocol/src/wire_types/custom_requests/config.rs) | 203 | Wire protocol specification, Local API envelopes and custom request DTOs | `PreferencesReadRequest, PreferencesSaveRequest, PreferenceKey, PreferenceValue, PreferencesReadResponse, ConfigReadRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/dictation.rs`](../../sdk/protocol/src/wire_types/custom_requests/dictation.rs) | 145 | Wire protocol specification, Local API envelopes and custom request DTOs | `DictationTranscribeRequest, DictationTranscribeResponse, DictationConfigRequest, DictationModelOption, DictationProviderStatusEntry, DictationConfigResponse` |
| [`sdk/protocol/src/wire_types/custom_requests/extension.rs`](../../sdk/protocol/src/wire_types/custom_requests/extension.rs) | 156 | Wire protocol specification, Local API envelopes and custom request DTOs | `GooseExtension, GooseExtensionEntry, SessionExtensionEntry, GetConfigExtensionsRequest, GetConfigExtensionsResponse, AddConfigExtensionRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/local_inference.rs`](../../sdk/protocol/src/wire_types/custom_requests/local_inference.rs) | 372 | Wire protocol specification, Local API envelopes and custom request DTOs | `LocalInferenceToolCallingMode, LocalInferenceChatTemplate, LocalInferenceSamplingConfig, LocalInferenceModelSettingsDto, LocalInferenceDownloadState, LocalInferenceModelDownloadStatusDto` |
| [`sdk/protocol/src/wire_types/custom_requests/mod.rs`](../../sdk/protocol/src/wire_types/custom_requests/mod.rs) | 30 | Submodule root: aggregates and re-exports sibling modules | None |
| [`sdk/protocol/src/wire_types/custom_requests/prompts.rs`](../../sdk/protocol/src/wire_types/custom_requests/prompts.rs) | 67 | Wire protocol specification, Local API envelopes and custom request DTOs | `PromptTemplateEntry, ListPromptsRequest, ListPromptsResponse, GetPromptRequest, GetPromptResponse, SavePromptRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/provider_config.rs`](../../sdk/protocol/src/wire_types/custom_requests/provider_config.rs) | 516 | Wire protocol specification, Local API envelopes and custom request DTOs | `ProviderConfigKey, ProviderConfigFieldValueDto, ProviderConfigStatusDto, ProviderConfigFieldUpdate, ProviderConfigReadRequest, ProviderConfigReadResponse` |
| [`sdk/protocol/src/wire_types/custom_requests/providers_inventory.rs`](../../sdk/protocol/src/wire_types/custom_requests/providers_inventory.rs) | 172 | Wire protocol specification, Local API envelopes and custom request DTOs | `ListProvidersRequest, ListProvidersResponse, ProviderReadinessCheckRequest, ProviderReadinessCheckResponse, ProviderSupportedModelsListRequest, ProviderSupportedModelsListResponse` |
| [`sdk/protocol/src/wire_types/custom_requests/recipe.rs`](../../sdk/protocol/src/wire_types/custom_requests/recipe.rs) | 416 | Wire protocol specification, Local API envelopes and custom request DTOs | `RecipeDto, RecipeAuthorDto, RecipeSettingsDto, RecipeResponseDto, SubRecipeDto, RecipeParameterDto` |
| [`sdk/protocol/src/wire_types/custom_requests/schedule.rs`](../../sdk/protocol/src/wire_types/custom_requests/schedule.rs) | 179 | Wire protocol specification, Local API envelopes and custom request DTOs | `ScheduledJobDto, ListSchedulesRequest, ListSchedulesResponse, CreateScheduleRequest, CreateScheduleResponse, DeleteScheduleRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/session.rs`](../../sdk/protocol/src/wire_types/custom_requests/session.rs) | 412 | Wire protocol specification, Local API envelopes and custom request DTOs | `AddSessionExtensionRequest, RemoveSessionExtensionRequest, GetToolsRequest, ToolListItem, GetToolsResponse, ReadResourceRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/sources.rs`](../../sdk/protocol/src/wire_types/custom_requests/sources.rs) | 264 | Wire protocol specification, Local API envelopes and custom request DTOs | `SourceType, SourceEntry, SourceScope, CreateSourceRequest, CreateSourceResponse, ListSourcesRequest` |
| [`sdk/protocol/src/wire_types/custom_requests/tests.rs`](../../sdk/protocol/src/wire_types/custom_requests/tests.rs) | 16 | Wire protocol specification, Local API envelopes and custom request DTOs | None |
| [`sdk/protocol/src/local_api/envelope.rs`](../../sdk/protocol/src/local_api/envelope.rs) | 197 | Wire protocol specification, Local API envelopes and custom request DTOs | `CommandEnvelope, AuthenticatedActor, EventEnvelope, ApiRequest, ApiResponse, new` |
| [`sdk/protocol/src/local_api/methods.rs`](../../sdk/protocol/src/local_api/methods.rs) | 195 | Wire protocol specification, Local API envelopes and custom request DTOs | None |
| [`sdk/protocol/src/local_api/mod.rs`](../../sdk/protocol/src/local_api/mod.rs) | 10 | Submodule root: aggregates and re-exports sibling modules | None |
| [`sdk/protocol/src/local_api/params.rs`](../../sdk/protocol/src/local_api/params.rs) | 314 | Wire protocol specification, Local API envelopes and custom request DTOs | `JobOperationalStatus, PacksJobCatalogItem, ListPacksJobsParams, GetPacksJobParams, ListPacksRolesParams, RunPacksJobRequest` |
| [`sdk/protocol/src/local_api/transport.rs`](../../sdk/protocol/src/local_api/transport.rs) | 9 | Wire protocol specification, Local API envelopes and custom request DTOs | `ApiTransport` |

### 3.13. Crate `custos-sdk` — Layer 4: Client Transports

- **Đường dẫn thư mục:** `sdk/client-rust`
- **Chủ sở hữu chính (Owner):** **Vinh**
- **Quy tắc ranh giới:** Phụ thuộc custos-protocol + custos-domain. Cấm phụ thuộc custos-core hay workbenches.
- **Tổng số tệp:** 10 tệp (9 tệp Rust) | **Tổng số dòng mã:** 1,512 dòng Rust.
- **Mô tả chức năng:** Bộ công cụ lập trình Client: LocalApiClient, ProcessTransport, TcpTransport, phân giải profile daemon và kết nối phi tập trung.

#### Danh mục các file bên trong `sdk/client-rust`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`sdk/client-rust/Cargo.toml`](../../sdk/client-rust/Cargo.toml) | 28 | Cargo manifest: package metadata and dependency configuration | None |
| [`sdk/client-rust/src/lib.rs`](../../sdk/client-rust/src/lib.rs) | 12 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`sdk/client-rust/src/profile.rs`](../../sdk/client-rust/src/profile.rs) | 74 | Official Rust SDK client: typed Local API RPC client and connection transports | `spawn_daemon_process, ensure_daemon_client_async, ensure_daemon_client` |
| [`sdk/client-rust/src/local_api/mod.rs`](../../sdk/client-rust/src/local_api/mod.rs) | 92 | Submodule root: aggregates and re-exports sibling modules | None |
| [`sdk/client-rust/src/local_api/tests.rs`](../../sdk/client-rust/src/local_api/tests.rs) | 64 | Official Rust SDK client: typed Local API RPC client and connection transports | None |
| [`sdk/client-rust/src/local_api/transport.rs`](../../sdk/client-rust/src/local_api/transport.rs) | 142 | Official Rust SDK client: typed Local API RPC client and connection transports | `FnTransport, ProcessTransport, TcpTransport, new, spawn, kill` |
| [`sdk/client-rust/src/local_api/client/mod.rs`](../../sdk/client-rust/src/local_api/client/mod.rs) | 64 | Submodule root: aggregates and re-exports sibling modules | `LocalApiClient, new, with_arc_transport, from_tcp, from_port, ping` |
| [`sdk/client-rust/src/local_api/client/research.rs`](../../sdk/client-rust/src/local_api/client/research.rs) | 459 | Official Rust SDK client: typed Local API RPC client and connection transports | `list_harnesses, get_harness, run_native_harness, list_artifacts, get_artifact, get_artifact_lineage_graph` |
| [`sdk/client-rust/src/local_api/client/tasks_sessions.rs`](../../sdk/client-rust/src/local_api/client/tasks_sessions.rs) | 342 | Official Rust SDK client: typed Local API RPC client and connection transports | `create_task, get_task, cancel_task, advance_task, complete_task, complete_task_with_evidence` |
| [`sdk/client-rust/src/local_api/client/workspaces.rs`](../../sdk/client-rust/src/local_api/client/workspaces.rs) | 263 | Official Rust SDK client: typed Local API RPC client and connection transports | `create_workspace, get_workspace, list_workspaces, archive_workspace, get_workspace_file_tree, read_workspace_file` |

### 3.14. Crate `custos-daemon` — Layer 4: Sole Composition Root

- **Đường dẫn thư mục:** `apps/daemon`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** Điểm ráp nối duy nhất của hệ thống. 4 sub-contexts rõ ràng; toàn bộ handler trong application/ là thin transport adapters.
- **Tổng số tệp:** 134 tệp (133 tệp Rust) | **Tổng số dòng mã:** 23,970 dòng Rust.
- **Mô tả chức năng:** Tiến trình nền hạt nhân: Local API server (HTTP/IPC), điều phối 4 ngữ cảnh (kernel, env, providers, workbenches), bootstrap SupervisedAgentRunner.

#### Danh mục các file bên trong `apps/daemon`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`apps/daemon/Cargo.toml`](../../apps/daemon/Cargo.toml) | 70 | Cargo manifest: package metadata and dependency configuration | None |
| [`apps/daemon/src/lib.rs`](../../apps/daemon/src/lib.rs) | 27 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`apps/daemon/src/main.rs`](../../apps/daemon/src/main.rs) | 147 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/profile.rs`](../../apps/daemon/src/profile.rs) | 178 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonLockError, DaemonLock, acquire, is_locked, release` |
| [`apps/daemon/src/runtime.rs`](../../apps/daemon/src/runtime.rs) | 188 | Custos daemon application service: API routing, auth and use-case dispatchers | `CustosRuntime, bootstrap_profile, bootstrap, bootstrap_with_model` |
| [`apps/daemon/src/bootstrap/background.rs`](../../apps/daemon/src/bootstrap/background.rs) | 81 | Custos daemon application service: API routing, auth and use-case dispatchers | `spawn_background_sweepers` |
| [`apps/daemon/src/bootstrap/coordinators.rs`](../../apps/daemon/src/bootstrap/coordinators.rs) | 164 | Custos daemon application service: API routing, auth and use-case dispatchers | `CoordinatorServices, init_coordinators_and_api` |
| [`apps/daemon/src/bootstrap/kernel.rs`](../../apps/daemon/src/bootstrap/kernel.rs) | 54 | Custos daemon application service: API routing, auth and use-case dispatchers | `KernelServices, init_kernel_services` |
| [`apps/daemon/src/bootstrap/mod.rs`](../../apps/daemon/src/bootstrap/mod.rs) | 21 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/bootstrap/storage.rs`](../../apps/daemon/src/bootstrap/storage.rs) | 15 | Custos daemon application service: API routing, auth and use-case dispatchers | `reconcile_storage_on_startup` |
| [`apps/daemon/src/bootstrap/workbenches.rs`](../../apps/daemon/src/bootstrap/workbenches.rs) | 142 | Custos daemon application service: API routing, auth and use-case dispatchers | `WorkbenchHarnessSetup, init_workbenches_and_harness` |
| [`apps/daemon/src/bin/custos-worker.rs`](../../apps/daemon/src/bin/custos-worker.rs) | 93 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/credential_store.rs`](../../apps/daemon/src/auth/credential_store.rs) | 499 | Custos daemon application service: API routing, auth and use-case dispatchers | `ProviderCredentials, new, initialize, get_oauth_token, save_oauth_token, save_provider` |
| [`apps/daemon/src/auth/credential_tests.rs`](../../apps/daemon/src/auth/credential_tests.rs) | 527 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/dispatcher.rs`](../../apps/daemon/src/auth/dispatcher.rs) | 58 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/mod.rs`](../../apps/daemon/src/auth/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/oauth_callback/html.rs`](../../apps/daemon/src/auth/oauth_callback/html.rs) | 78 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/oauth_callback/mod.rs`](../../apps/daemon/src/auth/oauth_callback/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/oauth_callback/server.rs`](../../apps/daemon/src/auth/oauth_callback/server.rs) | 474 | Custos daemon application service: API routing, auth and use-case dispatchers | `CallbackServerStatus, PendingOAuthContext, OAuthCallbackServer, new, with_credentials, start_copilot_device` |
| [`apps/daemon/src/auth/oauth_callback/tests.rs`](../../apps/daemon/src/auth/oauth_callback/tests.rs) | 118 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/oauth_callback/utils.rs`](../../apps/daemon/src/auth/oauth_callback/utils.rs) | 64 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/resolver/mod.rs`](../../apps/daemon/src/auth/resolver/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/resolver/resolver.rs`](../../apps/daemon/src/auth/resolver/resolver.rs) | 188 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonCredentialResolver` |
| [`apps/daemon/src/auth/resolver/selection.rs`](../../apps/daemon/src/auth/resolver/selection.rs) | 128 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/resolver/tests.rs`](../../apps/daemon/src/auth/resolver/tests.rs) | 452 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/http_server/handler.rs`](../../apps/daemon/src/http_server/handler.rs) | 339 | Custos daemon application service: API routing, auth and use-case dispatchers | `find_header_end, handle_http_client` |
| [`apps/daemon/src/http_server/mod.rs`](../../apps/daemon/src/http_server/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/http_server/security.rs`](../../apps/daemon/src/http_server/security.rs) | 63 | Custos daemon application service: API routing, auth and use-case dispatchers | `is_allowed_origin, is_authorized_ipc_client` |
| [`apps/daemon/src/http_server/server.rs`](../../apps/daemon/src/http_server/server.rs) | 41 | Custos daemon application service: API routing, auth and use-case dispatchers | `start_http_server` |
| [`apps/daemon/src/http_server/sse.rs`](../../apps/daemon/src/http_server/sse.rs) | 120 | Custos daemon application service: API routing, auth and use-case dispatchers | `write_live, handle_sse_events` |
| [`apps/daemon/src/http_server/tests.rs`](../../apps/daemon/src/http_server/tests.rs) | 192 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/admission.rs`](../../apps/daemon/src/api/admission.rs) | 105 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_request` |
| [`apps/daemon/src/api/builder.rs`](../../apps/daemon/src/api/builder.rs) | 342 | Custos daemon application service: API routing, auth and use-case dispatchers | `with_environment_coordinator, with_admission_coordinator, with_mcp_hub, mcp_hub, with_model_availability, with_sqlite_store` |
| [`apps/daemon/src/api/commands.rs`](../../apps/daemon/src/api/commands.rs) | 107 | Custos daemon application service: API routing, auth and use-case dispatchers | `MethodKind, classify_method, compute_command_fingerprint` |
| [`apps/daemon/src/api/contexts.rs`](../../apps/daemon/src/api/contexts.rs) | 49 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/mod.rs`](../../apps/daemon/src/api/mod.rs) | 390 | Submodule root: aggregates and re-exports sibling modules | `LocalApiDispatcher, LocalApiDependencies, ensure_module_registered, ensure_default_modules_registered, with_dependencies, new` |
| [`apps/daemon/src/api/tests/assistant.rs`](../../apps/daemon/src/api/tests/assistant.rs) | 412 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/assistant_only_isolation.rs`](../../apps/daemon/src/api/tests/assistant_only_isolation.rs) | 377 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/environment.rs`](../../apps/daemon/src/api/tests/environment.rs) | 109 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/harness.rs`](../../apps/daemon/src/api/tests/harness.rs) | 76 | Custos daemon application service: API routing, auth and use-case dispatchers | `MockStore, new` |
| [`apps/daemon/src/api/tests/mod.rs`](../../apps/daemon/src/api/tests/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/providers.rs`](../../apps/daemon/src/api/tests/providers.rs) | 375 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/sessions.rs`](../../apps/daemon/src/api/tests/sessions.rs) | 519 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/shared_capabilities.rs`](../../apps/daemon/src/api/tests/shared_capabilities.rs) | 549 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/research/lifecycle_and_artifacts.rs`](../../apps/daemon/src/api/tests/research/lifecycle_and_artifacts.rs) | 273 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/research/mod.rs`](../../apps/daemon/src/api/tests/research/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/research/notebooks_and_reviews.rs`](../../apps/daemon/src/api/tests/research/notebooks_and_reviews.rs) | 201 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/backtest.rs`](../../apps/daemon/src/api/tests/quant/backtest.rs) | 105 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/fixtures.rs`](../../apps/daemon/src/api/tests/quant/fixtures.rs) | 32 | Custos daemon application service: API routing, auth and use-case dispatchers | `setup_quant_env` |
| [`apps/daemon/src/api/tests/quant/market.rs`](../../apps/daemon/src/api/tests/quant/market.rs) | 130 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/mod.rs`](../../apps/daemon/src/api/tests/quant/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/quant/orders.rs`](../../apps/daemon/src/api/tests/quant/orders.rs) | 288 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/pipeline.rs`](../../apps/daemon/src/api/tests/quant/pipeline.rs) | 121 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/execution.rs`](../../apps/daemon/src/api/tests/tasks/execution.rs) | 269 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/lifecycle.rs`](../../apps/daemon/src/api/tests/tasks/lifecycle.rs) | 351 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/mod.rs`](../../apps/daemon/src/api/tests/tasks/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/tasks/packs.rs`](../../apps/daemon/src/api/tests/tasks/packs.rs) | 228 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/candidates.rs`](../../apps/daemon/src/api/tests/workspaces/candidates.rs) | 449 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/commands_and_outbox.rs`](../../apps/daemon/src/api/tests/workspaces/commands_and_outbox.rs) | 274 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/git_detection.rs`](../../apps/daemon/src/api/tests/workspaces/git_detection.rs) | 213 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/lifecycle.rs`](../../apps/daemon/src/api/tests/workspaces/lifecycle.rs) | 226 | Custos daemon application service: API routing, auth and use-case dispatchers | `hello` |
| [`apps/daemon/src/api/tests/workspaces/mod.rs`](../../apps/daemon/src/api/tests/workspaces/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/workspaces/races.rs`](../../apps/daemon/src/api/tests/workspaces/races.rs) | 253 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/tool_executor/executor.rs`](../../apps/daemon/src/tool_executor/executor.rs) | 549 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonGovernedToolExecutor, new, with_mcp_hub, with_pack_registry, with_supervision, parse_tool_invocation` |
| [`apps/daemon/src/tool_executor/mod.rs`](../../apps/daemon/src/tool_executor/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/tool_executor/parsing.rs`](../../apps/daemon/src/tool_executor/parsing.rs) | 134 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/tool_executor/tests.rs`](../../apps/daemon/src/tool_executor/tests.rs) | 101 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/blocks.rs`](../../apps/daemon/src/application/blocks.rs) | 133 | Custos daemon application service: API routing, auth and use-case dispatchers | `WorkbenchBlock, classify_method` |
| [`apps/daemon/src/application/capabilities.rs`](../../apps/daemon/src/application/capabilities.rs) | 559 | Custos daemon application service: API routing, auth and use-case dispatchers | `browser_execution_adapter_connected, remote_execution_adapter_connected, list_capabilities` |
| [`apps/daemon/src/application/compute.rs`](../../apps/daemon/src/application/compute.rs) | 424 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/environment.rs`](../../apps/daemon/src/application/environment.rs) | 96 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/execution.rs`](../../apps/daemon/src/application/execution.rs) | 328 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/mod.rs`](../../apps/daemon/src/application/mod.rs) | 127 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/oi.rs`](../../apps/daemon/src/application/oi.rs) | 44 | Custos daemon application service: API routing, auth and use-case dispatchers | `OiApiHandler, explain` |
| [`apps/daemon/src/application/pipeline.rs`](../../apps/daemon/src/application/pipeline.rs) | 219 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/sessions.rs`](../../apps/daemon/src/application/sessions.rs) | 279 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/system.rs`](../../apps/daemon/src/application/system.rs) | 191 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/tasks.rs`](../../apps/daemon/src/application/tasks.rs) | 269 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/mod.rs`](../../apps/daemon/src/application/research/mod.rs) | 44 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/research/runs.rs`](../../apps/daemon/src/application/research/runs.rs) | 113 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/sources.rs`](../../apps/daemon/src/application/research/sources.rs) | 285 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/handoff.rs`](../../apps/daemon/src/application/research/reviews/handoff.rs) | 129 | Review handoff saga transaction coordinator with create_task_batch atomic compensating rollback | None |
| [`apps/daemon/src/application/research/reviews/mod.rs`](../../apps/daemon/src/application/research/reviews/mod.rs) | 30 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/research/reviews/proposals.rs`](../../apps/daemon/src/application/research/reviews/proposals.rs) | 68 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/recipes.rs`](../../apps/daemon/src/application/research/reviews/recipes.rs) | 50 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/tests.rs`](../../apps/daemon/src/application/research/reviews/tests.rs) | 24 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/assistant/drafts.rs`](../../apps/daemon/src/application/assistant/drafts.rs) | 141 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_list, handle_get, handle_save, handle_delete` |
| [`apps/daemon/src/application/assistant/mod.rs`](../../apps/daemon/src/application/assistant/mod.rs) | 24 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/assistant/outbox.rs`](../../apps/daemon/src/application/assistant/outbox.rs) | 443 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_list, handle_create, handle_approve, handle_dispatch` |
| [`apps/daemon/src/application/quant/backtest.rs`](../../apps/daemon/src/application/quant/backtest.rs) | 107 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/market.rs`](../../apps/daemon/src/application/quant/market.rs) | 124 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/mod.rs`](../../apps/daemon/src/application/quant/mod.rs) | 40 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/orders.rs`](../../apps/daemon/src/application/quant/orders.rs) | 113 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/providers/crud.rs`](../../apps/daemon/src/application/providers/crud.rs) | 347 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/mod.rs`](../../apps/daemon/src/application/providers/mod.rs) | 41 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/providers/models.rs`](../../apps/daemon/src/application/providers/models.rs) | 518 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/oauth.rs`](../../apps/daemon/src/application/providers/oauth.rs) | 353 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/folders.rs`](../../apps/daemon/src/application/providers/local_models/folders.rs) | 169 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/lifecycle.rs`](../../apps/daemon/src/application/providers/local_models/lifecycle.rs) | 457 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/mod.rs`](../../apps/daemon/src/application/providers/local_models/mod.rs) | 20 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/github.rs`](../../apps/daemon/src/application/workspaces/github.rs) | 120 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/lifecycle.rs`](../../apps/daemon/src/application/workspaces/lifecycle.rs) | 274 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/mod.rs`](../../apps/daemon/src/application/workspaces/mod.rs) | 55 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/changes/compare.rs`](../../apps/daemon/src/application/workspaces/changes/compare.rs) | 178 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/integrate.rs`](../../apps/daemon/src/application/workspaces/changes/integrate.rs) | 315 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/mod.rs`](../../apps/daemon/src/application/workspaces/changes/mod.rs) | 23 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/changes/race.rs`](../../apps/daemon/src/application/workspaces/changes/race.rs) | 124 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/spawn.rs`](../../apps/daemon/src/application/workspaces/changes/spawn.rs) | 281 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/tests.rs`](../../apps/daemon/src/application/workspaces/changes/tests.rs) | 16 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/changes.rs`](../../apps/daemon/src/application/workspaces/inspection/changes.rs) | 157 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/detect.rs`](../../apps/daemon/src/application/workspaces/inspection/detect.rs) | 97 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/files.rs`](../../apps/daemon/src/application/workspaces/inspection/files.rs) | 110 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/mod.rs`](../../apps/daemon/src/application/workspaces/inspection/mod.rs) | 34 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/inspection/repository_git.rs`](../../apps/daemon/src/application/workspaces/inspection/repository_git.rs) | 128 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/tests.rs`](../../apps/daemon/src/application/workspaces/inspection/tests.rs) | 38 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/packs/mod.rs`](../../apps/daemon/src/application/packs/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/packs/package_management.rs`](../../apps/daemon/src/application/packs/package_management.rs) | 296 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/packs/workbench_catalog.rs`](../../apps/daemon/src/application/packs/workbench_catalog.rs) | 326 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/automation.rs`](../../apps/daemon/src/application/integrations/automation.rs) | 148 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/emulator.rs`](../../apps/daemon/src/application/integrations/emulator.rs) | 278 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/fleet.rs`](../../apps/daemon/src/application/integrations/fleet.rs) | 230 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/mod.rs`](../../apps/daemon/src/application/integrations/mod.rs) | 47 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/integrations/browser/inspection.rs`](../../apps/daemon/src/application/integrations/browser/inspection.rs) | 287 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/interaction.rs`](../../apps/daemon/src/application/integrations/browser/interaction.rs) | 160 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/mod.rs`](../../apps/daemon/src/application/integrations/browser/mod.rs) | 30 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/integrations/browser/navigation.rs`](../../apps/daemon/src/application/integrations/browser/navigation.rs) | 129 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/sessions.rs`](../../apps/daemon/src/application/integrations/browser/sessions.rs) | 61 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/tabs.rs`](../../apps/daemon/src/application/integrations/browser/tabs.rs) | 74 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/artifacts.rs`](../../apps/daemon/src/application/shared_capabilities/artifacts.rs) | 503 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/mod.rs`](../../apps/daemon/src/application/shared_capabilities/mod.rs) | 44 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/shared_capabilities/notebooks.rs`](../../apps/daemon/src/application/shared_capabilities/notebooks.rs) | 371 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/reviewer.rs`](../../apps/daemon/src/application/shared_capabilities/reviewer.rs) | 101 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workflow/mod.rs`](../../apps/daemon/src/application/workflow/mod.rs) | 275 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workflow/telemetry.rs`](../../apps/daemon/src/application/workflow/telemetry.rs) | 184 | Custos daemon application service: API routing, auth and use-case dispatchers | None |

### 3.15. Crate `custos-app-cli & custos-app-desktop` — Layer 4: Thin Presentation Clients

- **Đường dẫn thư mục:** `apps`
- **Chủ sở hữu chính (Owner):** **Vinh**
- **Quy tắc ranh giới:** apps/cli không phụ thuộc Tauri; apps/desktop/native là vỏ mỏng Tauri 2 ủy quyền toàn bộ cho Local API.
- **Tổng số tệp:** 142 tệp (139 tệp Rust) | **Tổng số dòng mã:** 24,821 dòng Rust.
- **Mô tả chức năng:** Giao diện đầu cuối: Headless pure-Rust CLI (apps/cli) và thin Tauri 2 native desktop app (apps/desktop/native).

#### Danh mục các file bên trong `apps`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`apps/desktop/native/Cargo.toml`](../../apps/desktop/native/Cargo.toml) | 29 | Cargo manifest: package metadata and dependency configuration | None |
| [`apps/desktop/native/build.rs`](../../apps/desktop/native/build.rs) | 3 | Native desktop shell host wrapper for Tauri 2 application | None |
| [`apps/desktop/native/src/browser.rs`](../../apps/desktop/native/src/browser.rs) | 266 | Native desktop shell host wrapper for Tauri 2 application | `BrowserRegistry, BrowserError, BrowserRequest, BrowserState, authorize, browser_command` |
| [`apps/desktop/native/src/lib.rs`](../../apps/desktop/native/src/lib.rs) | 179 | Crate root module: exports canonical public APIs and submodule declarations | `DesktopLifecycleManager, new, set_state, get_state, run` |
| [`apps/desktop/native/src/main.rs`](../../apps/desktop/native/src/main.rs) | 6 | Native desktop shell host wrapper for Tauri 2 application | None |
| [`apps/cli/Cargo.toml`](../../apps/cli/Cargo.toml) | 23 | Cargo manifest: package metadata and dependency configuration | None |
| [`apps/cli/src/lib.rs`](../../apps/cli/src/lib.rs) | 392 | Crate root module: exports canonical public APIs and submodule declarations | `CliArgs, CliCommand, TaskCommand, SupervisionCommand, CliTaskStatus, resolve_profile` |
| [`apps/cli/src/main.rs`](../../apps/cli/src/main.rs) | 5 | Headless pure-Rust command-line interface executable | None |
| [`apps/daemon/Cargo.toml`](../../apps/daemon/Cargo.toml) | 70 | Cargo manifest: package metadata and dependency configuration | None |
| [`apps/daemon/src/lib.rs`](../../apps/daemon/src/lib.rs) | 27 | Crate root module: exports canonical public APIs and submodule declarations | None |
| [`apps/daemon/src/main.rs`](../../apps/daemon/src/main.rs) | 147 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/profile.rs`](../../apps/daemon/src/profile.rs) | 178 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonLockError, DaemonLock, acquire, is_locked, release` |
| [`apps/daemon/src/runtime.rs`](../../apps/daemon/src/runtime.rs) | 188 | Custos daemon application service: API routing, auth and use-case dispatchers | `CustosRuntime, bootstrap_profile, bootstrap, bootstrap_with_model` |
| [`apps/daemon/src/bootstrap/background.rs`](../../apps/daemon/src/bootstrap/background.rs) | 81 | Custos daemon application service: API routing, auth and use-case dispatchers | `spawn_background_sweepers` |
| [`apps/daemon/src/bootstrap/coordinators.rs`](../../apps/daemon/src/bootstrap/coordinators.rs) | 164 | Custos daemon application service: API routing, auth and use-case dispatchers | `CoordinatorServices, init_coordinators_and_api` |
| [`apps/daemon/src/bootstrap/kernel.rs`](../../apps/daemon/src/bootstrap/kernel.rs) | 54 | Custos daemon application service: API routing, auth and use-case dispatchers | `KernelServices, init_kernel_services` |
| [`apps/daemon/src/bootstrap/mod.rs`](../../apps/daemon/src/bootstrap/mod.rs) | 21 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/bootstrap/storage.rs`](../../apps/daemon/src/bootstrap/storage.rs) | 15 | Custos daemon application service: API routing, auth and use-case dispatchers | `reconcile_storage_on_startup` |
| [`apps/daemon/src/bootstrap/workbenches.rs`](../../apps/daemon/src/bootstrap/workbenches.rs) | 142 | Custos daemon application service: API routing, auth and use-case dispatchers | `WorkbenchHarnessSetup, init_workbenches_and_harness` |
| [`apps/daemon/src/bin/custos-worker.rs`](../../apps/daemon/src/bin/custos-worker.rs) | 93 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/credential_store.rs`](../../apps/daemon/src/auth/credential_store.rs) | 499 | Custos daemon application service: API routing, auth and use-case dispatchers | `ProviderCredentials, new, initialize, get_oauth_token, save_oauth_token, save_provider` |
| [`apps/daemon/src/auth/credential_tests.rs`](../../apps/daemon/src/auth/credential_tests.rs) | 527 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/dispatcher.rs`](../../apps/daemon/src/auth/dispatcher.rs) | 58 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/mod.rs`](../../apps/daemon/src/auth/mod.rs) | 13 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/oauth_callback/html.rs`](../../apps/daemon/src/auth/oauth_callback/html.rs) | 78 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/oauth_callback/mod.rs`](../../apps/daemon/src/auth/oauth_callback/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/oauth_callback/server.rs`](../../apps/daemon/src/auth/oauth_callback/server.rs) | 474 | Custos daemon application service: API routing, auth and use-case dispatchers | `CallbackServerStatus, PendingOAuthContext, OAuthCallbackServer, new, with_credentials, start_copilot_device` |
| [`apps/daemon/src/auth/oauth_callback/tests.rs`](../../apps/daemon/src/auth/oauth_callback/tests.rs) | 118 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/oauth_callback/utils.rs`](../../apps/daemon/src/auth/oauth_callback/utils.rs) | 64 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/resolver/mod.rs`](../../apps/daemon/src/auth/resolver/mod.rs) | 14 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/auth/resolver/resolver.rs`](../../apps/daemon/src/auth/resolver/resolver.rs) | 188 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonCredentialResolver` |
| [`apps/daemon/src/auth/resolver/selection.rs`](../../apps/daemon/src/auth/resolver/selection.rs) | 128 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/auth/resolver/tests.rs`](../../apps/daemon/src/auth/resolver/tests.rs) | 452 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/http_server/handler.rs`](../../apps/daemon/src/http_server/handler.rs) | 339 | Custos daemon application service: API routing, auth and use-case dispatchers | `find_header_end, handle_http_client` |
| [`apps/daemon/src/http_server/mod.rs`](../../apps/daemon/src/http_server/mod.rs) | 12 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/http_server/security.rs`](../../apps/daemon/src/http_server/security.rs) | 63 | Custos daemon application service: API routing, auth and use-case dispatchers | `is_allowed_origin, is_authorized_ipc_client` |
| [`apps/daemon/src/http_server/server.rs`](../../apps/daemon/src/http_server/server.rs) | 41 | Custos daemon application service: API routing, auth and use-case dispatchers | `start_http_server` |
| [`apps/daemon/src/http_server/sse.rs`](../../apps/daemon/src/http_server/sse.rs) | 120 | Custos daemon application service: API routing, auth and use-case dispatchers | `write_live, handle_sse_events` |
| [`apps/daemon/src/http_server/tests.rs`](../../apps/daemon/src/http_server/tests.rs) | 192 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/admission.rs`](../../apps/daemon/src/api/admission.rs) | 105 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_request` |
| [`apps/daemon/src/api/builder.rs`](../../apps/daemon/src/api/builder.rs) | 342 | Custos daemon application service: API routing, auth and use-case dispatchers | `with_environment_coordinator, with_admission_coordinator, with_mcp_hub, mcp_hub, with_model_availability, with_sqlite_store` |
| [`apps/daemon/src/api/commands.rs`](../../apps/daemon/src/api/commands.rs) | 107 | Custos daemon application service: API routing, auth and use-case dispatchers | `MethodKind, classify_method, compute_command_fingerprint` |
| [`apps/daemon/src/api/contexts.rs`](../../apps/daemon/src/api/contexts.rs) | 49 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/mod.rs`](../../apps/daemon/src/api/mod.rs) | 390 | Submodule root: aggregates and re-exports sibling modules | `LocalApiDispatcher, LocalApiDependencies, ensure_module_registered, ensure_default_modules_registered, with_dependencies, new` |
| [`apps/daemon/src/api/tests/assistant.rs`](../../apps/daemon/src/api/tests/assistant.rs) | 412 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/assistant_only_isolation.rs`](../../apps/daemon/src/api/tests/assistant_only_isolation.rs) | 377 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/environment.rs`](../../apps/daemon/src/api/tests/environment.rs) | 109 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/harness.rs`](../../apps/daemon/src/api/tests/harness.rs) | 76 | Custos daemon application service: API routing, auth and use-case dispatchers | `MockStore, new` |
| [`apps/daemon/src/api/tests/mod.rs`](../../apps/daemon/src/api/tests/mod.rs) | 15 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/providers.rs`](../../apps/daemon/src/api/tests/providers.rs) | 375 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/sessions.rs`](../../apps/daemon/src/api/tests/sessions.rs) | 519 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/shared_capabilities.rs`](../../apps/daemon/src/api/tests/shared_capabilities.rs) | 549 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/research/lifecycle_and_artifacts.rs`](../../apps/daemon/src/api/tests/research/lifecycle_and_artifacts.rs) | 273 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/research/mod.rs`](../../apps/daemon/src/api/tests/research/mod.rs) | 2 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/research/notebooks_and_reviews.rs`](../../apps/daemon/src/api/tests/research/notebooks_and_reviews.rs) | 201 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/backtest.rs`](../../apps/daemon/src/api/tests/quant/backtest.rs) | 105 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/fixtures.rs`](../../apps/daemon/src/api/tests/quant/fixtures.rs) | 32 | Custos daemon application service: API routing, auth and use-case dispatchers | `setup_quant_env` |
| [`apps/daemon/src/api/tests/quant/market.rs`](../../apps/daemon/src/api/tests/quant/market.rs) | 130 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/mod.rs`](../../apps/daemon/src/api/tests/quant/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/quant/orders.rs`](../../apps/daemon/src/api/tests/quant/orders.rs) | 288 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/quant/pipeline.rs`](../../apps/daemon/src/api/tests/quant/pipeline.rs) | 121 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/execution.rs`](../../apps/daemon/src/api/tests/tasks/execution.rs) | 269 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/lifecycle.rs`](../../apps/daemon/src/api/tests/tasks/lifecycle.rs) | 351 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/tasks/mod.rs`](../../apps/daemon/src/api/tests/tasks/mod.rs) | 4 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/tasks/packs.rs`](../../apps/daemon/src/api/tests/tasks/packs.rs) | 228 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/candidates.rs`](../../apps/daemon/src/api/tests/workspaces/candidates.rs) | 449 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/commands_and_outbox.rs`](../../apps/daemon/src/api/tests/workspaces/commands_and_outbox.rs) | 274 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/git_detection.rs`](../../apps/daemon/src/api/tests/workspaces/git_detection.rs) | 213 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/api/tests/workspaces/lifecycle.rs`](../../apps/daemon/src/api/tests/workspaces/lifecycle.rs) | 226 | Custos daemon application service: API routing, auth and use-case dispatchers | `hello` |
| [`apps/daemon/src/api/tests/workspaces/mod.rs`](../../apps/daemon/src/api/tests/workspaces/mod.rs) | 5 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/api/tests/workspaces/races.rs`](../../apps/daemon/src/api/tests/workspaces/races.rs) | 253 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/tool_executor/executor.rs`](../../apps/daemon/src/tool_executor/executor.rs) | 549 | Custos daemon application service: API routing, auth and use-case dispatchers | `DaemonGovernedToolExecutor, new, with_mcp_hub, with_pack_registry, with_supervision, parse_tool_invocation` |
| [`apps/daemon/src/tool_executor/mod.rs`](../../apps/daemon/src/tool_executor/mod.rs) | 8 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/tool_executor/parsing.rs`](../../apps/daemon/src/tool_executor/parsing.rs) | 134 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/tool_executor/tests.rs`](../../apps/daemon/src/tool_executor/tests.rs) | 101 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/blocks.rs`](../../apps/daemon/src/application/blocks.rs) | 133 | Custos daemon application service: API routing, auth and use-case dispatchers | `WorkbenchBlock, classify_method` |
| [`apps/daemon/src/application/capabilities.rs`](../../apps/daemon/src/application/capabilities.rs) | 559 | Custos daemon application service: API routing, auth and use-case dispatchers | `browser_execution_adapter_connected, remote_execution_adapter_connected, list_capabilities` |
| [`apps/daemon/src/application/compute.rs`](../../apps/daemon/src/application/compute.rs) | 424 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/environment.rs`](../../apps/daemon/src/application/environment.rs) | 96 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/execution.rs`](../../apps/daemon/src/application/execution.rs) | 328 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/mod.rs`](../../apps/daemon/src/application/mod.rs) | 127 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/oi.rs`](../../apps/daemon/src/application/oi.rs) | 44 | Custos daemon application service: API routing, auth and use-case dispatchers | `OiApiHandler, explain` |
| [`apps/daemon/src/application/pipeline.rs`](../../apps/daemon/src/application/pipeline.rs) | 219 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/sessions.rs`](../../apps/daemon/src/application/sessions.rs) | 279 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/system.rs`](../../apps/daemon/src/application/system.rs) | 191 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/tasks.rs`](../../apps/daemon/src/application/tasks.rs) | 269 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/mod.rs`](../../apps/daemon/src/application/research/mod.rs) | 44 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/research/runs.rs`](../../apps/daemon/src/application/research/runs.rs) | 113 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/sources.rs`](../../apps/daemon/src/application/research/sources.rs) | 285 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/handoff.rs`](../../apps/daemon/src/application/research/reviews/handoff.rs) | 129 | Review handoff saga transaction coordinator with create_task_batch atomic compensating rollback | None |
| [`apps/daemon/src/application/research/reviews/mod.rs`](../../apps/daemon/src/application/research/reviews/mod.rs) | 30 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/research/reviews/proposals.rs`](../../apps/daemon/src/application/research/reviews/proposals.rs) | 68 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/recipes.rs`](../../apps/daemon/src/application/research/reviews/recipes.rs) | 50 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/research/reviews/tests.rs`](../../apps/daemon/src/application/research/reviews/tests.rs) | 24 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/assistant/drafts.rs`](../../apps/daemon/src/application/assistant/drafts.rs) | 141 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_list, handle_get, handle_save, handle_delete` |
| [`apps/daemon/src/application/assistant/mod.rs`](../../apps/daemon/src/application/assistant/mod.rs) | 24 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/assistant/outbox.rs`](../../apps/daemon/src/application/assistant/outbox.rs) | 443 | Custos daemon application service: API routing, auth and use-case dispatchers | `handle_list, handle_create, handle_approve, handle_dispatch` |
| [`apps/daemon/src/application/quant/backtest.rs`](../../apps/daemon/src/application/quant/backtest.rs) | 107 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/market.rs`](../../apps/daemon/src/application/quant/market.rs) | 124 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/mod.rs`](../../apps/daemon/src/application/quant/mod.rs) | 40 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/quant/orders.rs`](../../apps/daemon/src/application/quant/orders.rs) | 113 | Thin transport adapter for Quant workbench operations (no domain math, 100% delegated to workbench) | None |
| [`apps/daemon/src/application/providers/crud.rs`](../../apps/daemon/src/application/providers/crud.rs) | 347 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/mod.rs`](../../apps/daemon/src/application/providers/mod.rs) | 41 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/providers/models.rs`](../../apps/daemon/src/application/providers/models.rs) | 518 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/oauth.rs`](../../apps/daemon/src/application/providers/oauth.rs) | 353 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/folders.rs`](../../apps/daemon/src/application/providers/local_models/folders.rs) | 169 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/lifecycle.rs`](../../apps/daemon/src/application/providers/local_models/lifecycle.rs) | 457 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/providers/local_models/mod.rs`](../../apps/daemon/src/application/providers/local_models/mod.rs) | 20 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/github.rs`](../../apps/daemon/src/application/workspaces/github.rs) | 120 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/lifecycle.rs`](../../apps/daemon/src/application/workspaces/lifecycle.rs) | 274 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/mod.rs`](../../apps/daemon/src/application/workspaces/mod.rs) | 55 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/changes/compare.rs`](../../apps/daemon/src/application/workspaces/changes/compare.rs) | 178 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/integrate.rs`](../../apps/daemon/src/application/workspaces/changes/integrate.rs) | 315 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/mod.rs`](../../apps/daemon/src/application/workspaces/changes/mod.rs) | 23 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/changes/race.rs`](../../apps/daemon/src/application/workspaces/changes/race.rs) | 124 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/spawn.rs`](../../apps/daemon/src/application/workspaces/changes/spawn.rs) | 281 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/changes/tests.rs`](../../apps/daemon/src/application/workspaces/changes/tests.rs) | 16 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/changes.rs`](../../apps/daemon/src/application/workspaces/inspection/changes.rs) | 157 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/detect.rs`](../../apps/daemon/src/application/workspaces/inspection/detect.rs) | 97 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/files.rs`](../../apps/daemon/src/application/workspaces/inspection/files.rs) | 110 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/mod.rs`](../../apps/daemon/src/application/workspaces/inspection/mod.rs) | 34 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workspaces/inspection/repository_git.rs`](../../apps/daemon/src/application/workspaces/inspection/repository_git.rs) | 128 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workspaces/inspection/tests.rs`](../../apps/daemon/src/application/workspaces/inspection/tests.rs) | 38 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/packs/mod.rs`](../../apps/daemon/src/application/packs/mod.rs) | 19 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/packs/package_management.rs`](../../apps/daemon/src/application/packs/package_management.rs) | 296 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/packs/workbench_catalog.rs`](../../apps/daemon/src/application/packs/workbench_catalog.rs) | 326 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/automation.rs`](../../apps/daemon/src/application/integrations/automation.rs) | 148 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/emulator.rs`](../../apps/daemon/src/application/integrations/emulator.rs) | 278 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/fleet.rs`](../../apps/daemon/src/application/integrations/fleet.rs) | 230 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/mod.rs`](../../apps/daemon/src/application/integrations/mod.rs) | 47 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/integrations/browser/inspection.rs`](../../apps/daemon/src/application/integrations/browser/inspection.rs) | 287 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/interaction.rs`](../../apps/daemon/src/application/integrations/browser/interaction.rs) | 160 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/mod.rs`](../../apps/daemon/src/application/integrations/browser/mod.rs) | 30 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/integrations/browser/navigation.rs`](../../apps/daemon/src/application/integrations/browser/navigation.rs) | 129 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/sessions.rs`](../../apps/daemon/src/application/integrations/browser/sessions.rs) | 61 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/integrations/browser/tabs.rs`](../../apps/daemon/src/application/integrations/browser/tabs.rs) | 74 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/artifacts.rs`](../../apps/daemon/src/application/shared_capabilities/artifacts.rs) | 503 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/mod.rs`](../../apps/daemon/src/application/shared_capabilities/mod.rs) | 44 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/shared_capabilities/notebooks.rs`](../../apps/daemon/src/application/shared_capabilities/notebooks.rs) | 371 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/shared_capabilities/reviewer.rs`](../../apps/daemon/src/application/shared_capabilities/reviewer.rs) | 101 | Custos daemon application service: API routing, auth and use-case dispatchers | None |
| [`apps/daemon/src/application/workflow/mod.rs`](../../apps/daemon/src/application/workflow/mod.rs) | 275 | Submodule root: aggregates and re-exports sibling modules | None |
| [`apps/daemon/src/application/workflow/telemetry.rs`](../../apps/daemon/src/application/workflow/telemetry.rs) | 184 | Custos daemon application service: API routing, auth and use-case dispatchers | None |

### 3.16. Giao Diện Người Dùng Desktop (`ui/desktop`)

- **Đường dẫn thư mục:** `ui/desktop`
- **Chủ sở hữu chính (Owner):** **Vinh**
- **Quy tắc ranh giới:** Tuyệt đối Zero-Mock trong mã nguồn sản phẩm; tuân thủ Graphite JetBlack design tokens; trần $\le 600$ dòng cho mọi tệp; vượt qua `npm run build` và `tests/desktop/canvas-smoke.mjs` (42/42 tests).
- **Tổng số tệp:** 311 tệp mã nguồn UI (`.ts`, `.tsx`, `.css`) | **Tổng số dòng mã:** 54,642 dòng.
- **Mô tả chức năng:** Giao diện Desktop React 19 / Vite hỗ trợ đầy đủ 4 workbenches (`copilot`, `engineering`, `research`, `quant`), điều hướng trung tâm FilesChangesPane, ScreenSwitcherBar, ActivityRail, và hệ thống Run Inspector 3-tier.

### 3.17. Bộ Kiểm Thử & Tự Động Hóa Workspace (`tests/` & `xtask/`)

- **Đường dẫn thư mục:** `tests/` và `xtask/`
- **Chủ sở hữu chính (Owner):** **Vĩ + Vinh**
- **Tổng số tệp:** 41 tệp Rust | **Tổng số dòng mã:** 12,628 dòng.
- **Mô tả chức năng:**
  - `tests/contract/`: Kiểm tra bất biến hợp đồng và Zero-I/O boundary.
  - `tests/e2e/`: Kiểm tra tích hợp toàn trình (Golden Acceptance Tests G06-G12, `oi_topologies_slice.rs` Theme 1-5).
  - `tests/testkit/`: Môi trường giả lập cho test suite, cô lập hoàn toàn dưới `#[cfg(any(test, feature = "test-support"))]`.
  - `xtask/`: Bộ công cụ tự động hóa kiểm tra ranh giới (`check-boundaries`), giới hạn số dòng (`check-file-line-ceiling`) và cổng phát hành 5 bước (`cargo xtask gate`).

---

## 4. Các Phân Hệ & Công Cụ Bổ Trợ Workspace

### 4.1. Hệ Thống Kiểm Thử Toàn Trình (`tests/`)
Custos áp dụng kim tự tháp kiểm thử 5 tầng nghiêm ngặt:
1. **Unit Tests:** Kiểm tra logic thuần túy trong từng crate (nhanh, 100% in-memory).
2. **Contract Tests (`tests/contract/`):** Kiểm tra tính bất biến của ActorId, Permit, Evidence, Scope và Zero-I/O boundaries.
3. **Golden Acceptance Tests (`tests/e2e/`):** Bộ kiểm thử vàng G06–G12 bảo đảm hành vi tích hợp đa phân hệ:
   - `G06`: Notebook Document & Kernel Scoped Binding.
   - `G07`: Governed Agent Loop, Single-Receipt & Double-Dispatch Prevention.
   - `G08`: Crash Recovery & Uncertain Status Reconciliation.
   - `G09`: Evidence Integrity & Completion Gate.
   - `G10`: Three-Vertical Multi-Workbench Acceptance.
   - `G11`: Compute Relay & Checkpoint Durability.
   - `G12`: Measured Orchestration Intelligence & Economic Optimization.
4. **OI Topologies Slice (`tests/e2e/tests/oi_topologies_slice.rs`):** Kiểm thử 5 chủ đề tô pô thực thi (Theme 1: Direct Pinned, Theme 2: Dual Judge, Theme 3: Speculative Verification, Theme 4: Hierarchical Router, Theme 5: T5 Lead Worker DAG Coordinator).
5. **Desktop Canvas Smoke (`tests/desktop/canvas-smoke.mjs`):** 42 kịch bản kiểm tra giao diện tự động trên Headless Chromium.

### 4.2. Hệ Thống Kiểm Soát Chất Lượng (`cargo xtask gate`)
Trước mỗi lần đóng gói phát hành, lệnh `cargo xtask gate` thực thi 5 cổng chất lượng tự động:
- **Gate 1:** Kiểm tra tính toàn vẹn của Manifest các Workbench.
- **Gate 2:** Biên dịch giao diện Desktop UI (`npm --prefix ui/desktop run build`).
- **Gate 3:** Kiểm tra khói giao diện Canvas Desktop (`node tests/desktop/canvas-smoke.mjs`).
- **Gate 4:** Kiểm tra các bất biến kiến trúc và hợp đồng (`check-boundaries`, `check-file-line-ceiling`, `check_deps.sh`).
- **Gate 5:** Kiểm tra biên dịch toàn bộ workspace Cargo (`cargo check --workspace --all-targets`).

---

## 5. Quy Tắc Bất Biến & Tổng Kết Tái Cấu Trúc Toàn Diện (Phases 1 - 5)

### SADE Master Refactoring & Verification (Phases 1 - 5)

Dự án đã hoàn thành toàn diện 5 giai đoạn tái cấu trúc SuperAgent & Orchestration Intelligence:

1. **Giai Đoạn 1 — Chuẩn Hóa Danh Xưng CLI Agent & Supervision Decorator:**
   - `platform/core/src/contracts/agent.rs`: Định nghĩa trait `AgentDriver` chuẩn hóa (re-export `AgentRuntimePort`), thay thế dứt điểm thuật ngữ mơ hồ `harness`.
   - `platform/adapters/src/harnesses/adapter.rs`: Cung cấp alias `CliAgentAdapter`; cô lập toàn bộ `mock_state` dưới `#[cfg(any(test, feature = "test-support"))]` tuân thủ nghiêm ngặt Zero-Mock.
   - `platform/adapters/src/harnesses/stream/process.rs`: Tích hợp `setpgid(0)` trên Unix và `ProcessDropGuard` tự động gửi `SIGKILL` đến toàn bộ process group (`-pid`) khi drop, loại bỏ triệt để nguy cơ tiến trình ma (zombie process).
   - `platform/runtime/src/agent/runner.rs`: Hiện thực hóa `SupervisedAgentRunner` decorator (265 dòng), tự động bọc mọi lệnh gọi agent để thu thập `StepSpan`, đẩy dữ liệu về `SupervisionTraceSink`, và cảnh báo tức thời qua `SupervisedObserver`.
   - `apps/daemon/src/bootstrap/workbenches.rs`: Bọc toàn bộ agent CLI và runtime nội bộ qua `SupervisedAgentRunner`.

2. **Giai Đoạn 2 — Kiểm Toán apps/daemon Layering & Thin Transport Verification:**
   - Kiểm toán toàn bộ `apps/daemon/src/application/quant/{backtest,orders,market}.rs`: Xác nhận 100% là thin transport adapters không chứa bất kỳ phép toán tài chính hay quy tắc nghiệp vụ rò rỉ nào.
   - Tái cấu trúc `apps/daemon/src/application/research/reviews/handoff.rs`: Tách logic bù trừ (compensating transaction) vào helper `create_task_batch` (129 dòng), bảo đảm tính nguyên tử khi tạo batch task.

3. **Giai Đoạn 3 — Hoàn Thiện OI Brain & Tô Pô T5 Lead-Worker DAG:**
   - `platform/runtime/src/oi/topologies/t5_lead_worker.rs`: Hiện thực bộ điều phối T5 Lead-Worker DAG (271 dòng) sử dụng thuật toán Kahn để sắp xếp tô pô, phát hiện chu trình (cycle detection), kiểm tra phụ thuộc và tổng hợp kết quả qua cổng `CoverageThreshold`.
   - `platform/runtime/src/oi/selector.rs`: Nâng cấp bộ chọn Pareto Multi-Objective Selector tối ưu hóa hàm thỏa dụng $U = w_q \cdot Q - w_c \cdot C - w_t \cdot T$ kèm chuẩn hóa chi phí/độ trễ động và chiến lược ngữ cảnh neo cache.
   - `platform/runtime/src/oi/estimator.rs`: Tích hợp Model Pricing Catalog thực tế (Claude 3.5 Sonnet, Claude 3.5 Haiku, GPT-4o, GPT-4o-mini, Gemini 1.5, Local) và cơ chế Prompt Cache Hit Affinity (chiết khấu đọc 0.1x / ghi 0.5x).
   - `platform/runtime/src/oi/compiler.rs` & `benchmark.rs`: Tích hợp nhánh biên dịch và đánh giá cho `ExecutionTopology::T5LeadWorker`.
   - `tests/e2e/tests/oi_topologies_slice.rs`: Bổ sung Theme 5 kiểm thử tích hợp T5 Lead-Worker DAG (5/5 tests PASS).

4. **Giai Đoạn 4 — FastPath Ingress & Partitioned Storage Engine:**
   - `platform/runtime/src/agent/fast_path.rs`: Hiện thực bộ lọc `FastPathFilter` 0-Token (< 1ms, $0.00 chi phí LLM) bằng regex engine cho các lệnh hệ thống xác định (help, ping, version, clear, workbenches, git status/diff/log), bỏ qua hoàn toàn mô hình suy luận.
   - `platform/storage/src/connection.rs`: Bổ sung cơ chế `attach_partition` và `detach_partition` trên `DbConnection` có kiểm tra an toàn chống SQL injection, mở rộng khả năng mở rộng đa cơ sở dữ liệu SQLite.

5. **Giai Đoạn 5 — Nghiệm Thu Cổng Chất Lượng Tự Động:**
   - `cargo xtask check-boundaries`: PASS (0 rò rỉ platform-workbench, T1–T5 topic cohesion chuẩn mực).
   - `cargo xtask check-file-line-ceiling`: PASS (100% tệp nguồn $\le 600$ dòng mã).
   - `cargo xtask gate`: PASS 5/5 Cổng chất lượng tuyệt đối.
   - `cargo check --workspace --all-targets --offline`: PASS (Exit code 0).
