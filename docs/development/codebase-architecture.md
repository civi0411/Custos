# Monorepo Topology, Crate Boundaries & Deep Codebase Map

> **Backend folder setup index:** Logical architecture labels are mapped onto the
> canonical crates rather than implemented as parallel crates. Core maps to
> `custos-domain`/`custos-core`; Provider and Router to
> `custos-provider`/`custos-runtime` with implementations in `custos-adapters`;
> Gateway to `custos-daemon`/`custos-bridge`; MCP, A2A and ACP to
> `custos-adapters`; Policy to `custos-core`; Storage to `custos-persistence`;
> and applications to `custos-app/{cli,desktop}`. `custos-daemon` is an explicit
> root workspace member and remains the sole composition root. See
> [protocol and connectivity boundaries](../architecture/protocol-and-connectivity-hubs.md#4-physical-ownership-and-current-state-map).

> **Desktop resource-tab ownership:** [`ui/desktop/src/app/studio/page.tsx`](../../ui/desktop/src/app/studio/page.tsx) owns selected resource tabs above all three lens views; [`ResourceTabbedPane.tsx`](../../ui/desktop/src/components/views/ResourceTabbedPane.tsx) (aliased as `OrcaTabbedContainer.tsx` for compatibility) renders resource picker and content; [`ChatView.tsx`](../../ui/desktop/src/components/views/ChatView.tsx), [`CodeView.tsx`](../../ui/desktop/src/components/views/CodeView.tsx), and [`ResearchView.tsx`](../../ui/desktop/src/components/views/ResearchView.tsx) host that same pane. Research resources show explicit unconnected states until versioned backend APIs exist. This changes frontend presentation ownership only; no crate, schema, or backend authority moves. Historical file counts below remain unchanged.

> **Conversation continuity/research target, not implemented:** [UI architecture §12](../architecture/agent-workspace-and-ui.md#12-lịch-sử-hội-thoại-và-chuyển-workbench) specifies one canonical session journal with per-lens history projections, plus optional linked continuation manifest/context receipt. Current [`domain/session.rs`](../../crates/custos-domain/src/session.rs) has one `attached_to` Task and untyped journal entry content; [`daemon/api.rs`](../../crates/custos-daemon/src/api.rs) exposes create/get/list/journal/message/attach but no cross-workbench continuation transaction. Future contracts belong in domain/core or bridge, transactional records/migrations in persistence, orchestration/context compilation in runtime, Research semantics in packs, provider/compute I/O in adapters, and DTO/API composition in daemon/SDK. [`StudioPage`](../../ui/desktop/src/app/studio/page.tsx) can change lens and preserve in-memory resource tabs but cannot truthfully transfer model-visible history or native harness state today. Do not treat this target index as a migration result.

> **Continuity implementation map:** [Target contracts and migration sequence](../architecture/agent-workspace-and-ui.md#124-hợp-đồng-danh-tính-và-lịch-sử) place typed turn/lineage/receipt values in existing `custos-domain`, authority and Task-binding policy in `custos-core`/`custos-bridge`, idempotent session-link and membership records in `custos-persistence`, context compilation and attempt attribution in `custos-runtime`, Research source/claim/experiment semantics in `custos-packs`, compute/metadata I/O in `custos-adapters`, and API composition in `custos-daemon`. `session_messages` and `session_journal` already coexist; implementation must select one transcript source of truth before new writes and keep the other rebuildable. No new source file, migration, API or crate is claimed by this documentation map; update the physical file tables and counts when implementing each slice.

> **Open Science source map, target only:** [Pinned source study](open-science-source-study.md) maps its OpenCode/SDK, pane layout, notebook kernel, provenance/run records, reviewer and connector code to Custos's existing boundaries. Current Custos [`ResearchView.tsx`](../../ui/desktop/src/components/views/ResearchView.tsx) has starter prompts and resource tabs; [`research/mod.rs`](../../crates/custos-packs/src/research/mod.rs) is a pack descriptor plus selected skills/verifiers, not the complete Open Science loop. For implementation, keep view/pane code in `ui/desktop`, source/claim/experiment semantics in `custos-packs`, reusable context/run coordination in `custos-runtime`, approved compute and connector I/O in `custos-adapters`, canonical artifact/attempt state in `custos-persistence` behind core ports, and daemon as sole composition root. No Open Science code was copied into the crates by this map.

> **Crate blueprint audit:** [Plan §22](workspace-restructuring-plan.md#22-crate-blueprint-và-chuyển-lõi-orca-theo-trách-nhiệm) records production Cargo edges and mounted exports. Runtime retains a persistence manifest edge (no concrete imports found in the inspected source search); bridge persistence is dev-only; daemon currently has no packs dependency. [`workflow/lease.rs`](../../crates/custos-runtime/src/workflow/lease.rs) implements directory creation/file copying with in-memory ownership, not Git worktree allocation; [`workflow/dispatcher.rs`](../../crates/custos-runtime/src/workflow/dispatcher.rs) is a skeletal struct. These are source findings, not migrations. Current [`harness/claude_code.rs`](../../crates/custos-adapters/src/harness/claude_code.rs) has subprocess functionality to retain and verify. Proposed workspace/mailbox/atomic repository contracts remain target until implemented; no source counts change in this documentation pass.

> **Desktop/headless integration index:** [Superplan §21](workspace-restructuring-plan.md#21-superplan-kết-hợp-custos-và-orca-cho-desktop-và-headless) defines concrete reuse/rewrites, current-source evidence, target module owners, packet write zones, migrations and stream/projection contracts. Mobile is excluded. Nexus read-only inventory counted 1,076 Custos files and 32,534 OrCa files under its ignore rules; these are inventory counts, not production-source coverage or fresh AST verification. This documentation update creates no runtime module, migration or frontend port; source catalog rows remain subject to per-packet regeneration.

> **SADE economics/domain implementation index:** [Plan §11–19](workspace-restructuring-plan.md#11-quyết-định-khóa-cho-chiến-dịch-sade) maps the current live-execution gaps, simulated OI executor, in-memory budget reservations, existing `0004_usage.sql` ledger and pack-verifier limitations to W0–W5. Cost policy belongs in core, route estimation in runtime, durable accounting in persistence, domain semantics/verifiers in packs, concrete model/native execution in adapters and wiring in daemon. [Three-pack blueprints](../architecture/domain-packs-and-workflows.md) specify behavior; no source module or crate was created by this docs-only update. Five targeted e2e fixtures pass but do not establish live/native execution or measured savings.

> **OrCa comparison index:** [Pinned source study](orca-source-study.md) maps OrCa runtime/agent launch/orchestration/worktree/UI evidence to target Custos ownership. It adds no Custos source files or crate and does not make the historical file counts below current. Nexus supplied read-only OrCa inventory, not a TypeScript AST/call graph. Source changes must still update the physical catalog at the matching crate after implementation.

> **Current code-refactor audit:** [Plan sections 6–10](workspace-restructuring-plan.md#6-audit-code-và-quyết-định-giữchuyểnhợp-nhất) record a fresh manifest/module inspection: `cargo metadata --no-deps --offline` includes 16 packages, 12 product packages including both app hosts; daemon is included via path dependencies. `custos-app/cli` currently launches a Tauri host, not a headless Rust CLI. Daemon owns wire DTO/client code today; the target moves that client surface into SDK. Runtime has a production persistence dependency and duplicate memory trait definitions; these are migration findings, not claims files were moved. Source tables/counts below remain historical inventory until their own regeneration; this docs-only plan adds no Rust source or crate.

> **SADE design index:** [Supervision and outcome economics](../architecture/sade-design-and-supervision.md) deepens Master Parts 1/14/16. SADE introduces no new crate or source relocation: experience stays in UI, coordination in runtime, authority in core, domain verification in packs, concrete execution in adapters, composition in daemon. Existing source inventory counts are unchanged by this documentation addition.

> **Classification:** Normative Engineering Specification & Master Codebase Catalog  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Team RACI Allocation:** See [Team Work Allocation](../../dev_docs/TEAM_WORK_ALLOCATION.md).  
> **Generated By:** Custos Nexus (Local AST & Tree-sitter Call Graph Indexer).

> **Workspace restructuring index:** [Master Part 1/15](../../Custos.md), [workspace/UI](../architecture/agent-workspace-and-ui.md), [skills/capabilities](../architecture/capability-catalog-and-skills.md) and [migration plan](workspace-restructuring-plan.md) define the current target. `ui/desktop` is the main product frontend; `ui/cli` is an existing React frontend, distinct from the Rust CLI at `crates/custos-app/cli`. Hosts remain under `crates/custos-app`; no ADE/UI crate or `custos-gui` was created. Domain features stay in packs; shared orchestration in runtime; I/O in adapters/persistence. This docs-first change adds documentation only: source paths and historical catalog counts are not migration results. Current uncommitted Repo Intelligence changes are preserved and require their own catalog/build audit.

> **SADE–OrCa migration index:** [Implementation plan](sade-orca-integration-plan.md) and Host Thinning refactor completed Waves H0-H5: [`daemon/src/profile.rs`](../../crates/custos-daemon/src/profile.rs) provides canonical profile directory resolution (`~/.custos/profiles/<profile_id>`) and singleton process file lock (`daemon.lock`); [`daemon/src/main.rs`](../../crates/custos-daemon/src/main.rs) acts as the sole backend owner hosting loopback TCP (`daemon.port`) and stdio JSONL; both [`desktop/src/lib.rs`](../../crates/custos-app/desktop/src/lib.rs) and [`cli/src/lib.rs`](../../crates/custos-app/cli/src/lib.rs) are now thin clients holding `Arc<LocalApiClient>`. The [desktop UI audit](../architecture/agent-workspace-and-ui.md#11-kiểm-tra-desktop-hiện-hành-và-quyết-định-hoàn-thiện) separates this completed host boundary from incomplete frontend Task/Session/Run projections and demo/live truth.

> **Runtime audit note:** File presence and catalog inclusion do not prove compilation or daemon composition. At commit `3e4dac4`, `crates/custos-runtime/src/engine/` contains Goose-derived source but is not mounted from `runtime/src/lib.rs`; `runtime/src/agent/` is compiled but its state machine is not composed by `custos-daemon`. `ModelProvider` and the Goose-derived `Provider` coexist; Codex/Claude/Antigravity model-named adapters are stubs. An adapter's assurance must be established by call-path and effect-interception tests, not this index.

> **Protocol-boundary index:** [Part 7](../../Custos.md#phần-7-protocol-và-hub-layer) owns the decisions; [protocol and connectivity boundaries](../architecture/protocol-and-connectivity-hubs.md) owns the implementation plan. The current [daemon main](../../crates/custos-daemon/src/main.rs) serves stdio JSONL, not socket/pipe/HTTP. [Bridge](../../crates/custos-bridge/src/lib.rs) imports SQLite persistence only in tests via a dev-dependency, not the production bridge path. The [MCP client](../../crates/custos-adapters/src/mcp/adapters/client.rs) fabricates a mock success; [roaming A2A](../../crates/custos-adapters/src/roaming/a2a.rs) simulates dispatch. ACP, CAP, real A2A and extra Local API listeners are target/optional work, not active protocols. No new physical files were created for these target bindings by the protocol decision.

> **OI decision index:** [Part 14](../../Custos.md#phần-14-orchestration-intelligence-oi-s1-s2-meta) and [cognitive/orchestration design](../architecture/cognitive-fabric-and-orchestration.md) define a **single-worker-first target**. Existing [`cognitive/routing.rs`](../../crates/custos-runtime/src/cognitive/routing.rs), [`cognitive/arbiter.rs`](../../crates/custos-runtime/src/cognitive/arbiter.rs), [`workflow/machine.rs`](../../crates/custos-runtime/src/workflow/machine.rs) and [`workflow/scheduler.rs`](../../crates/custos-runtime/src/workflow/scheduler.rs) are implementation inventory, not proof of a calibrated OI/typed plan compiler/replan loop. Pack templates are under [`custos-packs`](../../crates/custos-packs/); policy/effect authority remains in `custos-core`, and concrete wiring belongs to `custos-daemon`. No OI code file/crate was added by this documentation decision; preserve existing source catalog counts until code changes.

Tài liệu này là bản đồ toàn cảnh chi tiết đến **từng file mã nguồn** của toàn bộ repository Custos, bao gồm cấu trúc 11 canonical product crates, vai trò kiến trúc của từng file, các struct/trait/enum cốt lõi và các mối liên kết phụ thuộc.

---

## 0. Quy Trình Đồng Bộ Kiến Trúc & Vận Hành Dành Cho AI Coding Agents

> **Mục tiêu:** Đảm bảo tất cả AI Coding Agents (Cursor, Claude Code, Antigravity, Copilot, Windsurf) khi làm việc trên repository đều có **chung một mô hình tư duy kiến trúc (Shared Mental Model)**, không tự ý suy đoán, không tạo file rác và tuân thủ tuyệt đối quy trình đồng bộ tài liệu.

### 0.1 Tam Giác Đồng Bộ Tài Liệu (The Documentation Triad)

Mọi thay đổi kiến trúc trong repository bắt buộc phải được phản ánh đồng bộ trên 3 trụ cột:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   TAM GIÁC ĐỒNG BỘ KIẾN TRÚC & TÀI LIỆU                │
├────────────────────────────────────────────────────────────────────────┤
│ 1. Custos.md (Root Master Specification — Single Source of Truth)       │
│    - Bản chất: Nguồn chân lý tối cao xác lập "TẠI SAO & CÁI GÌ"        │
│      (10 Invariants, State Machines, Trust Zones, Threat Defense,      │
│      Hub Protocols, Clean Architecture Boundaries).                    │
│    - Nhiệm vụ Agent: Phải cập nhật đúng Phần/Chương tương ứng khi có   │
│      bất kỳ thay đổi nào về mặt thiết kế hoặc hợp đồng hệ thống.       │
├────────────────────────────────────────────────────────────────────────┤
│ 2. Thư mục docs/ (Authoritative Topic-Based Specifications)            │
│    - Bản chất: Tài liệu chuyên sâu "HIỆN THỰC THẾ NÀO" theo chủ đề     │
│      (docs/architecture/, docs/reference/, docs/development/).          │
│    - Nhiệm vụ Agent: Phải cập nhật đúng file chuyên đề, duy trì tính   │
│      thường xanh (evergreen), không chứa số phiên bản/ngày tháng rác.  │
├────────────────────────────────────────────────────────────────────────┤
│ 3. docs/development/codebase-architecture.md (Master Physical Catalog) │
│    - Bản chất: Bản đồ vật lý "Ở ĐÂU & GỒM NHỮNG GÌ", quản lý chi tiết  │
│      đến từng file, số dòng, vai trò kiến trúc và struct/trait/hàm.    │
│    - Nhiệm vụ Agent: Chèn đúng chỗ vào bảng danh mục của Crate, không  │
│      được để bất kỳ file mã nguồn nào tồn tại vô danh trong repo.      │
└────────────────────────────────────────────────────────────────────────┘
```

### 0.2 Quy Trình 5 Bước Bắt Buộc Khi Có Quyết Định Kiến Trúc Mới

Khi Người dùng và Agent đạt được sự thống nhất về một thay đổi kiến trúc hoặc cấu trúc file:

1. **Bước 1 — Xác lập Đồng thuận (Consensus):** Thống nhất rõ ràng về layer, crate chịu trách nhiệm, chủ sở hữu (Vĩ, Trường, hay Vinh), và các ràng buộc bất biến (ví dụ: Zero-I/O cho `custos-domain`).
2. **Bước 2 — Cập nhật `Custos.md`:** Cập nhật nội dung vào đúng chương mục trong [`Custos.md`](../../Custos.md).
3. **Bước 3 — Cập nhật `docs/` đúng chủ đề:** Cập nhật tài liệu chuyên môn tương ứng trong [`docs/architecture/`](../architecture/), [`docs/reference/`](../reference/), hoặc [`docs/development/`](../development/).
4. **Bước 4 — Chèn đúng chỗ vào `codebase-architecture.md` (Tài liệu này):**
   - Xác định đúng Crate tại Mục 3 bên dưới.
   - Thêm dòng mới vào bảng danh mục tệp tin theo đúng chuẩn 4 cột:
     `| Cột 1: Link file mã nguồn | Cột 2: Số dòng | Cột 3: Vai trò kiến trúc | Cột 4: Các Struct / Trait / Hàm cốt lõi |`
     (ví dụ cụ thể: `| [`src/action.rs`](../../crates/custos-domain/src/action.rs) | 163 | Vai trò... | Các Struct... |`)
   - Cập nhật số lượng file và tổng số dòng mã trên đầu mục của crate.
5. **Bước 5 — Triển khai Code & Xác thực:** Viết code, kiểm tra `cargo check --workspace` và `cargo test`.
   - **LỆNH CẤM NGHIÊM NGẶT:** Cấm tuyệt đối việc nhảy cóc vào Bước 5 để viết code thay đổi kiến trúc trước khi hoàn thành Bước 1, 2, 3, 4!

### 0.3 Hướng Dẫn Đọc Dành Cho Coding Agents Trước Khi Viết Code

- **Định vị trước khi sửa:** Trước khi thêm struct, trait, hoặc chỉnh sửa logic, Agent PHẢI tra cứu Mục 2 (Ma trận Crate) và Mục 3 (Danh mục chi tiết từng crate) của tài liệu này để biết code nên đặt ở đâu.
- **Tuân thủ ranh giới bất biến:**
  - `custos-domain`: Tuyệt đối Zero-I/O, không async, không tokio, không rusqlite, không reqwest.
  - `custos-core`: Hạt nhân điều phối, chỉ phụ thuộc vào `custos-domain`.
  - `custos-persistence`: SQLite WAL mode, single-writer connection, chống WAL starvation.
  - `custos-bridge`: Cầu nối phiên, CẤM gọi trực tiếp vào `custos-persistence`.
  - `custos-daemon`: Điểm ráp nối duy nhất (Sole Composition Root).
- **Không tạo thư mục rác:** Mọi mã nguồn chỉ được phép nằm trong 11 canonical product crates, `schemas/`, `tests/`, `tools/`, `ui/`, hoặc `xtask/`. Tuyệt đối không tạo các thư mục rác như `scratch/`, `templates/`, `services/`.

---

## 1. Bản Đồ Tổng Thể Workspace Monorepo

```text
Custos/
├── Cargo.toml                       # Root Cargo workspace manifest (quản lý 11 product crates + tests + xtask)
├── rust-toolchain.toml              # Phiên bản Rust toolchain cố định
├── deny.toml                        # Cấu hình cargo-deny (bảo mật chuỗi cung ứng, license audit)
│
├── crates/                          # [TRỤC 11 CANONICAL PRODUCT CRATES]
│   ├── custos-domain/               # Layer 0: Thực thể miền thuần túy (Zero I/O, no async)
│   ├── custos-core/                 # Layer 1: Task Kernel, Authority Engine, Completion Gate
│   ├── custos-persistence/          # Layer 1: SQLite WAL persistence, schema migrations, Outbox, CAS
│   ├── custos-provider/             # Layer 1: Trait ProviderPort, token streaming, wire transformers
│   ├── custos-bridge/               # Layer 2: Session-to-Task promotion, Local API routing
│   ├── custos-runtime/              # Layer 2: Session lifecycle, Agent loop, Cognition S1/S2, Context Compiler
│   ├── custos-adapters/             # Layer 2: OS Sandbox (Seatbelt/Bubblewrap), MCP, Local Inference, Harnesses
│   ├── custos-packs/                # Layer 3: Engineering Pack, Research Pack, Assistant Pack workflows
│   ├── custos-daemon/               # Layer 4: Điểm ráp nối duy nhất (Sole Composition Root), background service
│   ├── custos-sdk/                  # Layer 4: Client bindings (Rust, Python, Kotlin/Java) & versioned DTOs
│   └── custos-app/
│       ├── cli/                     # Layer 4: Terminal TUI operator interface (ratatui, clap)
│       └── desktop/                 # Layer 4: Desktop GUI app interface (Tauri, Webview)
│
├── schemas/                         # Versioned cross-process JSON Schemas
├── tests/                           # Workspace test suites (contract, e2e, crash, test-support)
├── evals/                           # Quality benchmarks, token cost tracking, calibration
├── tools/repo_intelligent/          # Custos Nexus: AST Tree-sitter & call-graph intelligence engine
├── ui/                              # Web and desktop user interface components
└── xtask/                           # Automation scripts & workspace management tasks
```

---

## 2. Ma Trận 11 Canonical Product Crates

| Layer | Product Crate | Người sở hữu (RACI) | Trách nhiệm kiến trúc cốt lõi | Quy tắc phụ thuộc |
|---|---|:---:|---|---|
| **Layer 0** | `crates/custos-domain` | **Vĩ** | Thực thể miền thuần túy (Task, Session, Evidence, Permit). | **Zero-I/O Tuyệt đối**: Không tokio, rusqlite, reqwest, fs. |
| **Layer 1** | `crates/custos-core` | **Vĩ** | Task Kernel, Authority Engine, Completion Gate, Invariants. | Phụ thuộc độc quyền vào `custos-domain`. |
| | `crates/custos-persistence` | **Trường** | SQLite WAL mode, schema migrations, Outbox, CAS storage. | Hiện thực hóa repository traits của Domain; single writer. |
| | `crates/custos-provider` | **Vĩ** | ProviderPort, token streaming, wire format transformers. | Trừu tượng hóa mô hình; không phụ thuộc vendor SDK cụ thể. |
| **Layer 2** | `crates/custos-bridge` | **Vinh** | Thăng cấp Session-to-Task, định tuyến Local API commands. | Cầu nối giữa Client/Protocol và Task Kernel nội bộ. |
| | `crates/custos-runtime` | **Vinh + Vĩ** | Agent loop, Cognition S1/S2, Context Compiler 8 bước, Workflow. | Phụ thuộc `custos-core`, `custos-domain`, `custos-provider`. |
| | `crates/custos-adapters` | **Trường + Vinh** | OS Sandboxes (Seatbelt/Bubblewrap), MCP client, Harnesses. | Đóng gói tương tác ngoại vi; mọi đầu vào là untrusted. |
| **Layer 3** | `crates/custos-packs` | **Vĩ** | Engineering Pack, Research Pack, Assistant Pack workflows. | Phụ thuộc `custos-runtime` và `custos-core`. |
| **Layer 4** | `crates/custos-daemon` | **Vĩ** | **Sole Composition Root**: Ráp nối storage, runtime, adapters. | Crate duy nhất được import concrete implementations. |
| | `crates/custos-sdk` | **Vinh** | Versioned Client DTOs, UniFFI bindings (Python, Kotlin). | Thư viện client nhẹ; không mang runtime daemon hay database. |
| | `crates/custos-app/cli` | **Vinh** | Terminal TUI operator interface (`ratatui`, `clap`). | Giao tiếp với Daemon độc quyền qua IPC socket / SDK. |
| | `crates/custos-app/desktop` | **Vinh** | Desktop GUI app interface (Tauri, Webview). | Giao tiếp với Daemon độc quyền qua IPC socket / SDK. |

---

## 3. Bản Đồ Chi Tiết Từng Crate & Từng File Mã Nguồn (Deep File Catalog)

> Dữ liệu trích xuất trực tiếp bằng Custos Nexus thông qua phân tích cú pháp AST Tree-sitter và bảng chỉ mục SQLite (`nexus_index.db`).

### 3.1. Crate `custos-domain` — Layer 0: Pure Domain Core

- **Đường dẫn thư mục:** `crates/custos-domain`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** Bất biến Zero-I/O tuyệt đối. Không tokio, rusqlite, reqwest, std::fs.
- **Tổng số file Rust:** 37 files | **Tổng số dòng Rust:** 5,105 lines
- **Mô tả chức năng:** Trái tim của hệ thống: Chứa toàn bộ thực thể thuần khiết, định danh (TaskId, SessionId), các bất biến miền, máy trạng thái tác vụ thuần túy và các khế ước sự kiện (Event Sourcing).

#### Danh mục các file bên trong `crates/custos-domain/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-domain/Cargo.toml) | 16 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/action.rs`](../../crates/custos-domain/src/action.rs) | 369 | Action intent/effect lifecycle, assurance và bất biến không retry mù khi uncertain | `enum RiskLevel`, `enum ActionLifecycleState`, `enum Assurance`, `struct Action`, `struct EffectAttempt` |
| [`src/approval.rs`](../../crates/custos-domain/src/approval.rs) | 167 | Approval request/decision có identity, expiry và single-resolution validation | `enum ApprovalStatus`, `struct ApprovalRequest`, `struct ApprovalDecision`, `fn apply_to` |
| [`src/artifact.rs`](../../crates/custos-domain/src/artifact.rs) | 211 | Artifact reference và selected cross-pack handoff không mang authority | `enum ArtifactKind`, `struct ArtifactRef`, `enum HandoffConsent`, `struct ArtifactHandoff` |
| [`src/authority.rs`](../../crates/custos-domain/src/authority.rs) | 154 | A Grant endows an agent/session with authority to perform certain capabilities. | `enum RiskClass`, `fn fmt`, `struct Grant`, `fn new` |
| [`src/budget.rs`](../../crates/custos-domain/src/budget.rs) | 203 | Hạch toán reserve/settle/refund span và token với checked arithmetic | `struct Budget`, `struct ReservationToken`, `fn reserve`, `fn settle`, `fn refund` |
| [`src/capability.rs`](../../crates/custos-domain/src/capability.rs) | 14 | Khai báo CapabilityManifest và năng lực thực thi của công cụ | `struct CapabilityManifest` |
| [`src/claim.rs`](../../crates/custos-domain/src/claim.rs) | 291 | Research source/passage/claim/experiment values; tách client proposal khỏi trusted hash/verification status | `struct SourceProposal`, `struct PassageAnchorProposal`, `struct ResearchClaimProposal`, `struct ResearchClaim`, `struct ResearchExperimentRun` |
| [`src/context.rs`](../../crates/custos-domain/src/context.rs) | 205 | ContextPack và receipt ghi included/omitted refs, redaction, token estimate và delivery state | `struct ContextItem`, `struct ContextPack`, `struct ContextReceipt`, `enum OmittedContextReason` |
| [`src/continuation.rs`](../../crates/custos-domain/src/continuation.rs) | 290 | Integrity packet và linked-continuation manifest/state không mang permit hoặc secret | `struct ContinuationPacket`, `struct ContinuationManifest`, `enum ContinuationState`, `fn verify` |
| [`src/error.rs`](../../crates/custos-domain/src/error.rs) | 33 | Phân loại lỗi hệ thống DomainError thuần khiết | `enum DomainError` |
| [`src/evidence.rs`](../../crates/custos-domain/src/evidence.rs) | 351 | Evidence, per-criterion verifier record và Task outcome projection với unknown/stale first-class | `enum EvidenceStatus`, `struct EvidenceRecord`, `struct CriterionVerificationRecord`, `struct TaskOutcome` |
| [`src/fact.rs`](../../crates/custos-domain/src/fact.rs) | 13 | Thực thể sự thật Fact trích xuất từ môi trường | `struct Fact` |
| [`src/ids.rs`](../../crates/custos-domain/src/ids.rs) | 46 | Generates a prefixed unique ID: `{prefix}_{uuidv4_simple}` | `fn new_id`, `type TaskId`, `fn digest`, `fn canonical_json` |
| [`src/lib.rs`](../../crates/custos-domain/src/lib.rs) | 98 | Explicit module declarations và re-export bề mặt domain canonical | Domain public exports |
| [`src/packet.rs`](../../crates/custos-domain/src/packet.rs) | 3 | Module packet: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/run.rs`](../../crates/custos-domain/src/run.rs) | 644 | Run/WorkerRun/dispatch lifecycle, launch truth và per-attempt usage không đổi unknown thành zero | `struct Run`, `struct WorkerRun`, `struct LaunchAttempt`, `struct UsageRecord`, `enum UsageExecutorKind`, `enum UsageMeasurement` |
| [`src/session.rs`](../../crates/custos-domain/src/session.rs) | 260 | Session compatibility model, canonical typed turn và session–Task attribution | `struct Session`, `struct ConversationTurn`, `struct SessionTaskBinding`, `enum WorkbenchLens` |
| [`src/span.rs`](../../crates/custos-domain/src/span.rs) | 90 | Quản lý phân đoạn thực thi Span, SpanState | `enum SpanState`, `fn fmt`, `struct Span`, `fn new` |
| [`src/task.rs`](../../crates/custos-domain/src/task.rs) | 309 | Task/contract/revision lifecycle với checked epoch và state-version arithmetic | `enum EvidenceKind`, `struct TaskContract`, `struct TaskRevision`, `struct Task`, `fn create_revision` |
| [`src/decision.rs`](../../crates/custos-domain/src/decision.rs) | 35 | Re-export tương thích ngược cho mô hình quyết định OI | `type DecisionRecord`, `type StrategyProposal`, `type ExecutionTopology` |
| [`src/oi/candidate.rs`](../../crates/custos-domain/src/oi/candidate.rs) | 48 | Cấu trúc Candidate và phân loại lý do từ chối RejectionReason | `struct Candidate`, `enum RejectionReason` |
| [`src/oi/mod.rs`](../../crates/custos-domain/src/oi/mod.rs) | 20 | Module khai báo và re-export toàn bộ từ vựng miền OI | None |
| [`src/oi/placement.rs`](../../crates/custos-domain/src/oi/placement.rs) | 30 | Ánh xạ vị trí NodePlacement và ngân sách cắt lát | `struct NodePlacement`, `fn new` |
| [`src/oi/proposal.rs`](../../crates/custos-domain/src/oi/proposal.rs) | 39 | Bản đề xuất chiến lược StrategyProposal của OI | `struct StrategyProposal`, `fn native_baseline` |
| [`src/oi/record.rs`](../../crates/custos-domain/src/oi/record.rs) | 30 | Bản ghi quyết định DecisionRecord lưu vào sổ cái | `struct DecisionRecord`, `fn new` |
| [`src/oi/replan.rs`](../../crates/custos-domain/src/oi/replan.rs) | 65 | Bản tóm tắt tái hoạch định ReplanBrief và ReplanTrigger | `struct ReplanBrief`, `enum ReplanTrigger`, `struct ReplanRecord` |
| [`src/oi/snapshot.rs`](../../crates/custos-domain/src/oi/snapshot.rs) | 48 | Bức tranh thực tại DecisionSnapshot v2 kèm pin và egress rules | `struct DecisionSnapshot`, `fn new` |
| [`src/oi/topology.rs`](../../crates/custos-domain/src/oi/topology.rs) | 68 | Phân loại chiến thuật ExecutionTopology T0–T8 | `enum ExecutionTopology`, `fn is_parallel`, `fn requires_isolation` |
| [`src/oi/work_packet.rs`](../../crates/custos-domain/src/oi/work_packet.rs) | 62 | Gói công việc WorkPacket và kết quả WorkerResult | `struct WorkPacket`, `struct WorkerResult`, `enum WorkerStatus` |
| [`src/types.rs`](../../crates/custos-domain/src/types.rs) | 10 | Định nghĩa các kiểu dữ liệu dùng chung | None |
| [`src/workflow.rs`](../../crates/custos-domain/src/workflow.rs) | 175 | Intermediate Representation (IR) và WorkflowRevision (RFC 004) | `struct WorkflowStep`, `struct WorkflowRevision`, `struct RevisionNode` |

### 3.2. Crate `custos-core` — Layer 1: Kernel & Invariants

- **Đường dẫn thư mục:** `crates/custos-core`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** Logic nghiệp vụ thuần túy; phụ thuộc độc quyền vào custos-domain.
- **Tổng số file:** 32 files | **Tổng số dòng mã:** 3,349 lines
- **Mô tả chức năng:** Bộ hạt nhân điều phối: Quản lý TaskStateMachine, AuthorityEngine, CompletionGate thẩm định bằng chứng, và cơ chế sandbox policy logic.

#### Danh mục các file bên trong `crates/custos-core/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-core/Cargo.toml) | 25 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/authority/approvals.rs`](../../crates/custos-core/src/authority/approvals.rs) | 51 | Quản lý yêu cầu phê duyệt người dùng (ApprovalManager) | `struct ApprovalManager`, `fn new`, `fn request_approval`, `fn get` |
| [`src/authority/audit.rs`](../../crates/custos-core/src/authority/audit.rs) | 82 | Ghi nhật ký kiểm toán hành vi an toàn (AuditLog, AuditEntry) | `struct AuditEntry`, `struct AuditLog`, `fn new`, `fn record` |
| [`src/authority/grants.rs`](../../crates/custos-core/src/authority/grants.rs) | 37 | Kho lưu trữ quyền được cấp phát (GrantStore) | `struct GrantStore`, `fn new`, `fn insert`, `fn get` |
| [`src/authority/mod.rs`](../../crates/custos-core/src/authority/mod.rs) | 149 | AuthorityEngine acts as the central authority enforcement point for tasks | `struct AuthorityEngine`, `fn default`, `fn new` |
| [`src/authority/permits.rs`](../../crates/custos-core/src/authority/permits.rs) | 104 | Phát hành, ràng buộc tham số và tiêu thụ một lần vé thực thi (PermitIssuer) | `struct PermitIssuer`, `fn issue_permit_with_digest`, `fn consume_permit` |
| [`src/authority/policy.rs`](../../crates/custos-core/src/authority/policy.rs) | 50 | Default policy evaluator based on action risk level | `enum PolicyDecision`, `trait PolicyEvaluator`, `struct DefaultPolicyEvaluator` |
| [`src/authority/risk.rs`](../../crates/custos-core/src/authority/risk.rs) | 43 | Classifies an action into a RiskClass | `struct RiskEvaluator`, `fn classify_action`, `fn requires_human_approval` |
| [`src/capability/deterministic.rs`](../../crates/custos-core/src/capability/deterministic.rs) | 632 | Prototype read/list/patch-preview dispatcher; rejects unsupported capabilities rather than fabricating success. Permit burn is process-local, not crash-durable. | `struct DeterministicGate`, `fn dispatch_for_task` |
| [`src/capability/mod.rs`](../../crates/custos-core/src/capability/mod.rs) | 10 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/capability/traits.rs`](../../crates/custos-core/src/capability/traits.rs) | 24 | Module traits: phục vụ các cấu trúc và chức năng liên quan | `struct ExecutionResult`, `trait ToolGate` |
| [`src/context/anchor.rs`](../../crates/custos-core/src/context/anchor.rs) | 60 | Trích xuất các lát cắt ngữ cảnh (window excerpts) quanh anchor từ khóa (§6.4 Bước 3) | `struct AnchorRetriever`, `fn retrieve_anchors` |
| [`src/context/compaction.rs`](../../crates/custos-core/src/context/compaction.rs) | 164 | Nén lũy tiến ngữ cảnh theo Token Budget (§6.4 Bước 6) | `struct ProgressiveCompactor`, `fn compact`, `fn estimate_tokens` |
| [`src/context/dedup.rs`](../../crates/custos-core/src/context/dedup.rs) | 60 | Khử trùng lặp nội dung dựa trên mã băm khối SHA-256 (§6.4 Bước 5) | `struct HashDeduplicator`, `fn deduplicate` |
| [`src/context/mod.rs`](../../crates/custos-core/src/context/mod.rs) | 214 | Điều phối đường ống biên dịch ngữ cảnh ContextCompiler 8 bước (§6.4) | `struct ContextCompiler`, `struct CompileContextRequest`, `fn compile` |
| [`src/context/redaction.rs`](../../crates/custos-core/src/context/redaction.rs) | 169 | Bôi đen secret API keys và các chuỗi Shannon entropy cao (§6.4 Bước 7) | `struct SecretRedactor`, `fn redact`, `fn is_high_entropy_token` |
| [`src/context/structural.rs`](../../crates/custos-core/src/context/structural.rs) | 130 | Lược trích cấu trúc mã nguồn (signatures, types, docstrings) (§6.4 Bước 2) | `struct StructuralExtractor`, `fn extract_outline` |
| [`src/evidence/bundle.rs`](../../crates/custos-core/src/evidence/bundle.rs) | 31 | Đóng gói tập hợp bằng chứng EvidenceBundle | `struct EvidenceBundle`, `fn new` |
| [`src/evidence/mod.rs`](../../crates/custos-core/src/evidence/mod.rs) | 263 | Module nghiệm thu bằng chứng | None |
| [`src/evidence/pipeline.rs`](../../crates/custos-core/src/evidence/pipeline.rs) | 111 | Registers a custom verifier. | `struct RequirementEvaluation`, `struct EvidencePipeline`, `fn default`, `fn new` |
| [`src/evidence/verifier.rs`](../../crates/custos-core/src/evidence/verifier.rs) | 411 | Verifies command execution success by checking exit code == 0. | `trait Verifier`, `struct CommandExitCodeVerifier`, `fn verifier_id`, `struct HashVerifier` |
| [`src/kernel/budget.rs`](../../crates/custos-core/src/kernel/budget.rs) | 130 | Quản lý ngân sách token 2 pha Reserve/Settle và Anti-Gaming Headroom (§16.3) | `struct BudgetGovernor`, `fn reserve`, `fn settle`, `fn refund`, `fn headroom` |
| [`src/kernel/commands.rs`](../../crates/custos-core/src/kernel/commands.rs) | 45 | Các lệnh tác động vào Kernel: Create, Advance, Block, Complete, Cancel | `struct CreateTask`, `struct AdvanceTask`, `struct BlockTask`, `struct CompleteTask` |
| [`src/kernel/completion.rs`](../../crates/custos-core/src/kernel/completion.rs) | 250 | Thẩm định cổng nghiệm thu bằng chứng REAL, cấm claim Stale và lệch source_version | `struct CompletionGate`, `fn can_complete_with_evidence` |
| [`src/kernel/events.rs`](../../crates/custos-core/src/kernel/events.rs) | 109 | One-time seed preserving a task snapshot created before event journaling. | `struct TaskCreated`, `fn empty_metadata`, `struct TaskAdvanced`, `struct TaskBlocked` |
| [`src/kernel/invariants.rs`](../../crates/custos-core/src/kernel/invariants.rs) | 56 | Asserts title is valid and non-empty | `struct TaskInvariants`, `fn assert_valid_title`, `fn assert_epoch`, `fn assert_not_terminal` |
| [`src/kernel/mod.rs`](../../crates/custos-core/src/kernel/mod.rs) | 26 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/kernel/observability.rs`](../../crates/custos-core/src/kernel/observability.rs) | 11 | Khởi tạo theo dõi và giám sát telemetry | `fn init_tracing` |
| [`src/kernel/ports.rs`](../../crates/custos-core/src/kernel/ports.rs) | 44 | Persistence Port for Interactive Sessions & Interaction Journals | `trait TaskStore`, `trait SessionStore` |
| [`src/kernel/reducer.rs`](../../crates/custos-core/src/kernel/reducer.rs) | 195 | Creates an initial Task from TaskCreated event | `struct TaskReducer`, `fn from_created`, `fn replay`, `fn apply` |
| [`src/kernel/service.rs`](../../crates/custos-core/src/kernel/service.rs) | 387 | CQRS command: CreateTask | `struct TaskService`, `fn new`, `struct MockTaskStore` |
| [`src/kernel/session_state_machine.rs`](../../crates/custos-core/src/kernel/session_state_machine.rs) | 40 | Máy trạng thái vòng đời phiên làm việc | `struct SessionStateMachine`, `fn transition` |
| [`src/kernel/span_service.rs`](../../crates/custos-core/src/kernel/span_service.rs) | 60 | Dịch vụ xử lý TaskKernel chính (TaskService) | `struct SpanService`, `fn new` |
| [`src/kernel/state_machine.rs`](../../crates/custos-core/src/kernel/state_machine.rs) | 41 | Returns all valid next states for a given state | `struct TaskStateMachine`, `fn valid_next_states`, `fn can_transition`, `fn ensure_can_transition` |
| [`src/lib.rs`](../../crates/custos-core/src/lib.rs) | 17 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/contracts/judgment.rs`](../../crates/custos-core/src/contracts/judgment.rs) | 13 | Khế ước thẩm định phán quyết và độ tin cậy của bằng chứng (S1 Fabric) | `trait JudgmentPort` |
| [`src/contracts/oi.rs`](../../crates/custos-core/src/contracts/oi.rs) | 15 | Khế ước giao tiếp với bộ não lập kế hoạch Orchestration Intelligence | `trait OiPlannerPort` |
| [`src/decision.rs`](../../crates/custos-core/src/decision.rs) | 90 | Trích xuất Snapshot hiện thực chuẩn hoá (lọc theo task, fix G6) | `struct DecisionSnapshotExtractor`, `fn extract_snapshot` |
| [`src/oi/admissibility.rs`](../../crates/custos-core/src/oi/admissibility.rs) | 45 | Cổng thẩm định tính hợp lệ của đề xuất chiến lược | `enum AdmissibilityResult`, `struct AdmissibilityEvaluator` |
| [`src/oi/compiler.rs`](../../crates/custos-core/src/oi/compiler.rs) | 180 | Trình biên dịch đề xuất thành WorkflowRevision tất định T0–T8 | `struct PlanCompiler`, `fn compile` |
| [`src/oi/hard_filters.rs`](../../crates/custos-core/src/oi/hard_filters.rs) | 80 | Bộ lọc tĩnh tất định (pin, ngân sách, giới hạn mạng) | `struct HardFilters`, `fn evaluate` |
| [`src/oi/mod.rs`](../../crates/custos-core/src/oi/mod.rs) | 12 | Module quản trị các cổng OI Core | None |
| [`src/sandbox_policy/bubblewrap.rs`](../../crates/custos-core/src/sandbox_policy/bubblewrap.rs) | 13 | Cấu hình chính sách sandbox Bubblewrap cho Linux | `struct BubblewrapSandbox`, `fn new`, `fn default` |
| [`src/sandbox_policy/mod.rs`](../../crates/custos-core/src/sandbox_policy/mod.rs) | 7 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/sandbox_policy/path_sandbox.rs`](../../crates/custos-core/src/sandbox_policy/path_sandbox.rs) | 130 | Resolves and validates that the requested path strictly resides within the sandbox workspace. | `struct PathSandbox`, `fn new`, `fn workspace_root`, `fn resolve_and_contain` |
| [`src/sandbox_policy/seatbelt.rs`](../../crates/custos-core/src/sandbox_policy/seatbelt.rs) | 13 | Cấu hình chính sách sandbox Apple Seatbelt cho macOS | `struct SeatbeltSandbox`, `fn new`, `fn default` |

### 3.3. Crate `custos-persistence` — Layer 1: Relational Storage

- **Đường dẫn thư mục:** `crates/custos-persistence`
- **Chủ sở hữu chính (Owner):** **Trường (Systems & Persistence Lead)**
- **Quy tắc ranh giới:** Triệt tiêu lỗi đói WAL (P0), single-writer connection, runtime version check.
- **Tổng số file:** 21 files | **Tổng số dòng mã:** 1,597 lines
- **Mô tả chức năng:** Hạ tầng lưu trữ bền vững SQLite WAL mode, schema migrations, Event Store, Transactional Outbox, và CAS Content-Addressable Storage.

#### Danh mục các file bên trong `crates/custos-persistence/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-persistence/Cargo.toml) | 16 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0001_core.sql`](../../crates/custos-persistence/migrations/0001_core.sql) | 37 | Module 0001_core: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0002_runtime.sql`](../../crates/custos-persistence/migrations/0002_runtime.sql) | 18 | Module 0002_runtime: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0003_authority.sql`](../../crates/custos-persistence/migrations/0003_authority.sql) | 50 | Module 0003_authority: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0004_usage.sql`](../../crates/custos-persistence/migrations/0004_usage.sql) | 31 | Module 0004_usage: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0005_evidence.sql`](../../crates/custos-persistence/migrations/0005_evidence.sql) | 30 | Module 0005_evidence: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0006_session_and_bridge.sql`](../../crates/custos-persistence/migrations/0006_session_and_bridge.sql) | 47 | Module 0006_session_and_bridge: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0007_task_contract.sql`](../../crates/custos-persistence/migrations/0007_task_contract.sql) | 1 | Module 0007_task_contract: phục vụ các cấu trúc và chức năng liên quan | None |
| [`migrations/0008_task_events.sql`](../../crates/custos-persistence/migrations/0008_task_events.sql) | 10 | Module 0008_task_events: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/artifacts/filesystem.rs`](../../crates/custos-persistence/src/artifacts/filesystem.rs) | 53 | Kho lưu trữ tệp vật lý Content-Addressable (CAS) | `struct FsArtifactStore`, `fn new` |
| [`src/artifacts/lib.rs`](../../crates/custos-persistence/src/artifacts/lib.rs) | 5 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/artifacts/traits.rs`](../../crates/custos-persistence/src/artifacts/traits.rs) | 9 | Trait trừu tượng ArtifactStore | `trait ArtifactStore` |
| [`src/connection.rs`](../../crates/custos-persistence/src/connection.rs) | 57 | Managed thread-safe SQLite connection handle. | `struct DbConnection`, `fn open_in_memory`, `fn open`, `fn configure_and_migrate` |
| [`src/lib.rs`](../../crates/custos-persistence/src/lib.rs) | 12 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/migrations.rs`](../../crates/custos-persistence/src/migrations.rs) | 35 | Thực thi các script migration tự động | `fn run_migrations` |
| [`src/repositories/continuation.rs`](../../crates/custos-persistence/src/repositories/continuation.rs) | 124 | Gói tiếp tục ContinuationPacket kèm mã băm SHA-256 bảo đảm an toàn khi tiếp tục | `struct ContinuationRepository`, `fn new`, `fn save_continuation`, `fn get_latest_continuation` |
| [`src/repositories/mod.rs`](../../crates/custos-persistence/src/repositories/mod.rs) | 9 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/repositories/session.rs`](../../crates/custos-persistence/src/repositories/session.rs) | 201 | Thực thể Session, SessionId, SessionStatus, nhật ký SessionJournalEntry | `struct SessionRepository`, `fn new`, `fn get_session`, `fn list_sessions` |
| [`src/repositories/span.rs`](../../crates/custos-persistence/src/repositories/span.rs) | 134 | Quản lý phân đoạn thực thi Span, SpanState | `struct SpanRepository`, `fn new`, `fn get_span`, `fn save_span` |
| [`src/repositories/task.rs`](../../crates/custos-persistence/src/repositories/task.rs) | 312 | Mô hình nhiệm vụ cốt lõi: Task, TaskContract, TaskStatus, ContractEvidence | `struct TaskRepository`, `fn new`, `fn get_task`, `fn read_task` |
| [`src/store.rs`](../../crates/custos-persistence/src/store.rs) | 406 | SQLite-backed persistent storage implementing TaskStore and SessionStore. | `struct SqliteTaskStore`, `fn new_in_memory`, `fn new`, `fn db` |

### 3.4. Crate `custos-provider` — Layer 1: Model Contracts

- **Đường dẫn thư mục:** `crates/custos-provider`
- **Chủ sở hữu chính (Owner):** **Vĩ (AI/DS Lead)**
- **Quy tắc ranh giới:** Trừu tượng hóa nhà cung cấp LLM; không coupling cứng vào SDK vendor.
- **Tổng số file:** 47 files | **Tổng số dòng mã:** 34,387 lines
- **Mô tả chức năng:** Khế ước kết nối mô hình ngôn ngữ: Trait ProviderPort, xử lý token streaming, chuẩn hóa wire formats (OpenAI, Anthropic, Google, Ollama), tính toán token budget.

#### Danh mục các file bên trong `crates/custos-provider/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-provider/Cargo.toml) | 41 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`build.rs`](../../crates/custos-provider/build.rs) | 14 | Module build: phục vụ các cấu trúc và chức năng liên quan | `fn main` |
| [`src/conformance.rs`](../../crates/custos-provider/src/conformance.rs) | 50 | Asserts that a provider implementation adheres to the required ModelProvider contracts. | None |
| [`src/events.rs`](../../crates/custos-provider/src/events.rs) | 93 | Các sự kiện phát sinh từ Model: ToolCall, TokenUsage, Chunk | `enum ProviderEventType`, `struct ToolCall`, `struct TokenUsage`, `struct ProviderEvent` |
| [`src/lib.rs`](../../crates/custos-provider/src/lib.rs) | 14 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/port.rs`](../../crates/custos-provider/src/port.rs) | 76 | Streaming generation returning an asynchronous receiver of ProviderEvents. | `trait ModelProvider`, `trait AgentRuntimePort`, `trait CapabilityPort` |
| [`src/request.rs`](../../crates/custos-provider/src/request.rs) | 84 | Creates a simple request with reasonable defaults. | `struct ProviderRequest`, `fn simple`, `fn new`, `type ModelRequest` |
| [`src/tool.rs`](../../crates/custos-provider/src/tool.rs) | 25 | Định nghĩa ToolDefinition, ToolResult | `struct ToolResult`, `struct ToolDefinition`, `trait ToolPort` |
| [`src/traits.rs`](../../crates/custos-provider/src/traits.rs) | 5 | Module traits: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/types/base.rs`](../../crates/custos-provider/src/types/base.rs) | 1124 | Metadata about a provider's configuration requirements and capabilities | `struct ProviderMetadata`, `struct ProviderDeprecation`, `fn new`, `fn with_models` |
| [`src/types/cache_semantics.rs`](../../crates/custos-provider/src/types/cache_semantics.rs) | 342 | Unknown pairs default to `ImplicitStrict`, which is safe for every | `enum CacheSemantics`, `fn for_model`, `fn uses_explicit_breakpoints`, `fn apply_chat_payload_breakpoints` |
| [`src/types/canonical.rs`](../../crates/custos-provider/src/types/canonical.rs) | 365 | Return recommended model names for a provider using only the bundled canonical registry. | `struct ModelMapping`, `fn new`, `fn recommended_models_from_registry`, `fn google_generate_content_model` |
| [`src/types/canonical/README.md`](../../crates/custos-provider/src/types/canonical/README.md) | 23 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | None |
| [`src/types/canonical/canonical_models.json.zst`](../../crates/custos-provider/src/types/canonical/canonical_models.json.zst) | 3701 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | None |
| [`src/types/canonical/catalog.rs`](../../crates/custos-provider/src/types/canonical/catalog.rs) | 631 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | `struct ProviderMetadataEntry`, `enum ProviderFormat`, `fn as_str`, `type Err` |
| [`src/types/canonical/data/canonical_mapping_report.json`](../../crates/custos-provider/src/types/canonical/data/canonical_mapping_report.json) | 24 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | None |
| [`src/types/canonical/data/provider_metadata.json`](../../crates/custos-provider/src/types/canonical/data/provider_metadata.json) | 1 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | None |
| [`src/types/canonical/model.rs`](../../crates/custos-provider/src/types/canonical/model.rs) | 244 | Modality types for model input/output | `enum Modality`, `fn deserialize_modalities`, `struct Modalities`, `struct Pricing` |
| [`src/types/canonical/name_builder.rs`](../../crates/custos-provider/src/types/canonical/name_builder.rs) | 618 | Build canonical model name from provider and model identifiers | `fn canonical_name`, `fn is_meta_provider`, `fn map_provider_name`, `fn map_to_canonical_model` |
| [`src/types/canonical/registry.rs`](../../crates/custos-provider/src/types/canonical/registry.rs) | 110 | Ánh xạ định danh mô hình chuẩn hóa toàn cầu | `struct CanonicalModelRegistry`, `fn new`, `fn bundled`, `fn from_file` |
| [`src/types/context_limit.rs`](../../crates/custos-provider/src/types/context_limit.rs) | 195 | Module context_limit: phục vụ các cấu trúc và chức năng liên quan | `struct ContextLimitResolver`, `fn new`, `fn with_configured_limits`, `fn configured_limit` |
| [`src/types/conversation.rs`](../../crates/custos-provider/src/types/conversation.rs) | 2016 | Fix a conversation that we're about to send to an LLM. So the first and last | `struct Conversation`, `struct InvalidConversation`, `fn new`, `fn new_unvalidated` |
| [`src/types/conversation/message.rs`](../../crates/custos-provider/src/types/conversation/message.rs) | 2455 | Custom deserializer for MessageContent that sanitizes Unicode Tags in text content | `enum ToolCallResult`, `fn deserialize_sanitized_content`, `type ProviderMetadata`, `type ToolResult` |
| [`src/types/conversation/token_usage.rs`](../../crates/custos-provider/src/types/conversation/token_usage.rs) | 288 | `input_tokens` is the total input including cache read/write tokens; | `struct ProviderUsage`, `enum CostSource`, `struct ProviderStats`, `struct DraftStats` |
| [`src/types/conversation/tool_request.rs`](../../crates/custos-provider/src/types/conversation/tool_request.rs) | 288 | Returns true if this tool request was already executed externally | `fn from`, `fn tool_name_parts`, `fn to_readable_string`, `fn was_executed_externally` |
| [`src/types/conversation/tool_result_serde.rs`](../../crates/custos-provider/src/types/conversation/tool_result_serde.rs) | 194 | Quản lý lịch sử hội thoại, tin nhắn và gọi công cụ | `fn serialize`, `struct ToolCallWithValueArguments`, `fn into_call_tool_request_param`, `fn deserialize` |
| [`src/types/custos_mode.rs`](../../crates/custos-provider/src/types/custos_mode.rs) | 36 | Backward-compatible type name for clients migrating from the Goose-derived API. | `enum CustosMode`, `type GooseMode` |
| [`src/types/documents.rs`](../../crates/custos-provider/src/types/documents.rs) | 80 | Explains why a document was dropped so the model, and the caller reading the | `enum DocumentFormat`, `fn document_media_type_is_supported`, `fn convert_document`, `fn unsupported_document_text` |
| [`src/types/errors.rs`](../../crates/custos-provider/src/types/errors.rs) | 309 | Recover a typed `ProviderError` from a streaming decode error, falling | `enum ProviderError`, `fn stream_decode_error`, `fn telemetry_type`, `fn is_endpoint_not_found` |
| [`src/types/formats.rs`](../../crates/custos-provider/src/types/formats.rs) | 8 | Module formats: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/types/formats/anthropic.rs`](../../crates/custos-provider/src/types/formats/anthropic.rs) | 3305 | Anthropic-compatible providers keep `Default`, which does not request block binding. | `struct AnthropicFormatOptions`, `fn native`, `fn for_model`, `fn cache_control` |
| [`src/types/formats/databricks.rs`](../../crates/custos-provider/src/types/formats/databricks.rs) | 2062 | Convert Databricks' API response to internal Message format | `struct DatabricksMessage`, `fn format_text_content`, `fn format_tool_response`, `fn format_messages` |
| [`src/types/formats/google.rs`](../../crates/custos-provider/src/types/formats/google.rs) | 1888 | Convert internal Message format to Google's API message specification | `fn metadata_with_signature`, `fn get_thought_signature`, `fn is_user_loop_boundary`, `fn insert_thought_signature` |
| [`src/types/formats/ollama.rs`](../../crates/custos-provider/src/types/formats/ollama.rs) | 450 | Parse XML-style tool calls from content (Ollama/Qwen3-coder fallback format). | `fn parse_xml_tool_calls`, `fn response_to_message`, `fn extract_text_from_message`, `fn is_text_only_message` |
| [`src/types/formats/openai.rs`](../../crates/custos-provider/src/types/formats/openai.rs) | 5788 | Prefer `reasoning_content` (DeepSeek/OpenRouter) over `reasoning` | `type ToolCallData`, `fn deserialize_null_default_string`, `fn describe_json_value`, `fn output_token_limit_tool_error` |
| [`src/types/formats/openai_responses.rs`](../../crates/custos-provider/src/types/formats/openai_responses.rs) | 3119 | Parse a line per the SSE grammar and return its field name: | `struct ResponsesApiResponse`, `struct SummaryText`, `fn reasoning_from_summary`, `enum ResponseOutputItem` |
| [`src/types/formats/snowflake.rs`](../../crates/custos-provider/src/types/formats/snowflake.rs) | 732 | Convert internal Message format to Snowflake's API message specification | `fn format_messages`, `fn format_tools`, `fn format_system`, `fn parse_streaming_response` |
| [`src/types/images.rs`](../../crates/custos-provider/src/types/images.rs) | 552 | Convert an image content into an image json based on format | `enum ImageFormat`, `fn convert_image`, `fn detect_image_path`, `fn clean_path` |
| [`src/types/json.rs`](../../crates/custos-provider/src/types/json.rs) | 559 | Safely parse a JSON string that may contain doubly-encoded or malformed JSON. | `fn safely_parse_json`, `fn repair_truncated_json`, `fn json_escape_control_chars_in_string`, `fn looks_truncated` |
| [`src/types/mcp_utils.rs`](../../crates/custos-provider/src/types/mcp_utils.rs) | 109 | Module mcp_utils: phục vụ các cấu trúc và chức năng liên quan | `fn extract_text_from_resource` |
| [`src/types/mod.rs`](../../crates/custos-provider/src/types/mod.rs) | 24 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/types/model.rs`](../../crates/custos-provider/src/types/model.rs) | 1066 | Request params Custos/Goose consumes internally: formats that forward unknown params into | `fn is_custos_internal_request_param`, `struct ModelConfig`, `fn deserialize`, `fn new` |
| [`src/types/permission.rs`](../../crates/custos-provider/src/types/permission.rs) | 23 | Module permission: phục vụ các cấu trúc và chức năng liên quan | `enum Permission`, `enum PrincipalType`, `struct PermissionConfirmation` |
| [`src/types/request_log.rs`](../../crates/custos-provider/src/types/request_log.rs) | 134 | Module request_log: phục vụ các cấu trúc và chức năng liên quan | `type RequestLogError`, `struct LoggerAlreadyInstalled`, `fn fmt`, `fn install_logger` |
| [`src/types/retry.rs`](../../crates/custos-provider/src/types/retry.rs) | 351 | Trait for retry functionality to keep Provider dyn-compatible. | `struct RetryConfig`, `fn default`, `fn new`, `fn transient_only` |
| [`src/types/thinking.rs`](../../crates/custos-provider/src/types/thinking.rs) | 742 | A single selectable effort value advertised by a provider-managed harness. | `fn split_think_blocks`, `struct FilterOut`, `struct ThinkFilter`, `enum ThinkTag` |
| [`src/types/utils.rs`](../../crates/custos-provider/src/types/utils.rs) | 29 | Extract the model name from a JSON object. Common with most providers to have this top level attribute. | `fn is_in_unicode_tag_range`, `fn sanitize_unicode_tags`, `fn strip_unicode_tags`, `fn get_model` |

### 3.5. Crate `custos-bridge` — Layer 2: IPC & Protocol Bridge

- **Đường dẫn thư mục:** `crates/custos-bridge`
- **Chủ sở hữu chính (Owner):** **Vinh (Runtime & Client Lead)**
- **Quy tắc ranh giới:** Thăng cấp lũy thừa Session-to-Task, định tuyến Local API.
- **Tổng số file:** 4 files | **Tổng số dòng mã:** 473 lines
- **Mô tả chức năng:** Cầu nối giao tiếp giữa Client/UI và Kernel: chuyển đổi lệnh người dùng, quản lý chế độ đính kèm (attach/steer/recall/fork) và thăng cấp trạng thái.

#### Danh mục các file bên trong `crates/custos-bridge/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-bridge/Cargo.toml) | 24 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/lib.rs`](../../crates/custos-bridge/src/lib.rs) | 151 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/port.rs`](../../crates/custos-bridge/src/port.rs) | 91 | Trait cốt lõi ProviderPort, ModelProvider, AgentRuntimePort | `enum AttachMode`, `struct SteerReceipt`, `trait BridgePort`, `struct TaskObservation` |
| [`src/service.rs`](../../crates/custos-bridge/src/service.rs) | 207 | BridgeService: thăng cấp Session-to-Task và điều phối IPC | `struct BridgeService`, `fn new` |

### 3.6. Crate `custos-runtime` — Layer 2: Execution & Orchestration

- **Đường dẫn thư mục:** `crates/custos-runtime`
- **Chủ sở hữu chính (Owner):** **Vinh (Runtime & Client Lead) & Vĩ (Cognitive Lead)**
- **Quy tắc ranh giới:** Quản lý vòng lặp thực thi, nhận thức S1/S2, và context compiler.
- **Tổng số file:** 287 files | **Tổng số dòng mã:** 100,623 lines
- **Mô tả chức năng:** Động cơ vận hành tác vụ: Agent execution loop, code hiện có cho routing S1/S2, context và workflow. OI đích chọn direct/one-worker/bounded graph theo policy và outcome; hiện diện của `routing.rs`/`workflow/` **không** xác nhận compiler, semantic preflight, calibrated utility hoặc evidence-triggered replan đã hoàn tất.

#### Danh mục các file bên trong `crates/custos-runtime/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-runtime/Cargo.toml) | 42 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/agent/events.rs`](../../crates/custos-runtime/src/agent/events.rs) | 18 | Các sự kiện phát sinh từ Model: ToolCall, TokenUsage, Chunk | `enum AgentEvent` |
| [`src/agent/execution.rs`](../../crates/custos-runtime/src/agent/execution.rs) | 127 | Context passed through the tool execution pipeline. | `struct ToolCallNotificationEmitter`, `fn new`, `fn emit_best_effort`, `struct ToolCallContext` |
| [`src/agent/inference.rs`](../../crates/custos-runtime/src/agent/inference.rs) | 587 | The agent-visible conversation as the provider sees it: tool requests left | `struct PreparedInferenceRequest`, `trait InferenceRequestPreparer`, `struct IdentityInferenceRequestPreparer`, `trait InferenceEffect` |
| [`src/agent/machine.rs`](../../crates/custos-runtime/src/agent/machine.rs) | 207 | Module machine: phục vụ các cấu trúc và chức năng liên quan | `trait MachineSession`, `trait SessionLoader`, `trait EffectHandler`, `trait EffectUsage` |
| [`src/agent/mod.rs`](../../crates/custos-runtime/src/agent/mod.rs) | 12 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/agent/operation.rs`](../../crates/custos-runtime/src/agent/operation.rs) | 270 | Note on a message something this operation did, so that a pipeline rebuilt | `type OperationFuture`, `struct SlashCommand`, `fn messages_since_kickoff`, `fn trailing_error` |
| [`src/agent/reply.rs`](../../crates/custos-runtime/src/agent/reply.rs) | 170 | Coerce a string value based on the target JSON schema type definition | `fn coerce_value`, `fn try_coerce_number`, `fn try_coerce_boolean`, `fn coerce_object` |
| [`src/agent/runtime_port.rs`](../../crates/custos-runtime/src/agent/runtime_port.rs) | 186 | Evaluates tool name and arguments to classify deterministic risk level | `struct GovernedAgentRuntime`, `fn new`, `fn classify_tool_risk`, `fn extract_target` |
| [`src/agent/tool.rs`](../../crates/custos-runtime/src/agent/tool.rs) | 371 | Supplies tools whose definitions and implementations may vary by session. | `fn empty_input_schema`, `fn definition`, `fn pending_requests`, `fn interrupted_result` |
| [`src/cognitive/approval_router.rs`](../../crates/custos-runtime/src/cognitive/approval_router.rs) | 161 | Module approval_router: phục vụ các cấu trúc và chức năng liên quan | `enum ApprovalKind`, `enum ApprovalDecision`, `struct ApprovalPolicy`, `fn default` |
| [`src/cognitive/arbiter.rs`](../../crates/custos-runtime/src/cognitive/arbiter.rs) | 28 | Routes work to a low-cost or deliberate reasoning tier using explicit signals. | `struct CognitiveArbiter`, `fn new`, `fn with_policy`, `fn route` |
| [`src/cognitive/config.rs`](../../crates/custos-runtime/src/cognitive/config.rs) | 59 | Module config: phục vụ các cấu trúc và chức năng liên quan | `struct CognitiveConfig`, `fn default`, `fn from_json`, `fn to_json` |
| [`src/cognitive/deliberation.rs`](../../crates/custos-runtime/src/cognitive/deliberation.rs) | 235 | Execute deliberation pipeline across configured roles | `enum WorkerRole`, `fn fmt`, `struct DeliberationPlan`, `fn new` |
| [`src/cognitive/human_gate.rs`](../../crates/custos-runtime/src/cognitive/human_gate.rs) | 213 | Creates an elicitation or approval request and awaits human response or timeout. | `enum HumanGateOutcome`, `struct HumanGateRequest`, `struct PendingRequestInternal`, `struct PendingClaim` |
| [`src/cognitive/mod.rs`](../../crates/custos-runtime/src/cognitive/mod.rs) | 30 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/cognitive/pipeline.rs`](../../crates/custos-runtime/src/cognitive/pipeline.rs) | 378 | Process an incoming user request through the full 9Router cognitive pipeline | `trait A2ADelegationPort`, `enum CognitiveExecutionResult`, `struct CognitivePipeline`, `fn new` |
| [`src/cognitive/prompt_manager.rs`](../../crates/custos-runtime/src/cognitive/prompt_manager.rs) | 315 | Module prompt_manager: phục vụ các cấu trúc và chức năng liên quan | `enum CustosAgentMode`, `struct ExtensionPromptInfo`, `fn new`, `struct PromptManager` |
| [`src/cognitive/provider_selector.rs`](../../crates/custos-runtime/src/cognitive/provider_selector.rs) | 182 | Selects the highest-priority available provider for a given tier. | `struct ProviderPreference`, `struct TierProviderMap`, `fn default`, `struct ProviderSelector` |
| [`src/cognitive/rdc.rs`](../../crates/custos-runtime/src/cognitive/rdc.rs) | 6 | Module rdc: phục vụ các cấu trúc và chức năng liên quan | `enum RdcPhase` |
| [`src/cognitive/registry.rs`](../../crates/custos-runtime/src/cognitive/registry.rs) | 266 | Register a provider implementation | `enum HealthStatus`, `struct ProviderHealth`, `fn default`, `struct ProviderRegistry` |
| [`src/cognitive/routing.rs`](../../crates/custos-runtime/src/cognitive/routing.rs) | 383 | Module routing: phục vụ các cấu trúc và chức năng liên quan | `enum ReasoningTier`, `struct RoutingSignals`, `enum ContextMode`, `struct A2ATarget` |
| [`src/cognitive/signal_extractor.rs`](../../crates/custos-runtime/src/cognitive/signal_extractor.rs) | 201 | Extracts routing signals from a user prompt and available execution context. | `struct SignalExtractor`, `fn default`, `fn new`, `fn extract` |
| [`src/context/compaction/format.rs`](../../crates/custos-runtime/src/context/compaction/format.rs) | 83 | Module format: phục vụ các cấu trúc và chức năng liên quan | `fn format_message_for_compacting` |
| [`src/context/compaction/lib.rs`](../../crates/custos-runtime/src/context/compaction/lib.rs) | 70 | Everything compaction reads from the caller's conversation. | `trait CompactionInput`, `fn templates`, `trait CompactionOutput`, `fn messages` |
| [`src/context/compaction/model.rs`](../../crates/custos-runtime/src/context/compaction/model.rs) | 53 | The single completion call compaction needs. Implementations decide model | `trait CompactionModel`, `trait TokenEstimator`, `struct ProviderModel`, `fn new` |
| [`src/context/compaction/prompts/compaction.md`](../../crates/custos-runtime/src/context/compaction/prompts/compaction.md) | 45 | Module compaction: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context/compaction/prompts/compaction_summary.md`](../../crates/custos-runtime/src/context/compaction/prompts/compaction_summary.md) | 75 | Module compaction_summary: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context/compaction/provider.rs`](../../crates/custos-runtime/src/context/compaction/provider.rs) | 143 | Wraps a provider so a `ContextLengthExceeded` response triggers compaction | `struct CompactingProvider`, `fn new`, `fn with_templates`, `fn get_name` |
| [`src/context/compaction/structured.rs`](../../crates/custos-runtime/src/context/compaction/structured.rs) | 484 | Structured output of the compaction LLM call. | `struct StructuredSummary`, `struct FileActivity`, `fn stringify_lenient`, `fn lenient_string_list` |
| [`src/context/compaction/summarize.rs`](../../crates/custos-runtime/src/context/compaction/summarize.rs) | 264 | Drops tool responses from the middle outwards, where context is least | `struct SummarizeContext`, `struct Summary`, `fn has_tool_response`, `fn filter_tool_responses` |
| [`src/context/compaction/templates.rs`](../../crates/custos-runtime/src/context/compaction/templates.rs) | 59 | Prompt sources for a compaction run, letting callers substitute | `fn builtin_template`, `struct Templates`, `fn default`, `fn code_fence` |
| [`src/context/compiler.rs`](../../crates/custos-runtime/src/context/compiler.rs) | 216 | Token-aware compiler that packs relevant context items under a strict token budget. | `struct SourceDocument`, `fn new`, `fn from_file`, `struct TokenAwareContextCompiler` |
| [`src/context/compiler/compiler.rs`](../../crates/custos-runtime/src/context/compiler/compiler.rs) | 214 | Token-aware compiler that packs relevant context items under a strict token budget. | `struct SourceDocument`, `fn new`, `fn from_file`, `struct TokenAwareContextCompiler` |
| [`src/context/compiler/lib.rs`](../../crates/custos-runtime/src/context/compiler/lib.rs) | 7 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context/compiler/recipe.rs`](../../crates/custos-runtime/src/context/compiler/recipe.rs) | 56 | Module recipe: phục vụ các cấu trúc và chức năng liên quan | `struct EngineeringRecipe`, `fn new` |
| [`src/context/compiler/traits.rs`](../../crates/custos-runtime/src/context/compiler/traits.rs) | 13 | Module traits: phục vụ các cấu trúc và chức năng liên quan | `struct ContextSlice`, `trait ContextBuilder` |
| [`src/context/memory/lib.rs`](../../crates/custos-runtime/src/context/memory/lib.rs) | 2 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context/memory/traits.rs`](../../crates/custos-runtime/src/context/memory/traits.rs) | 17 | Module traits: phục vụ các cấu trúc và chức năng liên quan | `enum MemoryTier`, `trait MemoryStore` |
| [`src/context/mod.rs`](../../crates/custos-runtime/src/context/mod.rs) | 10 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context/repo_intelligence/graph.rs`](../../crates/custos-runtime/src/context/repo_intelligence/graph.rs) | 323 | Canonical identifier for any node in the codebase. | `struct SymbolId`, `fn crate_node`, `fn file_node`, `fn symbol_node` |
| [`src/context/repo_intelligence/lib.rs`](../../crates/custos-runtime/src/context/repo_intelligence/lib.rs) | 23 | Module lib: phục vụ các cấu trúc và chức năng liên quan | `struct RepoAnalyzer`, `fn new`, `fn default` |
| [`src/context/repo_intelligence/llm_view.rs`](../../crates/custos-runtime/src/context/repo_intelligence/llm_view.rs) | 232 | Formatting modes for LLM prompts. | `enum LlmViewFormat`, `struct LlmContextBuilder`, `fn new`, `fn build_repo_skeleton` |
| [`src/context/repo_intelligence/scanner.rs`](../../crates/custos-runtime/src/context/repo_intelligence/scanner.rs) | 241 | Recursively scans root directory and returns list of source files. | `struct FileEntry`, `struct SymbolRef`, `struct WorkspaceScanner`, `fn new` |
| [`src/context/token_counter.rs`](../../crates/custos-runtime/src/context/token_counter.rs) | 326 | Module token_counter: phục vụ các cấu trúc và chức năng liên quan | `struct TokenCounter`, `struct TokenCacheKey`, `fn from_text`, `fn count_tokens` |
| [`src/context/traits.rs`](../../crates/custos-runtime/src/context/traits.rs) | 13 | Module traits: phục vụ các cấu trúc và chức năng liên quan | `struct ContextSlice`, `trait ContextBuilder` |
| [`src/context_management/format.rs`](../../crates/custos-runtime/src/context_management/format.rs) | 83 | Module format: phục vụ các cấu trúc và chức năng liên quan | `fn format_message_for_compacting` |
| [`src/context_management/mod.rs`](../../crates/custos-runtime/src/context_management/mod.rs) | 72 | Everything compaction reads from the caller's conversation. | `trait CompactionInput`, `fn templates`, `trait CompactionOutput`, `fn messages` |
| [`src/context_management/model.rs`](../../crates/custos-runtime/src/context_management/model.rs) | 53 | The single completion call compaction needs. Implementations decide model | `trait CompactionModel`, `trait TokenEstimator`, `struct ProviderModel`, `fn new` |
| [`src/context_management/orchestrator.rs`](../../crates/custos-runtime/src/context_management/orchestrator.rs) | 110 | Checks if a conversation has exceeded the compaction threshold | `struct CompactionResult`, `struct CompactionConfig`, `fn default`, `struct CompactionManager` |
| [`src/context_management/prompts/compaction.md`](../../crates/custos-runtime/src/context_management/prompts/compaction.md) | 45 | Module compaction: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context_management/prompts/compaction_summary.md`](../../crates/custos-runtime/src/context_management/prompts/compaction_summary.md) | 75 | Module compaction_summary: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/context_management/provider.rs`](../../crates/custos-runtime/src/context_management/provider.rs) | 143 | Wraps a provider so a `ContextLengthExceeded` response triggers compaction | `struct CompactingProvider`, `fn new`, `fn with_templates`, `fn get_name` |
| [`src/context_management/structured.rs`](../../crates/custos-runtime/src/context_management/structured.rs) | 483 | Structured output of the compaction LLM call. | `struct StructuredSummary`, `struct FileActivity`, `fn stringify_lenient`, `fn lenient_string_list` |
| [`src/context_management/summarize.rs`](../../crates/custos-runtime/src/context_management/summarize.rs) | 264 | Drops tool responses from the middle outwards, where context is least | `struct SummarizeContext`, `struct Summary`, `fn has_tool_response`, `fn filter_tool_responses` |
| [`src/context_management/templates.rs`](../../crates/custos-runtime/src/context_management/templates.rs) | 59 | Prompt sources for a compaction run, letting callers substitute | `fn builtin_template`, `struct Templates`, `fn default`, `fn code_fence` |
| [`src/engine/action_required_manager.rs`](../../crates/custos-runtime/src/engine/action_required_manager.rs) | 657 | Module action_required_manager: phục vụ các cấu trúc và chức năng liên quan | `enum ElicitationOutcome`, `struct PendingRequest`, `struct PendingResponseClaim`, `fn submit` |
| [`src/engine/agents/container.rs`](../../crates/custos-runtime/src/engine/agents/container.rs) | 15 | Module container: phục vụ các cấu trúc và chức năng liên quan | `struct Container`, `fn new`, `fn id` |
| [`src/engine/agents/execute_commands.rs`](../../crates/custos-runtime/src/engine/agents/execute_commands.rs) | 702 | Whether a slash command should kick off an agent turn instead of just | `fn slash_commands_enabled`, `struct CommandDef`, `struct ParsedSlashCommand`, `fn parse_slash_command` |
| [`src/engine/agents/extension.rs`](../../crates/custos-runtime/src/engine/agents/extension.rs) | 1209 | Constructs a new Envs, skipping disallowed env vars with a warning | `struct ProcessExit`, `fn new`, `enum ExtensionError`, `fn from` |
| [`src/engine/agents/extension_malware_check.rs`](../../crates/custos-runtime/src/engine/agents/extension_malware_check.rs) | 1110 | Constructs a checker. Honors OSV_ENDPOINT env var if present. | `struct OsvChecker`, `fn new`, `fn with_endpoint`, `fn command_ecosystem` |
| [`src/engine/agents/extension_manager/builtin.rs`](../../crates/custos-runtime/src/engine/agents/extension_manager/builtin.rs) | 41 | Module builtin: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/extension_manager/mod.rs`](../../crates/custos-runtime/src/engine/agents/extension_manager/mod.rs) | 2782 | Manages goose extensions / MCP clients and their interactions | `type McpClientBox`, `struct ActionRequiredStream`, `fn new`, `type Item` |
| [`src/engine/agents/extension_manager/stdio.rs`](../../crates/custos-runtime/src/engine/agents/extension_manager/stdio.rs) | 101 | Module stdio: phục vụ các cấu trúc và chức năng liên quan | `fn docker_exec`, `fn resolve_command` |
| [`src/engine/agents/extension_manager/streamable_http.rs`](../../crates/custos-runtime/src/engine/agents/extension_manager/streamable_http.rs) | 1152 | Retry with OAuth for typed auth challenges and wrapped bare HTTP 401 responses. | `fn is_oauth_auth_failure`, `fn should_attempt_oauth_fallback`, `fn auth_challenge_from_error`, `fn auth_challenge_from_result` |
| [`src/engine/agents/final_output_tool.rs`](../../crates/custos-runtime/src/engine/agents/final_output_tool.rs) | 300 | Định nghĩa ToolDefinition, ToolResult | `fn structured_output_unsupported_message`, `struct FinalOutputTool`, `fn try_new`, `fn tool` |
| [`src/engine/agents/gen_ai_telemetry.rs`](../../crates/custos-runtime/src/engine/agents/gen_ai_telemetry.rs) | 583 | Merge consecutive text and reasoning parts into single entries so that | `fn capture_message_content`, `fn input_messages_json`, `fn simple_input_json`, `fn simple_output_json` |
| [`src/engine/agents/large_response_handler.rs`](../../crates/custos-runtime/src/engine/agents/large_response_handler.rs) | 265 | Process tool response and handle large text content | `fn max_tool_response_size`, `fn process_tool_response`, `fn write_large_text_to_file` |
| [`src/engine/agents/mcp_client.rs`](../../crates/custos-runtime/src/engine/agents/mcp_client.rs) | 1687 | Return the extension's current instructions. The default reads from | `type BoxError`, `type Error`, `fn default_mcp_apps_ui_extensions`, `struct GooseMcpHostInfo` |
| [`src/engine/agents/mod.rs`](../../crates/custos-runtime/src/engine/agents/mod.rs) | 56 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn latest_provider_session_id` |
| [`src/engine/agents/moim.rs`](../../crates/custos-runtime/src/engine/agents/moim.rs) | 318 | The turn's context block: composed once per turn, persisted as an | `fn system_prompt_block`, `fn turn_context_event`, `fn should_skip_moim`, `fn compose_moim` |
| [`src/engine/agents/platform_extensions/analyze/format.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/analyze/format.rs) | 458 | Module format: phục vụ các cấu trúc và chức năng liên quan | `fn format_structure`, `fn format_semantic`, `fn format_symbol_list`, `fn format_focused` |
| [`src/engine/agents/platform_extensions/analyze/graph.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/analyze/graph.rs) | 373 | (file_path, symbol_name, definition_line) — line disambiguates same-name | `type NodeKey`, `struct ChainLink`, `struct Node`, `struct CallGraph` |
| [`src/engine/agents/platform_extensions/analyze/languages.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/analyze/languages.rs) | 326 | Module languages: phục vụ các cấu trúc và chức năng liên quan | `struct LangInfo`, `struct LangQueries`, `fn lang_for_ext` |
| [`src/engine/agents/platform_extensions/analyze/mod.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/analyze/mod.rs) | 445 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `struct AnalyzeParams`, `fn default_max_depth`, `fn default_follow_depth`, `struct AnalyzeClient` |
| [`src/engine/agents/platform_extensions/analyze/parser.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/analyze/parser.rs) | 758 | Recursively collect Swift init_declaration and deinit_declaration nodes. | `struct FileAnalysis`, `struct Symbol`, `struct Import`, `struct Call` |
| [`src/engine/agents/platform_extensions/apps.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/apps.rs) | 1078 | Parameters for iterate_app tool | `struct CreateAppParams`, `struct IterateAppParams`, `struct DeleteAppParams`, `struct ListAppsParams` |
| [`src/engine/agents/platform_extensions/chatrecall.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/chatrecall.rs) | 481 | Module chatrecall: phục vụ các cấu trúc và chức năng liên quan | `struct ChatRecallParams`, `struct ChatRecallClient`, `fn agent_only_history`, `fn format_agent_visible_excerpt` |
| [`src/engine/agents/platform_extensions/code_execution.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/code_execution.rs) | 1123 | Get the cached CodeMode, rebuilding if callback configs have changed | `struct CodeExecutionClient`, `struct ToolGraphNode`, `struct ExecuteWithToolGraph`, `fn new` |
| [`src/engine/agents/platform_extensions/developer/edit.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/edit.rs) | 512 | Module edit: phục vụ các cấu trúc và chức năng liên quan | `struct FileReadParams`, `struct FileWriteParams`, `struct FileEditParams`, `struct EditTools` |
| [`src/engine/agents/platform_extensions/developer/image.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/image.rs) | 532 | Module image: phục vụ các cấu trúc và chức năng liên quan | `fn visible_text`, `struct ImageReadParams`, `struct CropParams`, `struct ImageTool` |
| [`src/engine/agents/platform_extensions/developer/mod.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/mod.rs) | 405 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn visible_text`, `struct DeveloperClient`, `fn developer_instructions`, `fn new` |
| [`src/engine/agents/platform_extensions/developer/shell.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/shell.rs) | 1408 | Check if the current process is running inside a Flatpak sandbox. | `fn is_flatpak`, `fn flatpak_spawn_command`, `fn flatpak_spawn_process`, `enum UnixShellFlavor` |
| [`src/engine/agents/platform_extensions/developer/shell_output_streaming.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/shell_output_streaming.rs) | 226 | Module shell_output_streaming: phục vụ các cấu trúc và chức năng liên quan | `enum ShellOutputStream`, `struct ShellOutputNotificationChunk`, `struct ShellOutputNotificationParams`, `fn parse_shell_output_notification` |
| [`src/engine/agents/platform_extensions/developer/tree.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/developer/tree.rs) | 299 | Module tree: phục vụ các cấu trúc và chức năng liên quan | `struct TreeParams`, `fn default_depth`, `struct TreeTool`, `fn new` |
| [`src/engine/agents/platform_extensions/ext_manager.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/ext_manager.rs) | 720 | Module ext_manager: phục vụ các cấu trúc và chức năng liên quan | `enum ExtensionManagerToolError`, `enum ManageExtensionAction`, `struct ManageExtensionsParams`, `struct ReadResourceParams` |
| [`src/engine/agents/platform_extensions/mod.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/mod.rs) | 314 | Definition for a platform extension that runs in-process with direct agent access. | `struct PlatformExtensionContext`, `fn result_with_platform_notification`, `struct PlatformExtensionDef` |
| [`src/engine/agents/platform_extensions/orchestrator.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/orchestrator.rs) | 1081 | Module orchestrator: phục vụ các cấu trúc và chức năng liên quan | `struct CancelTokenGuard`, `fn new`, `fn disarm`, `fn drop` |
| [`src/engine/agents/platform_extensions/scheduler.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/scheduler.rs) | 75 | Module scheduler: phục vụ các cấu trúc và chức năng liên quan | `struct SchedulerClient`, `fn new`, `fn get_info` |
| [`src/engine/agents/platform_extensions/summarize.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/summarize.rs) | 559 | Module summarize: phục vụ các cấu trúc và chức năng liên quan | `struct SummarizeParams`, `struct SummarizeClient`, `fn new`, `fn get_tools` |
| [`src/engine/agents/platform_extensions/summon.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/summon.rs) | 4434 | Result from handle_load_task_result with structured metadata for the caller | `fn durable_assistant_turn_count`, `fn kind_plural`, `struct DelegateParams`, `struct BackgroundTask` |
| [`src/engine/agents/platform_extensions/todo.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/todo.rs) | 196 | Module todo: phục vụ các cấu trúc và chức năng liên quan | `struct TodoWriteParams`, `struct TodoClient`, `fn new`, `fn get_tools` |
| [`src/engine/agents/platform_extensions/tom.rs`](../../crates/custos-runtime/src/engine/agents/platform_extensions/tom.rs) | 117 | Module tom: phục vụ các cấu trúc và chức năng liên quan | `struct TomClient`, `fn new`, `fn get_info`, `fn truncate_utf8` |
| [`src/engine/agents/platform_tools.rs`](../../crates/custos-runtime/src/engine/agents/platform_tools.rs) | 41 | Module platform_tools: phục vụ các cấu trúc và chức năng liên quan | `fn manage_schedule_tool` |
| [`src/engine/agents/prompt_manager.rs`](../../crates/custos-runtime/src/engine/agents/prompt_manager.rs) | 571 | Add an additional instruction to the system prompt with a key | `struct PromptManager`, `fn default`, `struct SystemPromptContext`, `struct SystemPromptBuilder` |
| [`src/engine/agents/reply_parts.rs`](../../crates/custos-runtime/src/engine/agents/reply_parts.rs) | 2139 | Fill `usage.stats` timing fields measured by the stream wrapper, keeping any | `fn coerce_value`, `fn try_coerce_number`, `fn try_coerce_boolean`, `fn coerce_tool_arguments` |
| [`src/engine/agents/retry.rs`](../../crates/custos-runtime/src/engine/agents/retry.rs) | 572 | Result of a retry logic evaluation | `enum RetryResult`, `struct RetryManager`, `fn default`, `fn new` |
| [`src/engine/agents/schedule_tool.rs`](../../crates/custos-runtime/src/engine/agents/schedule_tool.rs) | 526 | Run a scheduled job immediately | `fn recipe_file_error`, `fn read_schedule_recipe`, `struct ScheduleTool`, `fn new` |
| [`src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__all_platform_extensions.snap`](../../crates/custos-runtime/src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__all_platform_extensions.snap) | 156 | Module custos_engine__agents__prompt_manager__tests__all_platform_extensions: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__basic.snap`](../../crates/custos-runtime/src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__basic.snap) | 36 | Module custos_engine__agents__prompt_manager__tests__basic: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__one_extension.snap`](../../crates/custos-runtime/src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__one_extension.snap) | 46 | Module custos_engine__agents__prompt_manager__tests__one_extension: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__typical_setup.snap`](../../crates/custos-runtime/src/engine/agents/snapshots/custos_engine__agents__prompt_manager__tests__typical_setup.snap) | 50 | Module custos_engine__agents__prompt_manager__tests__typical_setup: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/effects.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/effects.rs) | 51 | Module effects: phục vụ các cấu trúc và chức năng liên quan | `enum GooseEffect`, `fn ensure_message_ids`, `fn from`, `fn from` |
| [`src/engine/agents/state_machine/inference_preparation.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/inference_preparation.rs) | 85 | Module inference_preparation: phục vụ các cấu trúc và chức năng liên quan | `struct GooseInferenceRequestPreparer` |
| [`src/engine/agents/state_machine/mod.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/mod.rs) | 76 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn enabled` |
| [`src/engine/agents/state_machine/ops_bang_shell.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_bang_shell.rs) | 75 | Module ops_bang_shell: phục vụ các cấu trúc và chức năng liên quan | `fn bang_shell_command`, `struct BangShellOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_compaction.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_compaction.rs) | 367 | Several operations answer parts of one tool batch in separate messages, so a | `fn compaction_part`, `fn awaits_tool_responses`, `struct CompactionOperation`, `fn new` |
| [`src/engine/agents/state_machine/ops_doctor.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_doctor.rs) | 78 | Module ops_doctor: phục vụ các cấu trúc và chức năng liên quan | `struct DoctorOperation`, `fn name` |
| [`src/engine/agents/state_machine/ops_entry_hook.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_entry_hook.rs) | 77 | Module ops_entry_hook: phục vụ các cấu trúc và chức năng liên quan | `struct EntryHookOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_exit_on_error.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_exit_on_error.rs) | 33 | Phân loại lỗi hệ thống DomainError thuần khiết | `struct ExitOnErrorOperation`, `fn name` |
| [`src/engine/agents/state_machine/ops_llm.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_llm.rs) | 238 | Module ops_llm: phục vụ các cấu trúc và chức năng liên quan | `struct GooseInferenceProvider`, `fn new`, `fn record_usage`, `fn enrich_unclaimed_tool_errors` |
| [`src/engine/agents/state_machine/ops_maxturns.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_maxturns.rs) | 68 | Module ops_maxturns: phục vụ các cấu trúc và chức năng liên quan | `struct MaxTurnsOperation`, `fn turn_budget_part`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_project.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_project.rs) | 39 | Module ops_project: phục vụ các cấu trúc và chức năng liên quan | `struct ProjectOperation`, `fn name` |
| [`src/engine/agents/state_machine/ops_recipe.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_recipe.rs) | 411 | Module ops_recipe: phục vụ các cấu trúc và chức năng liên quan | `struct RecipeOperation`, `fn new`, `fn final_output`, `fn assistant_block_bounds` |
| [`src/engine/agents/state_machine/ops_retry.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_retry.rs) | 285 | Attempts are counted on the kickoff message because a retry replaces the | `fn retry_error`, `struct RetryOperation`, `fn new`, `fn retry_config` |
| [`src/engine/agents/state_machine/ops_skills.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_skills.rs) | 432 | Module ops_skills: phục vụ các cấu trúc và chức năng liên quan | `struct SkillOperation`, `struct LoadSkillParams`, `fn skill_tool`, `fn skill_instructions` |
| [`src/engine/agents/state_machine/ops_slash_command.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_slash_command.rs) | 84 | Module ops_slash_command: phục vụ các cấu trúc và chức năng liên quan | `struct SlashCommandOperation`, `fn parse_slash_command`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_status.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_status.rs) | 85 | Module ops_status: phục vụ các cấu trúc và chức năng liên quan | `struct StatusOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_steer.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_steer.rs) | 78 | Module ops_steer: phục vụ các cấu trúc và chức năng liên quan | `type SteerQueue`, `struct SteerOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_stop_hook.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_stop_hook.rs) | 120 | Module ops_stop_hook: phục vụ các cấu trúc và chức năng liên quan | `fn denial_context_message`, `fn denial_notification`, `fn block_cap_warning`, `struct StopHookOperation` |
| [`src/engine/agents/state_machine/ops_tool_approval.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_tool_approval.rs) | 295 | Định nghĩa quy trình duyệt ApprovalRequest, ApprovalDecision và ApprovalStatus | `struct ToolApprovalOperation`, `fn new`, `fn name`, `struct PendingResponse` |
| [`src/engine/agents/state_machine/ops_tool_pair_compaction.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_tool_pair_compaction.rs) | 167 | Định nghĩa Action, ActionIntent, ActionLifecycleState và đánh giá RiskLevel cho các hành động | `struct ToolPairCompactionOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/ops_toolcalling.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_toolcalling.rs) | 1082 | Observation-only record of what the `PreToolUse` chain decided. Carries | `enum ToolCategory`, `fn categorize_tool`, `fn string_argument`, `fn platform_notification` |
| [`src/engine/agents/state_machine/ops_unknown_tool.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/ops_unknown_tool.rs) | 198 | Định nghĩa ToolDefinition, ToolResult | `struct UnknownToolOperation`, `fn new`, `fn name` |
| [`src/engine/agents/state_machine/session.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/session.rs) | 312 | Thực thể Session, SessionId, SessionStatus, nhật ký SessionJournalEntry | `fn contains_tool_confirmation_request`, `fn id`, `fn conversation`, `fn usage` |
| [`src/engine/agents/state_machine/tests/agent_reply.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/agent_reply.rs) | 538 | Module agent_reply: phục vụ các cấu trúc và chức năng liên quan | `fn confirmation_ids`, `fn assistant_only_acp_annotations`, `fn assistant_only_acp_text`, `fn empty_audience_acp_annotations` |
| [`src/engine/agents/state_machine/tests/calculator_extension.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/calculator_extension.rs) | 367 | Module calculator_extension: phục vụ các cấu trúc và chức năng liên quan | `fn value`, `fn delayed_value`, `fn named_values`, `struct ValueParams` |
| [`src/engine/agents/state_machine/tests/compaction_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/compaction_lifecycle.rs) | 503 | Module compaction_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/dummy_api.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/dummy_api.rs) | 990 | Bộ điều phối Local API tiếp nhận yêu cầu từ client | `struct ProviderFeatures`, `fn default`, `enum ApiResponse`, `struct ApiToolCall` |
| [`src/engine/agents/state_machine/tests/hooks_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/hooks_lifecycle.rs) | 1617 | Plugin fixture that can register several events at once, each with its own | `struct HookTestEnv`, `fn new`, `fn hook_manager`, `fn invocations` |
| [`src/engine/agents/state_machine/tests/mod.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/mod.rs) | 290 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/pipeline.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/pipeline.rs) | 1107 | Module pipeline: phục vụ các cấu trúc và chức năng liên quan | `struct FeatureProvider`, `fn get_name`, `fn manages_own_context`, `struct TestPipeline` |
| [`src/engine/agents/state_machine/tests/prompt_skill_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/prompt_skill_lifecycle.rs) | 142 | Module prompt_skill_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/provider_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/provider_lifecycle.rs) | 355 | Module provider_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/recipe_scheduling_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/recipe_scheduling_lifecycle.rs) | 589 | Module recipe_scheduling_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/reconstruction_isolation_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/reconstruction_isolation_lifecycle.rs) | 212 | Module reconstruction_isolation_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/steering_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/steering_lifecycle.rs) | 124 | Module steering_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tests/tool_lifecycle.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tests/tool_lifecycle.rs) | 507 | Module tool_lifecycle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/state_machine/tool_confirmation.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/tool_confirmation.rs) | 117 | Module tool_confirmation: phục vụ các cấu trúc và chức năng liên quan | `fn active_turn_messages`, `fn pending_tool_confirmations`, `fn has_unapplied_tool_confirmation_response` |
| [`src/engine/agents/state_machine/usage.rs`](../../crates/custos-runtime/src/engine/agents/state_machine/usage.rs) | 84 | Module usage: phục vụ các cấu trúc và chức năng liên quan | `fn attach_to_last_assistant`, `fn enrich` |
| [`src/engine/agents/subagent_execution_tool/mod.rs`](../../crates/custos-runtime/src/engine/agents/subagent_execution_tool/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/agents/subagent_execution_tool/notification_events.rs`](../../crates/custos-runtime/src/engine/agents/subagent_execution_tool/notification_events.rs) | 222 | Convert event to JSON format for MCP notification | `enum TaskStatus`, `fn fmt`, `enum TaskExecutionNotificationEvent`, `struct TaskExecutionStats` |
| [`src/engine/agents/subagent_handler.rs`](../../crates/custos-runtime/src/engine/agents/subagent_handler.rs) | 364 | Module subagent_handler: phục vụ các cấu trúc và chức năng liên quan | `type OnMessageCallback`, `struct SubagentPromptContext`, `type AgentMessagesFuture`, `struct SubagentRunParams` |
| [`src/engine/agents/subagent_task_config.rs`](../../crates/custos-runtime/src/engine/agents/subagent_task_config.rs) | 62 | Configuration for task execution with all necessary dependencies | `struct TaskConfig`, `fn fmt`, `fn new`, `fn with_max_turns` |
| [`src/engine/agents/tool_confirmation_coordinator.rs`](../../crates/custos-runtime/src/engine/agents/tool_confirmation_coordinator.rs) | 245 | Module tool_confirmation_coordinator: phục vụ các cấu trúc và chức năng liên quan | `enum ConfirmationAnswer`, `struct SessionToolConfirmationState`, `fn new`, `fn try_start_turn` |
| [`src/engine/agents/tool_confirmation_router.rs`](../../crates/custos-runtime/src/engine/agents/tool_confirmation_router.rs) | 182 | Module tool_confirmation_router: phục vụ các cấu trúc và chức năng liên quan | `struct ToolConfirmationRouter`, `fn new` |
| [`src/engine/agents/tool_execution.rs`](../../crates/custos-runtime/src/engine/agents/tool_execution.rs) | 245 | Context passed through the tool call dispatch chain. | `struct ToolCallNotificationEmitter`, `fn new`, `fn emit_best_effort`, `struct ToolCallContext` |
| [`src/engine/agents/tool_schema_normalize.rs`](../../crates/custos-runtime/src/engine/agents/tool_schema_normalize.rs) | 1068 | Normalize an rmcp tool `input_schema` in place, returning `true` if changed. | `fn normalize_input_schema`, `fn collapse_const_unions`, `fn dialect_predates_const`, `fn ref_siblings_ignored` |
| [`src/engine/agents/types.rs`](../../crates/custos-runtime/src/engine/agents/types.rs) | 146 | Configuration for retry logic in recipe execution | `type SharedProvider`, `struct RetryConfig`, `fn validate`, `enum SuccessCheck` |
| [`src/engine/agents/validate_extensions.rs`](../../crates/custos-runtime/src/engine/agents/validate_extensions.rs) | 247 | Module validate_extensions: phục vụ các cấu trúc và chức năng liên quan | `struct BundledExtensionEntry`, `fn validate_bundled_extensions`, `fn write_json` |
| [`src/engine/builtin_extension.rs`](../../crates/custos-runtime/src/engine/builtin_extension.rs) | 28 | Register a builtin extension into the global registry | `type SpawnServerFn`, `fn register_builtin_extension`, `fn register_builtin_extensions`, `fn get_builtin_extension` |
| [`src/engine/checks/mod.rs`](../../crates/custos-runtime/src/engine/checks/mod.rs) | 922 | Parsed YAML frontmatter for a check file. | `struct CheckFrontmatter`, `struct Check`, `fn from_path`, `fn parse` |
| [`src/engine/config/base.rs`](../../crates/custos-runtime/src/engine/config/base.rs) | 2951 | Configuration management for goose. | `fn write_secrets_file`, `fn secrets_lock_path`, `enum ConfigError`, `fn from` |
| [`src/engine/config/declarative_providers.rs`](../../crates/custos-runtime/src/engine/config/declarative_providers.rs) | 1068 | Expand `${VAR_NAME}` placeholders in a template string using the given env var configs. | `fn custom_providers_dir`, `fn expand_env_vars`, `struct LoadedProvider`, `fn generate_id` |
| [`src/engine/config/experiments.rs`](../../crates/custos-runtime/src/engine/config/experiments.rs) | 57 | Experiment configuration management | `struct ExperimentManager`, `fn get_all`, `fn set_enabled`, `fn is_enabled` |
| [`src/engine/config/extensions.rs`](../../crates/custos-runtime/src/engine/config/extensions.rs) | 815 | Returns true when an existing extension was updated, false when the key was missing. | `struct ExtensionEntry`, `fn name_to_key`, `fn is_extension_available`, `fn inject_name_if_missing` |
| [`src/engine/config/migrations.rs`](../../crates/custos-runtime/src/engine/config/migrations.rs) | 613 | Run only non-destructive migrations suitable for in-memory read paths. | `fn run_migrations`, `fn run_read_migrations`, `fn read_enabled_field`, `fn migrate_platform_extensions` |
| [`src/engine/config/mod.rs`](../../crates/custos-runtime/src/engine/config/mod.rs) | 35 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/paths.rs`](../../crates/custos-runtime/src/engine/config/paths.rs) | 120 | Module paths: phục vụ các cấu trúc và chức năng liên quan | `struct Paths`, `fn get_dir`, `fn path_root`, `fn validated_path_root` |
| [`src/engine/config/permission.rs`](../../crates/custos-runtime/src/engine/config/permission.rs) | 590 | Enum representing the possible permission levels for a tool. | `enum PermissionLevel`, `struct PermissionConfig`, `struct PermissionManager`, `fn new` |
| [`src/engine/config/providers.rs`](../../crates/custos-runtime/src/engine/config/providers.rs) | 200 | Module providers: phục vụ các cấu trúc và chức năng liên quan | `struct ProviderEntry`, `fn parse_providers_map`, `fn get_providers_map`, `fn get_provider_entry` |
| [`src/engine/config/search_path.rs`](../../crates/custos-runtime/src/engine/config/search_path.rs) | 123 | Module search_path: phục vụ các cấu trúc và chức năng liên quan | `struct SearchPaths`, `fn builder`, `fn with_npm`, `fn path` |
| [`src/engine/config/signup_openrouter/mod.rs`](../../crates/custos-runtime/src/engine/config/signup_openrouter/mod.rs) | 174 | Start local server and wait for callback | `struct PkceAuthFlow`, `struct TokenResponse`, `struct TokenRequest`, `fn new` |
| [`src/engine/config/signup_openrouter/server.rs`](../../crates/custos-runtime/src/engine/config/signup_openrouter/server.rs) | 126 | Run the callback server on localhost:3000 | `struct CallbackQuery` |
| [`src/engine/config/signup_openrouter/templates/error.html`](../../crates/custos-runtime/src/engine/config/signup_openrouter/templates/error.html) | 50 | Module error: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_openrouter/templates/invalid.html`](../../crates/custos-runtime/src/engine/config/signup_openrouter/templates/invalid.html) | 39 | Module invalid: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_openrouter/templates/success.html`](../../crates/custos-runtime/src/engine/config/signup_openrouter/templates/success.html) | 45 | Module success: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_openrouter/tests.rs`](../../crates/custos-runtime/src/engine/config/signup_openrouter/tests.rs) | 65 | Module tests: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_tetrate/mod.rs`](../../crates/custos-runtime/src/engine/config/signup_tetrate/mod.rs) | 171 | Complete flow: start server, open browser, wait for callback, exchange code | `struct PkceAuthFlow`, `struct TokenResponse`, `struct TokenRequest`, `fn new` |
| [`src/engine/config/signup_tetrate/server.rs`](../../crates/custos-runtime/src/engine/config/signup_tetrate/server.rs) | 123 | Run the callback server using the provided listener. | `struct CallbackQuery` |
| [`src/engine/config/signup_tetrate/templates/error.html`](../../crates/custos-runtime/src/engine/config/signup_tetrate/templates/error.html) | 85 | Module error: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_tetrate/templates/invalid.html`](../../crates/custos-runtime/src/engine/config/signup_tetrate/templates/invalid.html) | 77 | Module invalid: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_tetrate/templates/success.html`](../../crates/custos-runtime/src/engine/config/signup_tetrate/templates/success.html) | 76 | Module success: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/signup_tetrate/tests.rs`](../../crates/custos-runtime/src/engine/config/signup_tetrate/tests.rs) | 86 | Module tests: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/config/tls.rs`](../../crates/custos-runtime/src/engine/config/tls.rs) | 38 | Module tls: phục vụ các cấu trúc và chức năng liên quan | `fn provider_tls_config_from_config` |
| [`src/engine/context_limit.rs`](../../crates/custos-runtime/src/engine/context_limit.rs) | 41 | Module context_limit: phục vụ các cấu trúc và chức năng liên quan | `fn get_local_context_limit` |
| [`src/engine/context_mgmt/mod.rs`](../../crates/custos-runtime/src/engine/context_mgmt/mod.rs) | 1348 | Compact messages by summarizing them | `fn tool_pair_summarization_enabled`, `struct CompactionResult`, `struct GooseCompactionModel`, `struct GooseTokenEstimator` |
| [`src/engine/doctor.rs`](../../crates/custos-runtime/src/engine/doctor.rs) | 320 | Module doctor: phục vụ các cấu trúc và chức năng liên quan | `fn is_developer_platform_config`, `fn describe_error`, `fn custom_extension_named_developer_does_not_satisfy_requirement` |
| [`src/engine/download_manager.rs`](../../crates/custos-runtime/src/engine/download_manager.rs) | 14 | Module download_manager: phục vụ các cấu trúc và chức năng liên quan | `fn local_inference_uses_same_download_manager` |
| [`src/engine/elicitation.rs`](../../crates/custos-runtime/src/engine/elicitation.rs) | 72 | Module elicitation: phục vụ các cấu trúc và chức năng liên quan | `fn elicitation_response_user_data`, `fn elicitation_response_action`, `fn generated_elicitation_response_message` |
| [`src/engine/execution/active_run.rs`](../../crates/custos-runtime/src/engine/execution/active_run.rs) | 244 | Vòng đời thực thi Run, WorkerRun, RunStatus | `struct ActiveRun`, `struct SessionRunState`, `enum StartRunError`, `struct ActiveRunRegistry` |
| [`src/engine/execution/manager.rs`](../../crates/custos-runtime/src/engine/execution/manager.rs) | 899 | Get the shared SessionManager for session-only operations | `struct RuntimeContext`, `struct AgentManagerGetResult`, `struct AgentManager`, `fn scheduler` |
| [`src/engine/execution/mod.rs`](../../crates/custos-runtime/src/engine/execution/mod.rs) | 49 | Create an interactive chat mode | `enum SessionExecutionMode`, `fn chat`, `fn scheduled`, `fn task` |
| [`src/engine/gateway/handler.rs`](../../crates/custos-runtime/src/engine/gateway/handler.rs) | 950 | Resolve the max turns to use for a gateway session. | `fn resolve_gateway_max_turns`, `struct PendingConfirmation`, `type PerUserLocks`, `struct GatewayHandler` |
| [`src/engine/gateway/manager.rs`](../../crates/custos-runtime/src/engine/gateway/manager.rs) | 439 | Serialized form stored in goose config (no secrets). | `fn secret_key_for`, `struct SavedGatewayEntry`, `fn allowed_user_ids_from_platform_config`, `fn saved_allowed_user_ids` |
| [`src/engine/gateway/mod.rs`](../../crates/custos-runtime/src/engine/gateway/mod.rs) | 100 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `struct PlatformUser`, `fn eq`, `fn hash`, `struct IncomingMessage` |
| [`src/engine/gateway/pairing.rs`](../../crates/custos-runtime/src/engine/gateway/pairing.rs) | 893 | Module pairing: phục vụ các cấu trúc và chức năng liên quan | `struct StoredPairing`, `struct StoredPendingCode`, `struct StoredPendingCodes`, `fn revoke_imported_codes` |
| [`src/engine/gateway/telegram.rs`](../../crates/custos-runtime/src/engine/gateway/telegram.rs) | 1986 | Audio files sent as documents (not inline voice notes). | `struct VoiceFileIdentity`, `struct VoiceTempFile`, `fn remove`, `fn drop` |
| [`src/engine/hints/import_files.rs`](../../crates/custos-runtime/src/engine/hints/import_files.rs) | 1416 | Module import_files: phục vụ các cấu trúc và chức năng liên quan | `struct FileReference`, `struct ExpansionBudget`, `struct ImportBoundary`, `fn new` |
| [`src/engine/hints/load_hints.rs`](../../crates/custos-runtime/src/engine/hints/load_hints.rs) | 1039 | Build a `Gitignore` that includes `.gitignore` files from the git root | `fn get_context_filenames`, `struct SubdirectoryHintTracker`, `fn new`, `fn record_tool_arguments` |
| [`src/engine/hints/mod.rs`](../../crates/custos-runtime/src/engine/hints/mod.rs) | 7 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/hooks/mod.rs`](../../crates/custos-runtime/src/engine/hooks/mod.rs) | 2330 | Lifecycle events a hook can subscribe to. | `enum HookEvent`, `fn name`, `fn from_name`, `fn fmt` |
| [`src/engine/instance_id.rs`](../../crates/custos-runtime/src/engine/instance_id.rs) | 49 | Returns a stable, globally unique identifier for this Goose installation. | `fn file_path`, `fn load_or_create`, `fn get_instance_id` |
| [`src/engine/logging.rs`](../../crates/custos-runtime/src/engine/logging.rs) | 272 | Configuration for the shared logging setup. | `struct LoggingConfig`, `fn build_env_filter`, `fn build_logging_subscriber`, `fn prepare_log_directory` |
| [`src/engine/mcp_utils.rs`](../../crates/custos-runtime/src/engine/mcp_utils.rs) | 109 | Module mcp_utils: phục vụ các cấu trúc và chức năng liên quan | `type ToolResult`, `fn extract_text_from_resource` |
| [`src/engine/mod.rs`](../../crates/custos-runtime/src/engine/mod.rs) | 50 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/model_config.rs`](../../crates/custos-runtime/src/engine/model_config.rs) | 341 | Run a completion for a one-shot auxiliary task on the main session model. | `fn model_config_from_user_config`, `fn model_config_from_user_config_with_session_settings`, `fn materialize_model_config`, `fn apply_canonical_limits` |
| [`src/engine/oauth/mod.rs`](../../crates/custos-runtime/src/engine/oauth/mod.rs) | 879 | Pre-registered OAuth client supplied by a probe script, for servers whose | `struct OAuthFlowConfig`, `struct AppState`, `struct CallbackParams`, `fn resolve_oauth_callback_timeout` |
| [`src/engine/oauth/oauth_callback.html`](../../crates/custos-runtime/src/engine/oauth/oauth_callback.html) | 73 | Module oauth_callback: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/oauth/persist.rs`](../../crates/custos-runtime/src/engine/oauth/persist.rs) | 128 | Module persist: phục vụ các cấu trúc và chức năng liên quan | `struct PersistedCredentials`, `struct GooseCredentialStore`, `fn new`, `fn secret_key` |
| [`src/engine/otel/mod.rs`](../../crates/custos-runtime/src/engine/otel/mod.rs) | 1 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/otel/otlp.rs`](../../crates/custos-runtime/src/engine/otel/otlp.rs) | 1140 | One-shot stderr warning when `OTEL_EXPORTER_OTLP_PROTOCOL=grpc` is set | `type OtlpTracingLayer`, `type OtlpMetricsLayer`, `type OtlpLogsLayer`, `type OtlpResult` |
| [`src/engine/permission/mod.rs`](../../crates/custos-runtime/src/engine/permission/mod.rs) | 10 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/permission/permission_inspector.rs`](../../crates/custos-runtime/src/engine/permission/permission_inspector.rs) | 395 | Permission Inspector that handles tool permission checking | `struct PermissionInspector`, `fn cache_non_readonly_decision`, `fn new`, `fn apply_tool_annotations` |
| [`src/engine/permission/permission_judge.rs`](../../crates/custos-runtime/src/engine/permission/permission_judge.rs) | 271 | Creates the tool definition for checking read-only permissions. | `struct PermissionJudgeContext`, `fn create_read_only_tool`, `fn create_check_messages`, `fn extract_read_only_request_ids` |
| [`src/engine/permission/permission_store.rs`](../../crates/custos-runtime/src/engine/permission/permission_store.rs) | 144 | Hiện thực hóa SqliteTaskStore kết nối toàn bộ repository | `struct ToolPermissionRecord`, `struct ToolPermissionStore`, `fn default`, `fn new` |
| [`src/engine/plugins/discovery.rs`](../../crates/custos-runtime/src/engine/plugins/discovery.rs) | 558 | Per-plugin entry stored under the `plugins` map in `config.yaml`, keyed by | `struct PluginConfigEntry`, `struct DiscoveredPlugin`, `enum PluginScope`, `struct PluginSettings` |
| [`src/engine/plugins/formats/gemini.rs`](../../crates/custos-runtime/src/engine/plugins/formats/gemini.rs) | 232 | Module gemini: phục vụ các cấu trúc và chức năng liên quan | `struct GeminiManifest`, `struct SkillCandidate`, `fn try_install_from_manifest_at_root`, `fn validate_extension_name` |
| [`src/engine/plugins/formats/mod.rs`](../../crates/custos-runtime/src/engine/plugins/formats/mod.rs) | 2 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/plugins/formats/open_plugins.rs`](../../crates/custos-runtime/src/engine/plugins/formats/open_plugins.rs) | 848 | Module open_plugins: phục vụ các cấu trúc và chức năng liên quan | `struct OpenPluginsManifest`, `struct SkillCandidate`, `fn try_install_from_manifest_at_root`, `fn install_from_manifest` |
| [`src/engine/plugins/mcp_servers.rs`](../../crates/custos-runtime/src/engine/plugins/mcp_servers.rs) | 335 | Module mcp_servers: phục vụ các cấu trúc và chức năng liên quan | `struct McpServersDocument`, `struct McpServerConfig`, `fn enabled_plugin_mcp_servers`, `fn plugin_mcp_servers` |
| [`src/engine/plugins/mod.rs`](../../crates/custos-runtime/src/engine/plugins/mod.rs) | 602 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `enum PluginFormat`, `fn fmt`, `fn plugin_install_dir`, `fn project_plugin_install_dir` |
| [`src/engine/posthog.rs`](../../crates/custos-runtime/src/engine/posthog.rs) | 604 | Check if the user has made a telemetry choice. | `fn get_telemetry_choice`, `fn is_telemetry_enabled`, `struct CaptureEvent`, `struct InstallationData` |
| [`src/engine/prompt_template.rs`](../../crates/custos-runtime/src/engine/prompt_template.rs) | 276 | Information about a template including its content and customization status | `struct Template`, `fn builtin_content`, `fn user_prompts_dir`, `fn is_registered` |
| [`src/engine/prompts/apps_create.md`](../../crates/custos-runtime/src/engine/prompts/apps_create.md) | 19 | Module apps_create: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/apps_iterate.md`](../../crates/custos-runtime/src/engine/prompts/apps_iterate.md) | 25 | Module apps_iterate: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/permission_judge.md`](../../crates/custos-runtime/src/engine/prompts/permission_judge.md) | 1 | Module permission_judge: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/session_name.md`](../../crates/custos-runtime/src/engine/prompts/session_name.md) | 19 | Module session_name: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/subagent_system.md`](../../crates/custos-runtime/src/engine/prompts/subagent_system.md) | 34 | Module subagent_system: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/system.md`](../../crates/custos-runtime/src/engine/prompts/system.md) | 39 | Module system: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/prompts/tiny_model_system.md`](../../crates/custos-runtime/src/engine/prompts/tiny_model_system.md) | 22 | Module tiny_model_system: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/recipe_deeplink.rs`](../../crates/custos-runtime/src/engine/recipe_deeplink.rs) | 108 | Module recipe_deeplink: phục vụ các cấu trúc và chức năng liên quan | `enum DecodeError`, `fn encode`, `fn decode`, `fn create_test_recipe` |
| [`src/engine/scheduler/common.rs`](../../crates/custos-runtime/src/engine/scheduler/common.rs) | 187 | Module common: phục vụ các cấu trúc và chức năng liên quan | `struct ValidatedScheduleRecipe`, `fn new`, `fn bytes`, `fn open_regular_schedule_recipe` |
| [`src/engine/scheduler/full.rs`](../../crates/custos-runtime/src/engine/scheduler/full.rs) | 1619 | Module full: phục vụ các cấu trúc và chức năng liên quan | `type RunningTasksMap`, `type JobsMap`, `fn read_validated_schedule_recipe`, `fn clear_running_state` |
| [`src/engine/scheduler/mod.rs`](../../crates/custos-runtime/src/engine/scheduler/mod.rs) | 7 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/scheduler_trait.rs`](../../crates/custos-runtime/src/engine/scheduler_trait.rs) | 46 | Module scheduler_trait: phục vụ các cấu trúc và chức năng liên quan | `trait SchedulerTrait` |
| [`src/engine/security/adversary_inspector.rs`](../../crates/custos-runtime/src/engine/security/adversary_inspector.rs) | 747 | Adversary inspector that reviews tool calls against user-defined rules. | `struct AdversaryConfig`, `struct AdversaryInspector`, `fn new`, `fn with_config_dir` |
| [`src/engine/security/classification_client.rs`](../../crates/custos-runtime/src/engine/security/classification_client.rs) | 251 | Request format following HuggingFace Inference Text Classification API specification | `struct ClassificationRequest`, `struct ClassificationLabel`, `type ClassificationResponse`, `struct ModelEndpointInfo` |
| [`src/engine/security/egress_inspector.rs`](../../crates/custos-runtime/src/engine/security/egress_inspector.rs) | 555 | Module egress_inspector: phục vụ các cấu trúc và chức năng liên quan | `struct EgressInspector`, `fn new`, `fn default`, `enum EgressDirection` |
| [`src/engine/security/mod.rs`](../../crates/custos-runtime/src/engine/security/mod.rs) | 265 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn get_override`, `struct SecurityManager`, `struct SecurityResult`, `fn new` |
| [`src/engine/security/patterns.rs`](../../crates/custos-runtime/src/engine/security/patterns.rs) | 588 | Security threat patterns for command injection detection | `struct ThreatPattern`, `enum RiskLevel`, `enum ThreatCategory`, `fn confidence_score` |
| [`src/engine/security/scanner.rs`](../../crates/custos-runtime/src/engine/security/scanner.rs) | 792 | Module scanner: phục vụ các cấu trúc và chức năng liên quan | `enum ClassifierType`, `struct ScanResult`, `struct DetailedScanResult`, `struct PromptInjectionScanner` |
| [`src/engine/security/security_inspector.rs`](../../crates/custos-runtime/src/engine/security/security_inspector.rs) | 154 | Security inspector that uses pattern matching to detect malicious tool calls | `struct SecurityInspector`, `fn new`, `fn enabled`, `fn convert_security_result` |
| [`src/engine/session_context.rs`](../../crates/custos-runtime/src/engine/session_context.rs) | 186 | Local OS user running goose, shared by the OTLP `user.name` resource | `fn current_session_id`, `fn session_id_request_builder`, `fn session_id_request_builder_with_header_override`, `fn session_id_request_builder_with_header_name` |
| [`src/engine/skills/arguments.rs`](../../crates/custos-runtime/src/engine/skills/arguments.rs) | 244 | Module arguments: phục vụ các cấu trúc và chức năng liên quan | `fn is_resolvable`, `fn apply_skill_arguments`, `fn names`, `fn arguments_placeholder_is_replaced_with_raw_args` |
| [`src/engine/skills/builtin.rs`](../../crates/custos-runtime/src/engine/skills/builtin.rs) | 11 | Module builtin: phục vụ các cấu trúc và chức năng liên quan | `fn get_all` |
| [`src/engine/skills/builtins/goose_doc_guide.md`](../../crates/custos-runtime/src/engine/skills/builtins/goose_doc_guide.md) | 63 | Module goose_doc_guide: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/skills/builtins/web_search.md`](../../crates/custos-runtime/src/engine/skills/builtins/web_search.md) | 71 | Module web_search: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/skills/client.rs`](../../crates/custos-runtime/src/engine/skills/client.rs) | 686 | Controls whether Goose's bundled skills are exposed by this client. | `struct SkillsClient`, `fn new`, `fn with_builtin_skills`, `fn with_config` |
| [`src/engine/skills/mod.rs`](../../crates/custos-runtime/src/engine/skills/mod.rs) | 1222 | Canonical writable location for global user skills: `~/.agents/skills`. | `struct SkillFrontmatter`, `fn global_skills_dir`, `fn project_skills_dir`, `fn skills_dir_global_or_err` |
| [`src/engine/skills/supporting_files.rs`](../../crates/custos-runtime/src/engine/skills/supporting_files.rs) | 1064 | Module supporting_files: phục vụ các cấu trúc và chức năng liên quan | `enum ReadLimit`, `fn load_supporting_file`, `fn load_supporting_file_with_limit`, `fn read_supporting_file_with_limit` |
| [`src/engine/slash_commands/mod.rs`](../../crates/custos-runtime/src/engine/slash_commands/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/slash_commands/recipe_slash_command.rs`](../../crates/custos-runtime/src/engine/slash_commands/recipe_slash_command.rs) | 629 | Module recipe_slash_command: phục vụ các cấu trúc và chức năng liên quan | `struct SlashCommandMapping`, `fn list_commands`, `fn save_slash_commands`, `fn set_recipe_slash_command` |
| [`src/engine/slash_commands/skill_slash_command.rs`](../../crates/custos-runtime/src/engine/slash_commands/skill_slash_command.rs) | 163 | Module skill_slash_command: phục vụ các cấu trúc và chức năng liên quan | `fn list_commands`, `fn format_installed_skills`, `fn resolve_command`, `fn commands_from_sources` |
| [`src/engine/slash_commands/slash_command.rs`](../../crates/custos-runtime/src/engine/slash_commands/slash_command.rs) | 126 | Module slash_command: phục vụ các cấu trúc và chức năng liên quan | `fn list_builtin_commands`, `fn list_acp_commands`, `fn merge_command_sources`, `fn lists_acp_safe_builtin_commands` |
| [`src/engine/slash_commands/types.rs`](../../crates/custos-runtime/src/engine/slash_commands/types.rs) | 15 | Định nghĩa các kiểu dữ liệu dùng chung | `enum SlashCommandSource`, `struct SlashCommandEntry` |
| [`src/engine/slash_commands/util.rs`](../../crates/custos-runtime/src/engine/slash_commands/util.rs) | 3 | Module util: phục vụ các cấu trúc và chức năng liên quan | `fn normalize_command_name` |
| [`src/engine/source_roots.rs`](../../crates/custos-runtime/src/engine/source_roots.rs) | 16 | Module source_roots: phục vụ các cấu trúc và chức năng liên quan | `struct SourceRoot`, `fn read_only` |
| [`src/engine/sources.rs`](../../crates/custos-runtime/src/engine/sources.rs) | 2164 | Returns (display_name, description, body, properties). | `fn parse_frontmatter`, `fn require_mutable_type`, `fn require_listable_type`, `struct MarkdownSourceFrontmatter` |
| [`src/engine/subprocess.rs`](../../crates/custos-runtime/src/engine/subprocess.rs) | 144 | Creates a Git command that rejects implicit bare repositories and cannot run a | `fn configure_parent_death_signal`, `trait SubprocessExt`, `fn git_command`, `fn set_no_window` |
| [`src/engine/token_counter.rs`](../../crates/custos-runtime/src/engine/token_counter.rs) | 326 | Module token_counter: phục vụ các cấu trúc và chức năng liên quan | `struct TokenCounter`, `struct TokenCacheKey`, `fn from_text`, `fn count_tokens` |
| [`src/engine/tool_call_labels.rs`](../../crates/custos-runtime/src/engine/tool_call_labels.rs) | 881 | Module tool_call_labels: phục vụ các cấu trúc và chức năng liên quan | `fn prepare_tool_chain_steps`, `struct MockProvider`, `fn new`, `fn managing_own_context` |
| [`src/engine/tool_inspection.rs`](../../crates/custos-runtime/src/engine/tool_inspection.rs) | 313 | Result of inspecting a tool call | `struct InspectionResult`, `enum InspectionAction`, `trait ToolInspector`, `fn is_enabled` |
| [`src/engine/tool_monitor.rs`](../../crates/custos-runtime/src/engine/tool_monitor.rs) | 135 | Module tool_monitor: phục vụ các cấu trúc và chức năng liên quan | `struct InternalToolCall`, `fn matches`, `fn from_tool_call`, `struct RepetitionInspector` |
| [`src/engine/tracing/langfuse_layer.rs`](../../crates/custos-runtime/src/engine/tracing/langfuse_layer.rs) | 509 | Module langfuse_layer: phục vụ các cấu trúc và chức năng liên quan | `struct LangfuseIngestionResponse`, `struct LangfuseIngestionSuccess`, `struct LangfuseIngestionError`, `struct LangfuseBatchManager` |
| [`src/engine/tracing/mod.rs`](../../crates/custos-runtime/src/engine/tracing/mod.rs) | 11 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/engine/tracing/observation_layer.rs`](../../crates/custos-runtime/src/engine/tracing/observation_layer.rs) | 639 | Module observation_layer: phục vụ các cấu trúc và chức năng liên quan | `struct SpanData`, `fn map_level`, `fn flatten_metadata`, `trait BatchManager` |
| [`src/engine/tracing/rate_limiter.rs`](../../crates/custos-runtime/src/engine/tracing/rate_limiter.rs) | 143 | Module rate_limiter: phục vụ các cấu trúc và chức năng liên quan | `struct RateLimitedTelemetrySender`, `enum TelemetryEvent`, `struct SpanData`, `struct MetricData` |
| [`src/engine/utils.rs`](../../crates/custos-runtime/src/engine/utils.rs) | 234 | Encode bytes as a lowercase hexadecimal string. | `fn bytes_to_hex`, `fn is_in_unicode_tag_range`, `fn contains_unicode_tags`, `fn strip_unicode_tags` |
| [`src/gateway/budget/guard.rs`](../../crates/custos-runtime/src/gateway/budget/guard.rs) | 116 | Module guard: phục vụ các cấu trúc và chức năng liên quan | `struct BudgetCheck`, `struct BudgetLimits`, `fn default`, `struct BudgetGuard` |
| [`src/gateway/budget/mod.rs`](../../crates/custos-runtime/src/gateway/budget/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/gateway/budget/tracker.rs`](../../crates/custos-runtime/src/gateway/budget/tracker.rs) | 54 | Module tracker: phục vụ các cấu trúc và chức năng liên quan | `struct UsageRecord`, `struct BudgetTracker`, `fn new` |
| [`src/gateway/dispatch/a2a.rs`](../../crates/custos-runtime/src/gateway/dispatch/a2a.rs) | 24 | Module a2a: phục vụ các cấu trúc và chức năng liên quan | `struct A2ADispatcher` |
| [`src/gateway/dispatch/external.rs`](../../crates/custos-runtime/src/gateway/dispatch/external.rs) | 19 | Module external: phục vụ các cấu trúc và chức năng liên quan | `struct ExternalDispatcher` |
| [`src/gateway/dispatch/human.rs`](../../crates/custos-runtime/src/gateway/dispatch/human.rs) | 26 | Module human: phục vụ các cấu trúc và chức năng liên quan | `struct HumanDispatcher`, `fn new` |
| [`src/gateway/dispatch/local.rs`](../../crates/custos-runtime/src/gateway/dispatch/local.rs) | 32 | Module local: phục vụ các cấu trúc và chức năng liên quan | `struct LocalDispatcher` |
| [`src/gateway/dispatch/mod.rs`](../../crates/custos-runtime/src/gateway/dispatch/mod.rs) | 9 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/gateway/gateway.rs`](../../crates/custos-runtime/src/gateway/gateway.rs) | 308 | Module gateway: phục vụ các cấu trúc và chức năng liên quan | `struct RouteRequest`, `struct GatewayResponse`, `trait AgentGateway`, `struct CustosGateway` |
| [`src/gateway/mod.rs`](../../crates/custos-runtime/src/gateway/mod.rs) | 17 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/gateway/observability/metrics.rs`](../../crates/custos-runtime/src/gateway/observability/metrics.rs) | 43 | Module metrics: phục vụ các cấu trúc và chức năng liên quan | `struct GatewayMetrics`, `fn new`, `fn record_tier`, `fn record_human_gate` |
| [`src/gateway/observability/mod.rs`](../../crates/custos-runtime/src/gateway/observability/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/gateway/observability/tracer.rs`](../../crates/custos-runtime/src/gateway/observability/tracer.rs) | 64 | Module tracer: phục vụ các cấu trúc và chức năng liên quan | `struct GatewaySpan`, `struct GatewayTracer`, `fn new` |
| [`src/gateway/policy/mod.rs`](../../crates/custos-runtime/src/gateway/policy/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/gateway/policy/quota.rs`](../../crates/custos-runtime/src/gateway/policy/quota.rs) | 35 | Module quota: phục vụ các cấu trúc và chức năng liên quan | `struct SessionQuotaLimiter`, `fn new` |
| [`src/gateway/policy/rules.rs`](../../crates/custos-runtime/src/gateway/policy/rules.rs) | 21 | Module rules: phục vụ các cấu trúc và chức năng liên quan | `struct GatewayPolicyRule`, `fn default` |
| [`src/lib.rs`](../../crates/custos-runtime/src/lib.rs) | 29 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/memory_service/mod.rs`](../../crates/custos-runtime/src/memory_service/mod.rs) | 2 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/memory_service/traits.rs`](../../crates/custos-runtime/src/memory_service/traits.rs) | 17 | Module traits: phục vụ các cấu trúc và chức năng liên quan | `enum MemoryTier`, `trait MemoryStore` |
| [`src/session/journal.rs`](../../crates/custos-runtime/src/session/journal.rs) | 35 | Module journal: phục vụ các cấu trúc và chức năng liên quan | `struct SessionJournal`, `fn new`, `fn append`, `fn tool_call_count` |
| [`src/session/manager.rs`](../../crates/custos-runtime/src/session/manager.rs) | 321 | Creates an in-memory session manager without durable backing (for testing). | `struct SessionManager`, `fn default`, `fn new`, `fn with_store` |
| [`src/session/mod.rs`](../../crates/custos-runtime/src/session/mod.rs) | 214 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `struct MockSessionStore`, `fn new` |
| [`src/workflow/dispatcher.rs`](../../crates/custos-runtime/src/workflow/dispatcher.rs) | 15 | Module dispatcher: phục vụ các cấu trúc và chức năng liên quan | `struct WorkflowDispatcher`, `fn new`, `fn default` |
| [`src/workflow/lease.rs`](../../crates/custos-runtime/src/workflow/lease.rs) | 10 | Module lease: phục vụ các cấu trúc và chức năng liên quan | `struct TaskLease` |
| [`src/workflow/machine.rs`](../../crates/custos-runtime/src/workflow/machine.rs) | 397 | Module machine: phục vụ các cấu trúc và chức năng liên quan | `struct WorkflowState`, `fn default`, `enum OperationOutcome`, `trait WorkflowOperation` |
| [`src/workflow/mod.rs`](../../crates/custos-runtime/src/workflow/mod.rs) | 15 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/workflow/outbox.rs`](../../crates/custos-runtime/src/workflow/outbox.rs) | 9 | Module outbox: phục vụ các cấu trúc và chức năng liên quan | `struct OutboxMessage` |
| [`src/workflow/scheduler.rs`](../../crates/custos-runtime/src/workflow/scheduler.rs) | 250 | Schedule a new workflow job | `struct ScheduledJobId`, `fn new`, `fn generate`, `fn fmt` |

### 3.7. Crate `custos-adapters` — Layer 2: External Adapters & Sandbox

- **Đường dẫn thư mục:** `crates/custos-adapters`
- **Chủ sở hữu chính (Owner):** **Trường (Sandbox) & Vinh (MCP/Harnesses) & Vĩ (Providers)**
- **Quy tắc ranh giới:** Mọi dữ liệu ngoại vi đều là untrusted; cô lập an toàn trong sandbox.
- **Tổng số file:** 157 files | **Tổng số dòng mã:** 49,392 lines
- **Mô tả chức năng:** Bộ điều hợp thế giới thực: OS Sandbox (Seatbelt macOS, Bubblewrap Linux), MCP Client STDIO/SSE, inference cục bộ (llama.cpp/MLX) và external harnesses.

#### Danh mục các file bên trong `crates/custos-adapters/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-adapters/Cargo.toml) | 75 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/download_manager/mod.rs`](../../crates/custos-adapters/src/download_manager/mod.rs) | 690 | Remove orphaned `.part` files in the given directory (and one level of subdirectories). | `fn partial_path_for`, `fn cleanup_partial_downloads`, `struct DownloadProgress`, `enum DownloadStatus` |
| [`src/lib.rs`](../../crates/custos-adapters/src/lib.rs) | 37 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/local_inference/backend.rs`](../../crates/custos-adapters/src/local_inference/backend.rs) | 50 | Module backend: phục vụ các cấu trúc và chức năng liên quan | `trait BackendLoadedModel`, `struct LocalGenerationRequest`, `trait LocalInferenceBackend` |
| [`src/local_inference/config_resolver.rs`](../../crates/custos-adapters/src/local_inference/config_resolver.rs) | 60 | Module config_resolver: phục vụ các cấu trúc và chức năng liên quan | `type StringParamResolver`, `type BoolParamResolver`, `type ModelSettingsResolver`, `type ModelSettingsWriter` |
| [`src/local_inference/hf_models.rs`](../../crates/custos-adapters/src/local_inference/hf_models.rs) | 3082 | A single downloadable GGUF file (used internally and for downloads). | `struct HfModelInfo`, `struct HfModelVariant`, `fn default_supported`, `struct HfGgufFile` |
| [`src/local_inference/huggingface_auth.rs`](../../crates/custos-adapters/src/local_inference/huggingface_auth.rs) | 64 | Module huggingface_auth: phục vụ các cấu trúc và chức năng liên quan | `type TokenResolver`, `struct HuggingFaceTokenData`, `fn is_expired`, `fn oauth_cache_path` |
| [`src/local_inference/llamacpp/inference_emulated_tools.rs`](../../crates/custos-adapters/src/local_inference/llamacpp/inference_emulated_tools.rs) | 385 | Module inference_emulated_tools: phục vụ các cấu trúc và chức năng liên quan | `fn load_tiny_model_prompt`, `fn build_emulator_tool_description`, `fn send_emulator_action`, `fn generate_with_emulated_tools` |
| [`src/local_inference/llamacpp/inference_engine.rs`](../../crates/custos-adapters/src/local_inference/llamacpp/inference_engine.rs) | 688 | Estimate the maximum context length that can fit in available accelerator/CPU | `struct GenerationContext`, `struct LoadedModel`, `struct LoadedChatTemplates`, `struct PreparedGeneration` |
| [`src/local_inference/llamacpp/inference_native_tools.rs`](../../crates/custos-adapters/src/local_inference/llamacpp/inference_native_tools.rs) | 409 | Merge OpenAI streaming deltas by `index` into `MessageContent` items. | `fn generate_with_native_tools`, `fn extract_oai_tool_call_contents`, `fn get_content_tool_call_name`, `fn get_content_tool_call_args` |
| [`src/local_inference/llamacpp/mod.rs`](../../crates/custos-adapters/src/local_inference/llamacpp/mod.rs) | 783 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn builtin_chat_template_names`, `fn template_result_supports_native_tool_calling`, `fn supports_native_tool_calling`, `fn should_use_native_tool_calling` |
| [`src/local_inference/management.rs`](../../crates/custos-adapters/src/local_inference/management.rs) | 701 | Module management: phục vụ các cấu trúc và chức năng liên quan | `struct LocalModelSelection`, `fn download_progress`, `fn cancel_download`, `fn get_model_settings` |
| [`src/local_inference/mlx.rs`](../../crates/custos-adapters/src/local_inference/mlx.rs) | 1161 | Module mlx: phục vụ các cấu trúc và chức năng liên quan | `fn safetensors_shard`, `fn snapshot_files_are_complete`, `fn validate_snapshot_files`, `fn mlx_file_error` |
| [`src/local_inference/mod.rs`](../../crates/custos-adapters/src/local_inference/mod.rs) | 1109 | Remove `image_url` content parts from OpenAI-format messages JSON, replacing | `type ModelSlotHandle`, `struct ModelSlot`, `enum ModelSlotState`, `fn new` |
| [`src/local_inference/model.rs`](../../crates/custos-adapters/src/local_inference/model.rs) | 135 | Module model: phục vụ các cấu trúc và chức năng liên quan | `enum SamplingConfig`, `fn default`, `enum ToolCallingMode`, `enum ChatTemplate` |
| [`src/local_inference/multimodal.rs`](../../crates/custos-adapters/src/local_inference/multimodal.rs) | 336 | Walk the OpenAI-format messages JSON array. For each content part with | `struct ExtractedImage`, `struct MultimodalMessages`, `fn extract_images_from_messages_json`, `fn extract_images_from_messages` |
| [`src/local_inference/native_tool_parsing.rs`](../../crates/custos-adapters/src/local_inference/native_tool_parsing.rs) | 343 | Module native_tool_parsing: phục vụ các cấu trúc và chức năng liên quan | `fn message_from_native_tool_text`, `fn parse_openai_message_json`, `fn parse_tool_calls_json`, `fn is_tool_call_array` |
| [`src/local_inference/paths.rs`](../../crates/custos-adapters/src/local_inference/paths.rs) | 123 | Module paths: phục vụ các cấu trúc và chức năng liên quan | `struct Paths`, `fn get_dir`, `fn path_root`, `fn validated_path_root` |
| [`src/local_inference/prompt_template.rs`](../../crates/custos-adapters/src/local_inference/prompt_template.rs) | 43 | Module prompt_template: phục vụ các cấu trúc và chức năng liên quan | `fn render_string`, `fn render_template` |
| [`src/local_inference/prompts/tiny_model_system.md`](../../crates/custos-adapters/src/local_inference/prompts/tiny_model_system.md) | 22 | Module tiny_model_system: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/local_inference/provider_utils.rs`](../../crates/custos-adapters/src/local_inference/provider_utils.rs) | 24 | Module provider_utils: phục vụ các cấu trúc và chức năng liên quan | `fn filter_extensions_from_system_prompt` |
| [`src/local_inference/thinking_output.rs`](../../crates/custos-adapters/src/local_inference/thinking_output.rs) | 81 | Module thinking_output: phục vụ các cấu trúc và chức năng liên quan | `struct ThinkingOutputFilter`, `fn new`, `fn push_structured_reasoning`, `fn push_text` |
| [`src/local_inference/tool_emulation.rs`](../../crates/custos-adapters/src/local_inference/tool_emulation.rs) | 731 | Module tool_emulation: phục vụ các cấu trúc và chức năng liên quan | `fn load_tiny_model_prompt`, `fn build_emulator_tool_description`, `enum EmulatorAction`, `enum ParserState` |
| [`src/local_inference/tool_parsing.rs`](../../crates/custos-adapters/src/local_inference/tool_parsing.rs) | 52 | Module tool_parsing: phục vụ các cấu trúc và chức năng liên quan | `fn compact_tools_json` |
| [`src/mcp/adapters/client.rs`](../../crates/custos-adapters/src/mcp/adapters/client.rs) | 132 | CustosMcpClient manages communication with an external or embedded MCP server | `struct ToolCallContext`, `fn new`, `fn with_working_dir`, `struct CustosMcpHostInfo` |
| [`src/mcp/adapters/gateway_wrap.rs`](../../crates/custos-adapters/src/mcp/adapters/gateway_wrap.rs) | 227 | Fail-closed MCP/tool dispatch until a durable attempt claim exists; model prompt substring filtering remains a prototype, not comprehensive secret detection. | `struct GatewayTool`, `struct GatewayProvider`, `struct McpCapabilityAdapter` |
| [`src/mcp/adapters/mod.rs`](../../crates/custos-adapters/src/mcp/adapters/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/mod.rs`](../../crates/custos-adapters/src/mcp/core/autovisualiser/mod.rs) | 2265 | Build a Meta object with `_meta.ui.resourceUri` for linking a tool to a UI resource. | `fn ui_resource_meta`, `struct UIResourceDef`, `fn validate_data_param`, `fn validation_err` |
| [`src/mcp/core/autovisualiser/templates/assets/chart.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/chart.min.js) | 14 | Module chart.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/d3.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/d3.min.js) | 2 | Module d3.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/d3.sankey.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/d3.sankey.min.js) | 2 | Module d3.sankey.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/leaflet.markercluster.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/leaflet.markercluster.min.js) | 2 | Module leaflet.markercluster.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/leaflet.min.css`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/leaflet.min.css) | 661 | Module leaflet.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/leaflet.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/leaflet.min.js) | 6 | Module leaflet.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/mcp-app-base.css`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/mcp-app-base.css) | 103 | Module mcp-app-base: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/mcp-app-bridge.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/mcp-app-bridge.js) | 260 | Module mcp-app-bridge: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/assets/mermaid.min.js`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/assets/mermaid.min.js) | 2030 | Module mermaid.min: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/chart_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/chart_template.html) | 302 | Module chart_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/chord_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/chord_template.html) | 170 | Module chord_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/donut_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/donut_template.html) | 221 | Module donut_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/map_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/map_template.html) | 234 | Module map_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/mermaid_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/mermaid_template.html) | 166 | Module mermaid_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/radar_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/radar_template.html) | 193 | Module radar_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/sankey_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/sankey_template.html) | 249 | Module sankey_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/autovisualiser/templates/treemap_template.html`](../../crates/custos-adapters/src/mcp/core/autovisualiser/templates/treemap_template.html) | 171 | Module treemap_template: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/computercontroller/docx_tool.rs`](../../crates/custos-adapters/src/mcp/core/computercontroller/docx_tool.rs) | 791 | Định nghĩa ToolDefinition, ToolResult | `enum UpdateMode`, `struct DocxStyle`, `fn from_json`, `fn apply_to_run` |
| [`src/mcp/core/computercontroller/mod.rs`](../../crates/custos-adapters/src/mcp/core/computercontroller/mod.rs) | 888 | Parameters for the computer_control tool (macOS — Peekaboo CLI passthrough) | `struct ComputerControlParams`, `enum PdfOperation`, `struct PdfToolParams`, `enum DocxOperation` |
| [`src/mcp/core/computercontroller/pdf_tool.rs`](../../crates/custos-adapters/src/mcp/core/computercontroller/pdf_tool.rs) | 460 | Định nghĩa ToolDefinition, ToolResult | None |
| [`src/mcp/core/computercontroller/tests/data/FinancialSample.xlsx`](../../crates/custos-adapters/src/mcp/core/computercontroller/tests/data/FinancialSample.xlsx) | 2105 | Module FinancialSample: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/computercontroller/tests/data/sample.docx`](../../crates/custos-adapters/src/mcp/core/computercontroller/tests/data/sample.docx) | 535 | Module sample: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/computercontroller/tests/data/test.pdf`](../../crates/custos-adapters/src/mcp/core/computercontroller/tests/data/test.pdf) | 1997 | Module test: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/computercontroller/tests/data/test_image.pdf`](../../crates/custos-adapters/src/mcp/core/computercontroller/tests/data/test_image.pdf) | 749 | Module test_image: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/computercontroller/xlsx_tool.rs`](../../crates/custos-adapters/src/mcp/core/computercontroller/xlsx_tool.rs) | 495 | Định nghĩa ToolDefinition, ToolResult | `struct WorksheetInfo`, `struct CellValue`, `struct RangeData`, `struct XlsxTool` |
| [`src/mcp/core/mcp_server_runner.rs`](../../crates/custos-adapters/src/mcp/core/mcp_server_runner.rs) | 50 | Module mcp_server_runner: phục vụ các cấu trúc và chức năng liên quan | `enum McpCommand`, `type Err`, `fn from_str`, `fn name` |
| [`src/mcp/core/memory/mod.rs`](../../crates/custos-adapters/src/mcp/core/memory/mod.rs) | 851 | Parameters for the remember_memory tool | `fn is_reserved_windows_category`, `fn extract_working_dir_from_meta`, `fn memory_error`, `struct RememberMemoryParams` |
| [`src/mcp/core/mod.rs`](../../crates/custos-adapters/src/mcp/core/mod.rs) | 101 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `type SpawnServerFn`, `fn spawn_and_serve` |
| [`src/mcp/core/peekaboo/mod.rs`](../../crates/custos-adapters/src/mcp/core/peekaboo/mod.rs) | 85 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `fn is_peekaboo_installed`, `fn resolve_brew`, `fn auto_install_peekaboo` |
| [`src/mcp/core/subprocess.rs`](../../crates/custos-adapters/src/mcp/core/subprocess.rs) | 119 | Resolve the user's full PATH by running a login shell. | `trait SubprocessExt`, `fn set_no_window`, `fn set_no_window`, `fn resolve_login_shell_path` |
| [`src/mcp/core/tutorial/mod.rs`](../../crates/custos-adapters/src/mcp/core/tutorial/mod.rs) | 209 | Parameters for the load_tutorial tool | `struct LoadTutorialParams`, `struct TutorialServer`, `fn default`, `fn new` |
| [`src/mcp/core/tutorial/tutorials/build-mcp-extension.md`](../../crates/custos-adapters/src/mcp/core/tutorial/tutorials/build-mcp-extension.md) | 415 | Module build-mcp-extension: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/core/tutorial/tutorials/first-game.md`](../../crates/custos-adapters/src/mcp/core/tutorial/tutorials/first-game.md) | 178 | Module first-game: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/mcp/mod.rs`](../../crates/custos-adapters/src/mcp/mod.rs) | 5 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/antigravity/mod.rs`](../../crates/custos-adapters/src/providers/antigravity/mod.rs) | 37 | Unconfigured placeholder; fails closed instead of impersonating an Antigravity response. | `struct AntigravityProvider`, `fn new`, `fn default`, `fn provider_id` |
| [`src/providers/claude/mod.rs`](../../crates/custos-adapters/src/providers/claude/mod.rs) | 36 | Unconfigured placeholder; fails closed instead of impersonating a Claude response. | `struct ClaudeProvider`, `fn new`, `fn default`, `fn provider_id` |
| [`src/providers/codex/mod.rs`](../../crates/custos-adapters/src/providers/codex/mod.rs) | 36 | Unconfigured placeholder; fails closed instead of impersonating a Codex response. | `struct CodexProvider`, `fn new`, `fn default`, `fn provider_id` |
| [`src/providers/fake/mod.rs`](../../crates/custos-adapters/src/providers/fake/mod.rs) | 146 | A fake provider for testing, replay fixtures, and end-to-end simulation. | `struct FakeProvider`, `fn new`, `fn with_default_text`, `fn enqueue_response` |
| [`src/providers/local_model/mod.rs`](../../crates/custos-adapters/src/providers/local_model/mod.rs) | 45 | Unconfigured placeholder; real local inference uses separate backend integration. Fails closed instead of fabricating output. | `struct LocalModelProvider`, `fn new`, `fn with_model`, `fn default` |
| [`src/providers/mod.rs`](../../crates/custos-adapters/src/providers/mod.rs) | 38 | Provider exports and regression test that unconfigured adapters do not fabricate model output. | `mod placeholder_tests` |
| [`src/providers/providers/anthropic.rs`](../../crates/custos-adapters/src/providers/providers/anthropic.rs) | 856 | Builder for [`AnthropicProvider`]. | `struct AnthropicProvider`, `struct AnthropicProviderBuilder`, `fn new`, `fn api_client` |
| [`src/providers/providers/api_client.rs`](../../crates/custos-adapters/src/providers/providers/api_client.rs) | 1092 | Configure TLS settings on a reqwest ClientBuilder | `type RequestBuilderDecorator`, `struct ApiClient`, `enum TransportPolicy`, `enum AuthMethod` |
| [`src/providers/providers/azure_foundry.rs`](../../crates/custos-adapters/src/providers/providers/azure_foundry.rs) | 1395 | Module azure_foundry: phục vụ các cấu trúc và chức năng liên quan | `enum EndpointKind`, `fn endpoint_kind`, `fn is_project_endpoint`, `enum ModelPublisher` |
| [`src/providers/providers/browser_live_transport.rs`](../../crates/custos-adapters/src/providers/providers/browser_live_transport.rs) | 103 | Trait cốt lõi ProviderPort, ModelProvider, AgentRuntimePort | `struct BrowserLiveOutbound`, `struct BrowserLiveTransport`, `fn new`, `fn default` |
| [`src/providers/providers/databricks.rs`](../../crates/custos-adapters/src/providers/providers/databricks.rs) | 1118 | Module databricks: phục vụ các cấu trúc và chức năng liên quan | `struct DatabricksEndpointInfo`, `struct DatabricksUpstreamModel`, `struct CachedDatabricksEndpointInfo`, `enum EndpointMetadataLookup` |
| [`src/providers/providers/databricks_auth.rs`](../../crates/custos-adapters/src/providers/providers/databricks_auth.rs) | 93 | Module databricks_auth: phục vụ các cấu trúc và chức năng liên quan | `type DatabricksOauthTokenFuture`, `type DatabricksOauthTokenProvider`, `type DatabricksTokenResolver`, `type DatabricksRefreshHook` |
| [`src/providers/providers/databricks_v2.rs`](../../crates/custos-adapters/src/providers/providers/databricks_v2.rs) | 1282 | Routes requests through a gateway deployment served under a different | `struct ModelCatalog`, `enum DatabricksV2Route`, `struct DatabricksV2Provider`, `fn new` |
| [`src/providers/providers/decision.rs`](../../crates/custos-adapters/src/providers/providers/decision.rs) | 87 | Module decision: phục vụ các cấu trúc và chức năng liên quan | `struct DecisionRequest`, `enum DecisionQuestion`, `struct NoulCriteria`, `struct DecisionResponse` |
| [`src/providers/providers/declarative.rs`](../../crates/custos-adapters/src/providers/providers/declarative.rs) | 681 | Deserialize an optional string, treating empty/whitespace-only values as None. | `fn fixed_provider_configs`, `fn fixed_provider_config_entries`, `struct EnvVarConfig`, `enum ProviderEngine` |
| [`src/providers/providers/declarative/definitions/aimlapi.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/aimlapi.json) | 57 | Module aimlapi: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/alibaba.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/alibaba.json) | 60 | Module alibaba: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/atomic_chat.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/atomic_chat.json) | 35 | Module atomic_chat: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/celeris.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/celeris.json) | 20 | Module celeris: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/cerebras.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/cerebras.json) | 41 | Module cerebras: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/deepseek.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/deepseek.json) | 44 | Module deepseek: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/empiriolabs.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/empiriolabs.json) | 53 | Module empiriolabs: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/eurouter.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/eurouter.json) | 118 | Module eurouter: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/fireworks.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/fireworks.json) | 55 | Module fireworks: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/friendli.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/friendli.json) | 21 | Module friendli: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/futurmix.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/futurmix.json) | 36 | Module futurmix: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/groq.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/groq.json) | 68 | Module groq: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/iflytek.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/iflytek.json) | 22 | Module iflytek: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/iflytek_astron.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/iflytek_astron.json) | 41 | Module iflytek_astron: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/inception.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/inception.json) | 15 | Module inception: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/llama_swap.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/llama_swap.json) | 23 | Module llama_swap: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/lmstudio.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/lmstudio.json) | 34 | Module lmstudio: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/lynkr.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/lynkr.json) | 24 | Module lynkr: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/meta.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/meta.json) | 30 | Module meta: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/minimax.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/minimax.json) | 32 | Module minimax: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/mistral.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/mistral.json) | 61 | Module mistral: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/moonshot.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/moonshot.json) | 31 | Module moonshot: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/nearai.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/nearai.json) | 48 | Module nearai: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/novita.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/novita.json) | 43 | Module novita: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/nvidia.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/nvidia.json) | 31 | Module nvidia: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/ollama_cloud.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/ollama_cloud.json) | 13 | Module ollama_cloud: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/omlx.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/omlx.json) | 23 | Module omlx: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/opencode_go.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/opencode_go.json) | 30 | Module opencode_go: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/opencode_zen.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/opencode_zen.json) | 28 | Module opencode_zen: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/opper.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/opper.json) | 30 | Module opper: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/orcarouter.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/orcarouter.json) | 31 | Module orcarouter: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/ovhcloud.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/ovhcloud.json) | 144 | Module ovhcloud: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/perplexity.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/perplexity.json) | 21 | Module perplexity: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/pleumrouter.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/pleumrouter.json) | 30 | Module pleumrouter: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/routstr.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/routstr.json) | 35 | Module routstr: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/sakana.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/sakana.json) | 31 | Module sakana: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/saladcloud.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/saladcloud.json) | 47 | Module saladcloud: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/saygm.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/saygm.json) | 22 | Module saygm: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/scaleway.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/scaleway.json) | 109 | Module scaleway: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/tanzu.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/tanzu.json) | 29 | Module tanzu: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/tensorix.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/tensorix.json) | 54 | Module tensorix: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/together.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/together.json) | 49 | Module together: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/trustedrouter.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/trustedrouter.json) | 52 | Module trustedrouter: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/venice.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/venice.json) | 25 | Module venice: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/vercel_ai_gateway.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/vercel_ai_gateway.json) | 169 | Module vercel_ai_gateway: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/zai.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/zai.json) | 44 | Module zai: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/zai_coding_plan.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/zai_coding_plan.json) | 31 | Module zai_coding_plan: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/definitions/zhipu.json`](../../crates/custos-adapters/src/providers/providers/declarative/definitions/zhipu.json) | 30 | Module zhipu: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/declarative/macros.rs`](../../crates/custos-adapters/src/providers/providers/declarative/macros.rs) | 44 | Module macros: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/google.rs`](../../crates/custos-adapters/src/providers/providers/google.rs) | 215 | Module google: phục vụ các cấu trúc và chức năng liên quan | `struct GoogleProvider`, `fn new`, `fn metadata`, `fn get_name` |
| [`src/providers/providers/http_status.rs`](../../crates/custos-adapters/src/providers/providers/http_status.rs) | 836 | Strip credentials and sensitive query parameters from a URL for safe | `fn sanitize_url`, `fn extract_retry_after`, `fn duration_from_finite_secs`, `fn parse_retry_after_header` |
| [`src/providers/providers/live.rs`](../../crates/custos-adapters/src/providers/providers/live.rs) | 298 | Module live: phục vụ các cấu trúc và chức năng liên quan | `trait LiveTransport`, `trait LiveProtocol`, `enum LiveSessionEndReason`, `struct LiveSessionOutcome` |
| [`src/providers/providers/live_transport_websocket.rs`](../../crates/custos-adapters/src/providers/providers/live_transport_websocket.rs) | 86 | Module live_transport_websocket: phục vụ các cấu trúc và chức năng liên quan | `type Socket`, `type SocketSink`, `type SocketStream`, `struct WebSocketLiveTransport` |
| [`src/providers/providers/live_voice_provider.rs`](../../crates/custos-adapters/src/providers/providers/live_voice_provider.rs) | 104 | Module live_voice_provider: phục vụ các cấu trúc và chức năng liên quan | `struct WebRtcOffer`, `fn new`, `fn into_sdp`, `struct WebRtcAnswer` |
| [`src/providers/providers/mod.rs`](../../crates/custos-adapters/src/providers/providers/mod.rs) | 39 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/providers/providers/ollama.rs`](../../crates/custos-adapters/src/providers/providers/ollama.rs) | 780 | Provider settings resolved from `config::Config` at construction time. | `struct OllamaOptions`, `fn default`, `struct OllamaProvider`, `struct OllamaProviderBuilder` |
| [`src/providers/providers/openai.rs`](../../crates/custos-adapters/src/providers/providers/openai.rs) | 1937 | Ensure a base URL has an explicit scheme. | `enum CachedContextLimit`, `fn value`, `type OpenAiBaseUrlParts`, `fn ensure_url_scheme` |
| [`src/providers/providers/openai_compatible.rs`](../../crates/custos-adapters/src/providers/providers/openai_compatible.rs) | 455 | Module openai_compatible: phục vụ các cấu trúc và chức năng liên quan | `struct OpenAiCompatibleProvider`, `fn new`, `fn with_supports_streaming`, `fn build_request_for_model` |
| [`src/providers/providers/openai_live.rs`](../../crates/custos-adapters/src/providers/providers/openai_live.rs) | 1265 | Module openai_live: phục vụ các cấu trúc và chức năng liên quan | `struct OpenAiLiveSessionConfig`, `struct OpenAiLiveMessage`, `enum OpenAiLiveMessageRole`, `enum OpenAiLiveContextChannel` |
| [`src/providers/providers/openai_live_voice_provider.rs`](../../crates/custos-adapters/src/providers/providers/openai_live_voice_provider.rs) | 425 | Module openai_live_voice_provider: phục vụ các cấu trúc và chức năng liên quan | `struct OpenAiLiveVoiceProvider`, `fn new`, `struct OpenAiProviderConnection`, `enum AppendContextOutcome` |
| [`src/providers/providers/openrouter.rs`](../../crates/custos-adapters/src/providers/providers/openrouter.rs) | 1387 | Spans of the literal `$ref` token inside opaque tool text. | `type OpenRouterSessionIdProvider`, `struct OpenRouterProvider`, `fn new`, `fn is_mandatory_reasoning_error` |
| [`src/providers/providers/openrouter_format.rs`](../../crates/custos-adapters/src/providers/providers/openrouter_format.rs) | 374 | Returns true when a reasoning disable request was inserted, which | `fn has_assistant_content`, `fn extract_reasoning_details`, `fn get_reasoning_details`, `fn response_to_message` |
| [`src/providers/providers/snowflake.rs`](../../crates/custos-adapters/src/providers/providers/snowflake.rs) | 397 | Module snowflake: phục vụ các cấu trúc và chức năng liên quan | `enum SnowflakeAuth`, `fn token`, `struct SnowflakeProvider`, `fn new` |
| [`src/providers/providers/typesafe.rs`](../../crates/custos-adapters/src/providers/providers/typesafe.rs) | 88 | Module typesafe: phục vụ các cấu trúc và chức năng liên quan | `struct TypeSafeProvider`, `fn new` |
| [`src/roaming/a2a.rs`](../../crates/custos-adapters/src/roaming/a2a.rs) | 167 | List active, trusted peer candidates | `struct A2ACapability`, `struct A2ADelegationRequest`, `enum A2ADelegationStatus`, `struct A2ADelegationResult` |
| [`src/roaming/card.rs`](../../crates/custos-adapters/src/roaming/card.rs) | 247 | A shareable, non-secret identity-plus-reachability card for a node. | `struct ConnectionCard`, `fn new`, `fn fingerprint`, `fn encode` |
| [`src/roaming/directory.rs`](../../crates/custos-adapters/src/roaming/directory.rs) | 268 | Which way a connection was established. | `enum Direction`, `struct PeerEntry`, `struct Directory`, `fn new` |
| [`src/roaming/error.rs`](../../crates/custos-adapters/src/roaming/error.rs) | 22 | Errors produced by the roaming subsystem. | `enum RoamingError` |
| [`src/roaming/frame.rs`](../../crates/custos-adapters/src/roaming/frame.rs) | 79 | Write a `u32`-length-prefixed frame. | None |
| [`src/roaming/handshake.rs`](../../crates/custos-adapters/src/roaming/handshake.rs) | 43 | First message a connecting client sends. Carries no credential — the | `struct ClientHello`, `enum HostAck`, `fn new` |
| [`src/roaming/identity.rs`](../../crates/custos-adapters/src/roaming/identity.rs) | 208 | A roaming node's long-lived identity. | `struct RoamingIdentity`, `fn from_secret`, `fn generate`, `fn load_or_create` |
| [`src/roaming/mod.rs`](../../crates/custos-adapters/src/roaming/mod.rs) | 56 | Parse an [`EndpointId`] (a peer's public key) from its string form. | `fn parse_endpoint_id` |
| [`src/roaming/node.rs`](../../crates/custos-adapters/src/roaming/node.rs) | 688 | Serves an accepted, authorized ACP byte stream. Implemented by the | `trait AcpStreamServer`, `struct RoamingConfig`, `fn new`, `fn with_relay` |
| [`src/roaming/peerbook.rs`](../../crates/custos-adapters/src/roaming/peerbook.rs) | 241 | A single saved remote node. | `struct PeerRecord`, `struct PeerBook`, `fn load`, `fn save` |
| [`src/roaming/relay.rs`](../../crates/custos-adapters/src/roaming/relay.rs) | 139 | How the roaming endpoint reaches relays. | `enum RelaySettings`, `struct RelayEntry`, `fn new`, `fn with_auth` |
| [`src/roaming/trust.rs`](../../crates/custos-adapters/src/roaming/trust.rs) | 232 | Persisted trust state: the inbound allowlist plus revocations. | `struct TrustBook`, `fn new`, `fn accept`, `fn revoke_key` |
| [`src/sandbox/bubblewrap/mod.rs`](../../crates/custos-adapters/src/sandbox/bubblewrap/mod.rs) | 13 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `struct BubblewrapSandbox`, `fn new`, `fn default` |
| [`src/sandbox/developer.rs`](../../crates/custos-adapters/src/sandbox/developer.rs) | 403 | Validates that a requested file path does not escape the workspace root. | `struct SovereignDeveloperAdapter`, `fn new`, `fn with_timeout`, `fn canonicalize_safe_path` |
| [`src/sandbox/mod.rs`](../../crates/custos-adapters/src/sandbox/mod.rs) | 7 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/sandbox/seatbelt/mod.rs`](../../crates/custos-adapters/src/sandbox/seatbelt/mod.rs) | 13 | Module mod: phục vụ các cấu trúc và chức năng liên quan | `struct SeatbeltSandbox`, `fn new`, `fn default` |

### 3.8. Crate `custos-packs` — Layer 3: Domain Workflows

- **Đường dẫn thư mục:** `crates/custos-packs`
- **Chủ sở hữu chính (Owner):** **Vĩ (AI/DS Lead) & Vinh**
- **Quy tắc ranh giới:** Khai báo workflow chuyên biệt: Engineering, Research, Assistant.
- **Tổng số file:** 28 files | **Tổng số dòng mã:** 3,318 lines
- **Mô tả chức năng:** Các gói kỹ năng miền: Phân tích mã nguồn AST, chuyển đổi định dạng phiên làm việc (Claude Code, Codex, Pi), và tự động hóa văn phòng.

#### Danh mục các file bên trong `crates/custos-packs/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-packs/Cargo.toml) | 22 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`declarative/engineering/capabilities/coding-plan.v1.json`](../../crates/custos-packs/declarative/engineering/capabilities/coding-plan.v1.json) | 19 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/context-recipes/triage.v1.yaml`](../../crates/custos-packs/declarative/engineering/context-recipes/triage.v1.yaml) | 14 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/pack.yaml`](../../crates/custos-packs/declarative/engineering/pack.yaml) | 23 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/policies/read-only.v1.yaml`](../../crates/custos-packs/declarative/engineering/policies/read-only.v1.yaml) | 12 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/prompts/coding-plan.v1.md`](../../crates/custos-packs/declarative/engineering/prompts/coding-plan.v1.md) | 5 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/prompts/diagnose.v1.md`](../../crates/custos-packs/declarative/engineering/prompts/diagnose.v1.md) | 9 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/prompts/implement.v1.md`](../../crates/custos-packs/declarative/engineering/prompts/implement.v1.md) | 9 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/prompts/plan.v1.md`](../../crates/custos-packs/declarative/engineering/prompts/plan.v1.md) | 9 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/tasks/bug-fix.yaml`](../../crates/custos-packs/declarative/engineering/tasks/bug-fix.yaml) | 20 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/tasks/repo-explain.yaml`](../../crates/custos-packs/declarative/engineering/tasks/repo-explain.yaml) | 16 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/verifiers/rust.v1.yaml`](../../crates/custos-packs/declarative/engineering/verifiers/rust.v1.yaml) | 16 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/workflows/bug-fix.v1.yaml`](../../crates/custos-packs/declarative/engineering/workflows/bug-fix.v1.yaml) | 21 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/workflows/coding-change.v1.json`](../../crates/custos-packs/declarative/engineering/workflows/coding-change.v1.json) | 41 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`declarative/engineering/workflows/repo-explain.v1.yaml`](../../crates/custos-packs/declarative/engineering/workflows/repo-explain.v1.yaml) | 15 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | None |
| [`src/assistant/mod.rs`](../../crates/custos-packs/src/assistant/mod.rs) | 54 | Gói công việc Trợ lý: tự động hóa văn phòng và nhắc việc | `struct AssistantPackDescriptor`, `fn get_descriptor`, `struct AssistantPack`, `fn manifest` |
| [`src/assistant/sdk.rs`](../../crates/custos-packs/src/assistant/sdk.rs) | 16 | Gói công việc Trợ lý: tự động hóa văn phòng và nhắc việc | `struct DomainPackManifest`, `trait DomainPack` |
| [`src/engineering/mod.rs`](../../crates/custos-packs/src/engineering/mod.rs) | 58 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | `struct EngineeringPackDescriptor`, `fn get_descriptor`, `struct EngineeringPack`, `fn manifest` |
| [`src/engineering/sdk.rs`](../../crates/custos-packs/src/engineering/sdk.rs) | 16 | Gói công việc Kỹ thuật phần mềm: duyệt git, phân tích AST, build | `struct DomainPackManifest`, `trait DomainPack` |
| [`src/engineering/skills.rs`](../../crates/custos-packs/src/engineering/skills.rs) | 140 | Skill: Search codebase using AST / Nexus call graphs | `struct SkillParameter`, `struct SkillMetadata`, `struct SkillExecutionResult`, `trait EngineeringSkill` |
| [`src/lib.rs`](../../crates/custos-packs/src/lib.rs) | 10 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/research/import_formats/claude_code.rs`](../../crates/custos-packs/src/research/import_formats/claude_code.rs) | 410 | Gói công việc Nghiên cứu: tổng hợp tài liệu, nhập phiên Claude/Codex/Pi | `fn convert`, `fn convert_user_message`, `fn convert_assistant_message`, `fn build_tool_result` |
| [`src/research/import_formats/codex.rs`](../../crates/custos-packs/src/research/import_formats/codex.rs) | 382 | Heuristic: Codex's first "user" message is often a giant | `fn convert`, `fn collect_user_text`, `fn is_context_blob`, `fn skips_developer_and_system_messages` |
| [`src/research/import_formats/mod.rs`](../../crates/custos-packs/src/research/import_formats/mod.rs) | 223 | Session-level fields harvested from a foreign transcript, used to build | `struct ImportedSession`, `fn build_session_json`, `enum ImportFormat`, `fn detect_format` |
| [`src/research/import_formats/pi.rs`](../../crates/custos-packs/src/research/import_formats/pi.rs) | 448 | Gói công việc Nghiên cứu: tổng hợp tài liệu, nhập phiên Claude/Codex/Pi | `fn convert`, `fn apply_user_content`, `fn apply_assistant_content`, `fn sanitize_json_strings` |
| [`src/research/markdown_export/mod.rs`](../../crates/custos-packs/src/research/markdown_export/mod.rs) | 1240 | Gói công việc Nghiên cứu: tổng hợp tài liệu, nhập phiên Claude/Codex/Pi | `fn value_to_simple_markdown_string`, `fn value_to_markdown`, `fn is_shell_tool_name`, `fn is_developer_file_tool_name` |
| [`src/research/mod.rs`](../../crates/custos-packs/src/research/mod.rs) | 54 | Gói công việc Nghiên cứu: tổng hợp tài liệu, nhập phiên Claude/Codex/Pi | `struct ResearchPackDescriptor`, `fn get_descriptor`, `struct ResearchPack`, `fn manifest` |
| [`src/research/sdk.rs`](../../crates/custos-packs/src/research/sdk.rs) | 16 | Gói công việc Nghiên cứu: tổng hợp tài liệu, nhập phiên Claude/Codex/Pi | `struct DomainPackManifest`, `trait DomainPack` |

### 3.9. Crate `custos-daemon` — Layer 4: Sole Composition Root

- **Đường dẫn thư mục:** `crates/custos-daemon`
- **Chủ sở hữu chính (Owner):** **Vĩ (Chief Architect)**
- **Quy tắc ranh giới:** File main.rs duy nhất được phép ráp nối storage, runtime, adapters.
- **Tổng số file:** 7 files | **Tổng số dòng mã:** 1,760 lines
- **Mô tả chức năng:** Tiến trình dịch vụ chạy ngầm của Custos: Lắng nghe Unix Domain Socket / TCP loopback, khởi tạo SQLite pool, điều phối Local API dispatcher và bảo đảm tính bền vững.

#### Danh mục các file bên trong `crates/custos-daemon/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`Cargo.toml`](../../crates/custos-daemon/Cargo.toml) | 39 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/api.rs`](../../crates/custos-daemon/src/api.rs) | 714 | Local API Dispatcher wrapping TaskService, SessionManager, and BridgeService for IPC callers. | `struct LocalApiDispatcher`, `fn new`, `struct MockStore` |
| [`src/lib.rs`](../../crates/custos-daemon/src/lib.rs) | 16 | Module lib: export api, profile, runtime, và local_api | None |
| [`src/local_api/lib.rs`](../../crates/custos-daemon/src/local_api/lib.rs) | 680 | Abstract transport for communicating with the Custos Daemon | `struct ApiRequest`, `struct ApiResponse`, `struct TcpTransport`, `struct LocalApiClient` |
| [`src/main.rs`](../../crates/custos-daemon/src/main.rs) | 108 | Điểm khởi đầu thực thi duy nhất của daemon (Composition Root, Singleton Lock, TCP/stdio server) | None |
| [`src/profile.rs`](../../crates/custos-daemon/src/profile.rs) | 291 | Profile directory resolver, singleton file lock (`daemon.lock`), và auto-connect helper | `struct ProfileResolver`, `struct DaemonLock`, `fn ensure_daemon_client` |
| [`src/runtime.rs`](../../crates/custos-daemon/src/runtime.rs) | 48 | Khởi tạo và cấu hình runtime ngầm | `struct CustosRuntime`, `fn bootstrap`, `fn bootstrap_profile` |


### 3.10. Crate `custos-sdk` — Layer 4: Client Bindings

- **Đường dẫn thư mục:** `crates/custos-sdk`
- **Chủ sở hữu chính (Owner):** **Vinh (Client Lead)**
- **Quy tắc ranh giới:** Thư viện DTOs client nhẹ; không mang phụ thuộc daemon hay database.
- **Tổng số file:** 39 files | **Tổng số dòng mã:** 7,366 lines
- **Mô tả chức năng:** Bộ công cụ phát triển phần mềm cho client: Định nghĩa wire DTOs, UniFFI bindings cho Python và Kotlin/Java, cùng hooks quan sát dữ liệu.

#### Danh mục các file bên trong `crates/custos-sdk/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`.gitignore`](../../crates/custos-sdk/.gitignore) | 6 | Module .gitignore: phục vụ các cấu trúc và chức năng liên quan | None |
| [`CHANGELOG.md`](../../crates/custos-sdk/CHANGELOG.md) | 33 | Module CHANGELOG: phục vụ các cấu trúc và chức năng liên quan | None |
| [`Cargo.toml`](../../crates/custos-sdk/Cargo.toml) | 52 | Module Cargo: phục vụ các cấu trúc và chức năng liên quan | None |
| [`README.md`](../../crates/custos-sdk/README.md) | 96 | Module README: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/acp_client.rs`](../../crates/custos-sdk/examples/acp_client.rs) | 160 | Module acp_client: phục vụ các cấu trúc và chức năng liên quan | `fn parse_args` |
| [`examples/deepseek.json`](../../crates/custos-sdk/examples/deepseek.json) | 30 | Module deepseek: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/README.md`](../../crates/custos-sdk/examples/uniffi/README.md) | 58 | Module README: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/kotlin/README.md`](../../crates/custos-sdk/examples/uniffi/kotlin/README.md) | 31 | Module README: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/kotlin/build.gradle.kts`](../../crates/custos-sdk/examples/uniffi/kotlin/build.gradle.kts) | 34 | Module build.gradle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/kotlin/settings.gradle.kts`](../../crates/custos-sdk/examples/uniffi/kotlin/settings.gradle.kts) | 16 | Module settings.gradle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/kotlin/src/main/kotlin/Main.kt`](../../crates/custos-sdk/examples/uniffi/kotlin/src/main/kotlin/Main.kt) | 47 | Module Main: phục vụ các cấu trúc và chức năng liên quan | None |
| [`examples/uniffi/provider.py`](../../crates/custos-sdk/examples/uniffi/provider.py) | 47 | Module provider: phục vụ các cấu trúc và chức năng liên quan | None |
| [`justfile`](../../crates/custos-sdk/justfile) | 168 | Module justfile: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/README.md`](../../crates/custos-sdk/maven/README.md) | 32 | Module README: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/build.gradle.kts`](../../crates/custos-sdk/maven/build.gradle.kts) | 82 | Module build.gradle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/gradlew`](../../crates/custos-sdk/maven/gradlew) | 10 | Module gradlew: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/settings.gradle.kts`](../../crates/custos-sdk/maven/settings.gradle.kts) | 15 | Module settings.gradle: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/NativeLibraryLoader.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/NativeLibraryLoader.kt) | 45 | Module NativeLibraryLoader: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/ProviderFlow.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/ProviderFlow.kt) | 22 | Module ProviderFlow: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/providers/anthropic/Anthropic.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/providers/anthropic/Anthropic.kt) | 9 | Module Anthropic: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/providers/databricks/Databricks.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/providers/databricks/Databricks.kt) | 6 | Module Databricks: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/providers/groq/Groq.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/providers/groq/Groq.kt) | 5 | Module Groq: phục vụ các cấu trúc và chức năng liên quan | None |
| [`maven/src/support/kotlin/io/github/aaif_goose/providers/openai/OpenAi.kt`](../../crates/custos-sdk/maven/src/support/kotlin/io/github/aaif_goose/providers/openai/OpenAi.kt) | 8 | Module OpenAi: phục vụ các cấu trúc và chức năng liên quan | None |
| [`python/README.md`](../../crates/custos-sdk/python/README.md) | 15 | Module README: phục vụ các cấu trúc và chức năng liên quan | None |
| [`python/pyproject.toml`](../../crates/custos-sdk/python/pyproject.toml) | 34 | Module pyproject: phục vụ các cấu trúc và chức năng liên quan | None |
| [`python/setup.py`](../../crates/custos-sdk/python/setup.py) | 20 | Module setup: phục vụ các cấu trúc và chức năng liên quan | None |
| [`scripts/gdk-release.py`](../../crates/custos-sdk/scripts/gdk-release.py) | 156 | Module gdk-release: phục vụ các cấu trúc và chức năng liên quan | None |
| [`scripts/maven-resource-prefix.sh`](../../crates/custos-sdk/scripts/maven-resource-prefix.sh) | 11 | Module maven-resource-prefix: phục vụ các cấu trúc và chức năng liên quan | None |
| [`scripts/prepare-maven-package.sh`](../../crates/custos-sdk/scripts/prepare-maven-package.sh) | 57 | Module prepare-maven-package: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/bin/uniffi-bindgen.rs`](../../crates/custos-sdk/src/bin/uniffi-bindgen.rs) | 3 | Module uniffi-bindgen: phục vụ các cấu trúc và chức năng liên quan | `fn main` |
| [`src/bindings.rs`](../../crates/custos-sdk/src/bindings.rs) | 2259 | Receives provider request logs as JSONL records. | `enum GooseError`, `fn generic`, `fn from`, `fn from` |
| [`src/lib.rs`](../../crates/custos-sdk/src/lib.rs) | 16 | Module lib: phục vụ các cấu trúc và chức năng liên quan | None |
| [`src/observability.rs`](../../crates/custos-sdk/src/observability.rs) | 520 | Receives structured provider request lifecycle events. | `trait ObservabilityHook`, `enum RequestOperation`, `struct RequestPayload`, `struct RequestStartEvent` |
| [`src/wire_types/custom_notifications.rs`](../../crates/custos-sdk/src/wire_types/custom_notifications.rs) | 285 | Goose-custom session update notification — a parallel to ACP's | `struct GooseSessionNotification`, `enum GooseSessionUpdate`, `struct LiveVoiceInteractionEndedUpdate`, `enum LiveVoiceInteractionOutcome` |
| [`src/wire_types/custom_requests.rs`](../../crates/custos-sdk/src/wire_types/custom_requests.rs) | 2374 | Schema descriptor for a single custom method, produced by the | `struct CustomMethodSchema`, `struct AddSessionExtensionRequest`, `struct RemoveSessionExtensionRequest`, `struct GetToolsRequest` |
| [`src/wire_types/custom_requests/recipe.rs`](../../crates/custos-sdk/src/wire_types/custom_requests/recipe.rs) | 416 | Ask the client to provide values for a recipe's parameters. | `fn default_recipe_version`, `struct RecipeDto`, `fn default`, `struct RecipeAuthorDto` |
| [`src/wire_types/custom_requests/schedule.rs`](../../crates/custos-sdk/src/wire_types/custom_requests/schedule.rs) | 179 | Delete a scheduled recipe job. | `struct ScheduledJobDto`, `struct ListSchedulesRequest`, `struct ListSchedulesResponse`, `struct CreateScheduleRequest` |
| [`src/wire_types/mod.rs`](../../crates/custos-sdk/src/wire_types/mod.rs) | 7 | Module mod: phục vụ các cấu trúc và chức năng liên quan | None |
| [`uniffi.toml`](../../crates/custos-sdk/uniffi.toml) | 2 | Module uniffi: phục vụ các cấu trúc và chức năng liên quan | None |

### 3.11. Crate `custos-app/cli` & `custos-app/desktop` — Layer 4: Thin Presentation Clients

- **Đường dẫn thư mục:** `crates/custos-app/cli` & `crates/custos-app/desktop`
- **Chủ sở hữu chính (Owner):** **Vinh (Client Lead)**
- **Quy tắc ranh giới:** Giao tiếp với Daemon thuần túy qua IPC socket; cấm truy cập SQLite trực tiếp. Không dính GUI Tauri dependency vào CLI binary.
- **Tổng số file:** 13 files | **Tổng số dòng mã:** ~2,670 lines
- **Mô tả chức năng:** Các ứng dụng giao diện trình diễn mỏng (Thin Presentation Clients): Render tiến độ tác vụ bằng ratatui (CLI) và Tauri Webview (Desktop), bảng hiển thị mã màu diff thay đổi, bảng điều khiển trạng thái và xác nhận cấp quyền.
- **Hợp đồng tương tác CLI:** phiên mặc định mang nhãn `custos`; `/` mở palette không cần Enter; `/mode` mở tầng mode gồm `/code`, `/research`, `/assistant`; prompt phản ánh mode bằng nhãn `custos-<mode>`.

#### Danh mục các file bên trong `crates/custos-app/cli/` & `crates/custos-app/desktop/`:

| Tập tin | Số dòng | Vai trò & Trách nhiệm kiến trúc | Các Struct / Trait / Hàm cốt lõi |
|---|:---:|---|---|
| [`cli/Cargo.toml`](../../crates/custos-app/cli/Cargo.toml) | 35 | Module Cargo CLI: phục vụ biên dịch executable `custos-cli` siêu nhẹ | None |
| [`cli/src/lib.rs`](../../crates/custos-app/cli/src/lib.rs) | 667 | Module lib CLI: các lệnh, parser và UI helpers | `struct Cli`, `enum CliTaskStatus`, `fn from`, `enum Commands` |
| [`cli/src/main.rs`](../../crates/custos-app/cli/src/main.rs) | 4 | Điểm khởi đầu thực thi duy nhất của `custos-cli` | None |
| [`cli/src/ui/art.rs`](../../crates/custos-app/cli/src/ui/art.rs) | 751 | Đồ họa ASCII và giao diện khởi động terminal | `fn center_text`, `fn compute_showcase_dims`, `fn get_banner_lines`, `fn compute_mascot_size` |
| [`cli/src/ui/assets.rs`](../../crates/custos-app/cli/src/ui/assets.rs) | 823 | Module assets: phục vụ lưu giữ banner/asset | `enum AssetKind`, `fn raw_bytes`, `fn filename`, `fn title` |
| [`cli/src/ui/banner.rs`](../../crates/custos-app/cli/src/ui/banner.rs) | 26 | Module banner: in banner terminal | `fn print_banner`, `fn print_text_banner` |
| [`cli/src/ui/diff.rs`](../../crates/custos-app/cli/src/ui/diff.rs) | 53 | Render mã màu cho bản vá (diff) trực quan | `struct DiffSummary`, `fn print_unified_diff` |
| [`cli/src/ui/mod.rs`](../../crates/custos-app/cli/src/ui/mod.rs) | 110 | Module mod UI terminal | `fn get_terminal_width`, `fn get_terminal_height`, `enum ResponsiveTier`, `fn current` |
| [`cli/src/ui/prompt.rs`](../../crates/custos-app/cli/src/ui/prompt.rs) | 122 | Nhận phản hồi duyệt quyền hạn từ người dùng | `enum RiskLevel`, `fn badge`, `fn confirm_execution`, `fn wait_for_mode_prompt` |
| [`cli/src/ui/spinner.rs`](../../crates/custos-app/cli/src/ui/spinner.rs) | 46 | Hiển thị hoạt ảnh tiến độ công việc | `struct CliSpinner`, `fn new`, `fn set_message`, `fn finish_success` |
| [`desktop/Cargo.toml`](../../crates/custos-app/desktop/Cargo.toml) | 24 | Module Cargo Desktop: phục vụ biên dịch executable `custos-desktop` (Tauri/Webview) | None |
| [`desktop/build.rs`](../../crates/custos-app/desktop/build.rs) | 4 | Build script Tauri cho desktop backend | `fn main` |
| [`desktop/src/lib.rs`](../../crates/custos-app/desktop/src/lib.rs) | 55 | Tauri thin host: lấy `Arc<LocalApiClient>` theo profile, forward `custos_request`/`custos_dispatch` qua Local API; không bootstrap backend riêng | `fn greet`, `fn custos_request`, `fn custos_dispatch`, `fn run` |
| [`desktop/src/main.rs`](../../crates/custos-app/desktop/src/main.rs) | 7 | Điểm khởi đầu thực thi duy nhất của `custos-desktop` | None |

---

## 4. Các Phân Hệ & Công Cụ Bổ Trợ Workspace

### 4.1. Hệ Thống Kiểm Thử Toàn Trình (`tests/`)

| Test Suite | Đường dẫn | Mục tiêu kiểm thử | Quy chuẩn nghiệm thu |
|---|---|---|---|
| **Contract Tests** | `tests/contract/` | Kiểm chứng tính tương thích giữa các Trait và Provider formats. | Không phá vỡ khế ước giao tiếp khi nâng cấp phiên bản. |
| **E2E Integration** | `tests/e2e/` | Kiểm thử toàn trình từ Client CLI -> Daemon IPC -> SQLite WAL. | Xác nhận vòng đời đầy đủ của Task chạy độc lập không lỗi. |
| **Crash Resilience**| `tests/crash/` | Giả lập sập nguồn đột ngột (`SIGKILL`) trước/sau commit transaction. | Chứng minh không bao giờ mất dữ liệu hoặc tha hóa SQLite WAL. |
| **Test Support** | `tests/custos-test-support/` | Cung cấp MCP fixture servers, session mockers, OTEL guards. | Hỗ trợ giả lập môi trường kiểm thử cô lập, nhanh chóng. |

### 4.2. Trí Tuệ Kho Mã Nguồn: Custos Nexus (`tools/repo_intelligent/`)

- **Đường dẫn:** `tools/repo_intelligent/`
- **Bản chất:** Động cơ nội soi và thấu hiểu mã nguồn cục bộ dựa trên Tree-sitter AST, xây dựng đồ thị gọi hàm (Call Graph), tìm kiếm ngữ nghĩa FTS5 và giao thức MCP Server.
- **Cơ sở dữ liệu:** SQLite WAL (`nexus_index.db`) lưu trữ toàn bộ symbols, call edges và semantic chunks của repository.
- **Các công cụ MCP cung cấp:** `nexus_map`, `nexus_overview`, `nexus_symbol`, `nexus_callers`, `nexus_callees`, `nexus_flow`, `nexus_file`, `nexus_crate`, `nexus_search`, `nexus_read`.

---

## 5. Quy Tắc Bất Biến Về Ranh Giới Mã Nguồn (Blast Radius Rules)

1. **Zero-I/O Domain Purity:** Tuyệt đối không thêm bất kỳ thư viện async runtime, filesystem, mạng, hoặc database nào vào `crates/custos-domain`. Mọi I/O phải đi qua cổng Trait trừu tượng.
2. **Sole Composition Root:** Chỉ duy nhất `crates/custos-daemon/src/main.rs` được phép kết hợp các implementation cụ thể từ `custos-persistence` và `custos-adapters` vào `custos-runtime`.
3. **Decoupled Client Access:** `custos-cli` và `ui/` tuyệt đối không được truy cập trực tiếp file cơ sở dữ liệu SQLite; mọi thao tác phải thông qua `custos-sdk` gửi yêu cầu đến Daemon qua Unix Domain Socket.
4. **Proof-Carrying Verification:** Không một Agent nào được phép tự đánh dấu hoàn thành tác vụ nếu thiếu bằng chứng đã được thẩm tra độc lập qua `CompletionGate` tại `crates/custos-core/src/kernel/completion.rs`.
