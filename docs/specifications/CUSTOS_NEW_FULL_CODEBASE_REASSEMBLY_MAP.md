# CUSTOS_NEW — BẢN ĐỒ TỔNG THỂ ĐỊNH HÌNH & BỐ TRÍ MÃ NGUỒN (MASTER CODEBASE REASSEMBLY MAP)

> **Ngày thực hiện:** 26/09/2026  
> **Cơ sở phân tích:** 6 tài liệu kiến trúc nền tảng + Nexus Lens V2 (2,458 files, 14,468 AST symbols)  
> **Mục tiêu:** Cắt, định hình và di chuyển toàn bộ các module tinh hoa của **Goose** và **Custos cũ** vào đúng vị trí cấu trúc của **Custos_new** (Modular Monolith), sẵn sàng chia nhỏ để triển khai chi tiết.

---

## I. TỔNG QUAN KIẾN TRÚC PHÂN TẦNG CỦA CUSTOS_NEW

```
                                  +--------------------------------------------------------+
                                  |                  HUMAN OPERATOR / CLI                  |
                                  |             (crates/app/custos-cli: TUI/UI)            |
                                  +--------------------------------------------------------+
                                                              |
                                                    Local API / IPC Protocol
                                                              v
                                  +--------------------------------------------------------+
                                  |         COMPOSITION ROOT: crates/app/custos-daemon     |
                                  |             (main.rs, runtime.rs, api.rs)              |
                                  +--------------------------------------------------------+
                                                              |
                 +--------------------------------------------+--------------------------------------------+
                 |                                            |                                            |
                 v                                            v                                            v
    +-------------------------+                  +-------------------------+                  +-------------------------+
    |       TASK KERNEL       | <==============> |    AUTHORITY ENGINE     |                  |   COGNITIVE RUNTIME     |
    |   (custos-kernel)       |   Permit Gate    |    (custos-security)    |                  |   (custos-cognitive)    |
    | CQRS / Events / Reducer |                  | Grants / Permits / Gate |                  | S1/S2 Router / RDC / LLM|
    +-------------------------+                  +-------------------------+                  +-------------------------+
                 |                                            |                                            |
                 | Spans / Packets                            | Dispatch / Sandbox                         | Worker Turns
                 v                                            v                                            v
    +-------------------------+                  +-------------------------+                  +-------------------------+
    |    WORKFLOW RUNTIME     |                  |   CAPABILITY GATEWAY    |                  |    CONTEXT ENGINE       |
    |   (custos-workflow)     |                  |  (security/gateway)     |                  |   (custos-context)      |
    | Operation Machine/Summon|                  | Deterministic / Seatbelt|                  | AST / Graph / Compaction|
    +-------------------------+                  +-------------------------+                  +-------------------------+
                 |                                            |                                            |
                 +---------------------+----------------------+--------------------------------------------+
                                       |
                                       v
    +-------------------------------------------------------------------------------------------------------------------+
    |                                                ADAPTERS & PACKS                                                   |
    |  - MCP Adapters (custos-adapters-mcp: stdio, http, extension_manager, mcp_client)                                 |
    |  - Provider Fleet (custos-adapters-provider: OpenAI, Claude, Local GGUF, Databricks, etc.)                        |
    |  - Engineering Pack (custos-packs-engineering: repo-explain, bug-fix, edit, shell, tree)                         |
    |  - Research Pack (custos-packs-research: paper-reading, export_markdown, import_formats)                          |
    |  - Assistant Pack (custos-packs-assistant: draft, resolve, exact approval)                                        |
    |  - Persistence (custos-persistence: SQLite WAL, Task/Span Repositories, Artifact CAS Store)                       |
    +-------------------------------------------------------------------------------------------------------------------+
```

---

## II. MA TRẬN ĐỊNH HÌNH & DI CHUYỂN FILE CHI TIẾT (MODULE BY MODULE)

### 1. `crates/core/custos-domain` (Core Contracts & Strongly-Typed IDs)
- **Nguồn:** Trích xuất từ `Custos/crates/core-domain/src/`.
- **Trách nhiệm:** Lưu giữ toàn bộ các value objects bất biến, typed ID, enum trạng thái và contract cơ sở.
- **Danh sách file đã bố trí:**
  - `ids.rs`: `TaskId`, `TaskRevisionId`, `WorkerRunId`, `ActionIntentId`, `AttemptId`, `ReceiptId`, `EvidenceRecordId`.
  - `task.rs`: `TaskContract`, `TaskStatus` (Draft, Ready, Running, Waiting, Verifying, Succeeded, Failed).
  - `action.rs`: `ActionIntent`, `ActionAttempt`, `ActionReceipt`.
  - `authority.rs`: `Grant`, `ApprovalRequest`, `ExecutionPermit`, `PolicyDecision`.
  - `evidence.rs`: `EvidenceRecord`, `EvidenceCriterion`, `EvidenceStatus`.
  - `budget.rs`: `TokenBudget`, `CostReservation`, `CostSettlement`.
  - `context.rs`: `ContextPack`, `SourceVersionId`, `ArtifactDigest`.
  - `continuation.rs`: `ContinuationPacket` (phục hồi an toàn sau crash).
  - `claim.rs`, `fact.rs`, `run.rs`, `span.rs`, `types.rs`, `workflow.rs`, `error.rs`, `lib.rs`.

---

### 2. `crates/core/custos-kernel` (CQRS Task State Machine & Invariants)
- **Nguồn:** Trích xuất từ `Custos/crates/task-kernel/src/`.
- **Trách nhiệm:** Trái tim điều phối trạng thái tác vụ, chặn đứng state machine 17-bước lộn xộn của Goose, thay bằng mô hình CQRS Reducer chuẩn với optimistic locking (`expected_epoch`).
- **Danh sách file đã bố trí:**
  - `commands.rs`: `CreateTask`, `AdvanceTask`, `CancelTask`, `StartRun`, `RequestAction`.
  - `events.rs`: `TaskCreated`, `TaskAdvanced`, `RunStarted`, `ActionRequested`, `ReceiptRecorded`.
  - `reducer.rs`: Pure reducer áp dụng event vào state, kiểm soát `epoch`.
  - `invariants.rs`: Kiểm tra tính toàn vẹn nghiệp vụ trước khi chuyển trạng thái.
  - `completion.rs`: Completion Gate — chỉ cho phép sang `Succeeded` khi toàn bộ evidence PASS.
  - `service.rs`, `span_service.rs`, `ports.rs`, `lib.rs`.

---

### 3. `crates/runtime/custos-security` (Authority Engine, Gateway, Evidence & Sandbox)
- **Nguồn:** Hợp nhất từ `Custos/crates/authority-engine/`, `Custos/crates/capability-gateway/`, `Custos/crates/evidence-engine/`, và `Custos/adapters/sandboxes/`.
- **Trách nhiệm:** Đảm bảo nguyên tắc Zero-Trust, cách ly sandbox và xác minh bằng chứng hậu kiểm.
- **Danh sách file đã bố trí:**
  - `src/authority/`:
    - `grants.rs`: Phạm vi quyền hạn ủy thác (`SubGrant`).
    - `approvals.rs`: Cơ chế Human-in-the-Loop khi rủi ro cao.
    - `permits.rs`: Cấp phát thẻ bài đơn kỳ (`ExecutionPermit`).
    - `policy.rs`: Đánh giá chính sách bảo mật Zero-trust.
    - `risk.rs`: Phân loại rủi ro (Low, Medium, High, Critical).
    - `audit.rs`: Ghi nhận nhật ký kiểm toán không thể sửa đổi.
  - `src/gateway/`:
    - `deterministic.rs`: Rào chắn chặn mọi tool call từ Agent trước khi chạm OS.
    - `traits.rs`: Interface cổng thực thi.
  - `src/evidence/`:
    - `verifier.rs`: 4 chốt chặn kiểm duyệt (CommandExitCode, HashVerifier, ExactMatch, Citation).
    - `pipeline.rs`: Quy trình chạy pipeline xác minh bằng chứng.
    - `bundle.rs`: Gom nhóm bằng chứng kết quả (`OutcomeBundle`).
  - `src/sandbox/`:
    - `seatbelt.rs`: Cách ly sandbox trên macOS (Seatbelt profiles).
    - `bubblewrap.rs`: Cách ly sandbox trên Linux (Bubblewrap namespaces).

---

### 4. `crates/runtime/custos-context` (Context Compiler, Repo Intelligence & Compaction)
- **Nguồn:** Hợp nhất từ `Custos/crates/context-compiler/`, `Custos/crates/repo-intelligence/`, `Custos/crates/memory-service/` và `goose/crates/goose-context-management/`.
- **Trách nhiệm:** Trích xuất ngữ cảnh AST, quản lý bộ nhớ tri thức, và nén ngữ cảnh tự động.
- **Danh sách file đã bố trí:**
  - `src/compiler/`:
    - `compiler.rs`: Trình biên dịch tạo `ContextPack` bất biến theo token budget.
    - `recipe.rs`: Recipe ngữ cảnh theo từng tác vụ.
    - `traits.rs`: Abstraction cổng ngữ cảnh.
  - `src/repo_intelligence/`:
    - `scanner.rs`: Quét cấu trúc repository, file changes, dirty worktree.
    - `graph.rs`: Xây dựng AST call-graph và symbol topology.
    - `llm_view.rs`: Tạo chế độ xem tối ưu token cho LLM.
  - `src/memory/`:
    - `traits.rs`: Interface lưu trữ sự thật đã kiểm chứng (scoped accepted facts).
  - `src/compaction/`:
    - `summarize.rs`, `structured.rs`, `format.rs`, `model.rs`, `provider.rs`, `templates.rs`: Bộ nén ngữ cảnh đa tầng (Hierarchical Compaction) từ Goose.
    - `prompts/`: Prompt mẫu nén hội thoại.

---

### 5. `crates/runtime/custos-workflow` (Worker Operation Machine, Sub-Agents & Recipes)
- **Nguồn:** Hợp nhất từ `goose/crates/goose-agent/`, `goose/crates/goose/src/agents/platform_extensions/summon.rs`, `goose/crates/goose/src/recipe/`, và `Custos/crates/workflow-runtime/`.
- **Trách nhiệm:** Vòng lặp re-entrant từng turn của Worker, điều phối agent con và kịch bản tự động hóa.
- **Danh sách file đã bố trí:**
  - `machine.rs`: Máy trạng thái re-entrant lưu vết bền vững sau mỗi thao tác.
  - `operation.rs`: Các thao tác tường minh (cancellation, steering, limits, retry, completion).
  - `tool.rs`: Vòng đời gọi tool và nhận kết quả tool.
  - `dispatcher.rs`: Điều phối công việc theo worker pool.
  - `lease.rs`: Quản lý lease và khoá ngăn xung đột thực thi đồng thời.
  - `outbox.rs`: Transactional outbox đảm bảo độ tin cậy sự kiện.
  - `subagent/summon.rs`: Cơ chế triệu hồi sub-agent kế thừa từ Goose, gắn kèm ngân sách (`SubGrant`).
  - `recipe/`: Toàn bộ động cơ tự động hóa kịch bản đa bước (`template_recipe.rs`, `validate_recipe.rs`, `local_recipes.rs`, `manifest.rs`).

---

### 6. `crates/runtime/custos-cognitive` (S1/S2 Routing, RDC Cycle & Deliberation)
- **Nguồn:** Hợp nhất từ `Custos/crates/cognitive-runtime/`, `Custos/crates/judgment-contracts/`, `Custos/crates/deliberation-contracts/`, và logic `inference.rs` của Goose.
- **Trách nhiệm:** Điều phối nhận thức, phân luồng S1 (nhanh/rẻ) vs S2 (sâu/chất lượng), và chu trình RDC.
- **Danh sách file đã bố trí:**
  - `routing.rs`: Định tuyến nhận thức S1/S2 có kiểm soát chi phí và ràng buộc user pin.
  - `rdc.rs`: Chu trình Resolve (S1 chọn hướng) -> Delegate (S2 thực hiện) -> Check (Verifier đối soát).
  - `arbiter.rs`: Trọng tài phán quyết giữa các phương án đề xuất.
  - `inference.rs`: Quản lý vòng đời gọi suy luận, xử lý streaming, timeout và hủy lệnh.
  - `judgment.rs`: Contract phán đoán với khả năng chủ động từ chối (`abstain`).
  - `deliberation.rs`: Contract suy ngẫm nhiều bước cho tác vụ phức tạp.

---

### 7. `crates/adapters/custos-adapters-mcp` (Model Context Protocol Hub)
- **Nguồn:** Trích xuất từ `goose/crates/goose/src/agents/extension_manager/` và `goose/crates/goose/src/agents/mcp_client.rs`.
- **Trách nhiệm:** Kết nối và định tuyến toàn bộ máy chủ công cụ MCP bên ngoài.
- **Danh sách file đã bố trí:**
  - `stdio.rs`: Quản lý tiến trình con MCP qua Stdio transport.
  - `http.rs`: Giao tiếp Streamable HTTP transport, phiên làm việc và xác thực.
  - `mcp_client.rs`: Client MCP chuẩn hóa (Tools, Resources, Prompts).
  - `extension_manager.rs`: Bộ định tuyến mở rộng (Extension Router) quản lý danh mục tool.
  - `builtin.rs`: Bộ đăng ký extensions nội tại.

---

### 8. `crates/adapters/custos-adapters-provider` (LLM Provider Fleet)
- **Nguồn:** Trích xuất từ `goose/crates/goose-providers/` và `goose-provider-types/`.
- **Trách nhiệm:** Adapter giao tiếp với hơn 30+ nhà cung cấp mô hình AI lớn nhỏ.
- **Danh sách file đã bố trí:**
  - `src/formats/`: `openai.rs`, `anthropic.rs`, `google.rs`, `azure.rs`, `bedrock.rs`, `ollama.rs`, `snowflake.rs`, `databricks.rs`, `openrouter.rs`, `deepseek.rs`.
  - `src/model.rs`, `retry.rs`, `thinking.rs`, `images.rs`, `permission.rs`, `request_log.rs`.

---

### 9. `crates/packs/custos-packs-engineering` (Engineering Pack)
- **Nguồn:** Hợp nhất từ `Custos/domain-packs/engineering/`, `goose/crates/goose/src/agents/platform_extensions/developer/`, `Custos/adapters/tools/`, và `Custos/crates/domain-pack-sdk/`.
- **Trách nhiệm:** Gói nghiệp vụ chuyên sâu cho lập trình viên (hiểu repo, sửa lỗi, tạo diff, chạy test).
- **Danh sách file đã bố trí:**
  - `declarative/`:
    - `pack.yaml`: Khai báo manifest pack.
    - `tasks/`: `repo-explain.yaml`, `bug-fix.yaml`.
    - `workflows/`: `repo-explain.v1.yaml`, `bug-fix.v1.yaml`.
    - `context-recipes/`: `triage.v1.yaml`.
    - `policies/`: `read-only.v1.yaml`.
    - `prompts/`: `plan.v1.md`, `diagnose.v1.md`, `implement.v1.md`, `coding-plan.v1.md`.
    - `verifiers/`: `rust.v1.yaml`.
  - `src/tools/`:
    - `edit.rs`: Thao tác sửa file, regex replacement, multi-replace từ Goose.
    - `shell.rs`: Thực thi lệnh terminal có kiểm soát từ Goose.
    - `tree.rs`: Quét cây thư mục và tìm kiếm file từ Goose.
    - `image.rs`: Xem và phân tích hình ảnh từ Goose.
    - `shell_output_streaming.rs`: Stream output từ shell tiến trình con.
    - `standard_tools.rs`: Bộ công cụ hệ thống chuẩn của Custos cũ.
  - `src/sdk.rs`: SDK kết nối domain pack vào Kernel.

---

### 10. `crates/packs/custos-packs-research` (Research Pack)
- **Nguồn:** Hợp nhất từ `Custos/domain-packs/research/`, `goose/crates/goose/src/session/export_markdown.rs`, và `goose/crates/goose/src/session/import_formats/`.
- **Trách nhiệm:** Nghiên cứu tài liệu, trích xuất luận điểm khoa học, đối soát claim và xuất ghi chú Markdown/Obsidian.
- **Danh sách file đã bố trí:**
  - `declarative/pack.yaml`: Manifest Research Pack.
  - `src/markdown_export/mod.rs`: Trình kết xuất ghi chú Markdown từ Goose (không bị overwrite conflict).
  - `src/import_formats/`: Bộ chuyển đổi định dạng phiên làm việc (`claude_code.rs`, `codex.rs`, `pi.rs`, `mod.rs`).

---

### 11. `crates/packs/custos-packs-assistant` (Personal Assistant Pack)
- **Nguồn:** Hợp nhất từ `Custos/domain-packs/personal/`.
- **Trách nhiệm:** Soạn thảo tin nhắn, phân giải người nhận, quản lý lịch trình, và gửi đi có Human Approval.
- **Danh sách file đã bố trí:**
  - `declarative/pack.yaml`: Manifest Assistant Pack.

---

### 12. `crates/infrastructure/custos-persistence` (SQLite WAL & Artifact CAS)
- **Nguồn:** Hợp nhất từ `Custos/crates/persistence-sqlite/` và `Custos/crates/artifact-store/`.
- **Trách nhiệm:** Lưu trữ bền vững trạng thái, event log và tệp artifact content-addressed (CAS).
- **Danh sách file đã bố trí:**
  - `migrations/`: `0001_core.sql` -> `0005_evidence.sql`.
  - `connection.rs`: Khởi tạo SQLite connection pool ở chế độ WAL và busy_timeout.
  - `store.rs`: Triển khai `TaskStorePort` với Event Sourcing và Transactional Outbox.
  - `repositories/`: `task.rs`, `span.rs`, `continuation.rs`.
  - `artifacts/`: `filesystem.rs`, `traits.rs`, `lib.rs` (Content-Addressed Storage cho artifacts).

---

### 13. `crates/app/custos-daemon` (Composition Root & Local API Server)
- **Nguồn:** Hợp nhất từ `Custos/apps/custosd/` và `Custos/crates/local-api/`.
- **Trách nhiệm:** Điểm khởi động toàn hệ thống (Composition Root), quản lý vòng đời tiến trình ngầm, và mở cổng Local API / IPC.
- **Danh sách file đã bố trí:**
  - `main.rs`: Điểm khởi chạy daemon với async runtime, liên kết toàn bộ crates thành viên.
  - `runtime.rs`: Giám sát và điều phối vòng đời shutdown graceful.
  - `api.rs`: Dispatcher và DTOs của Local API (hỗ trợ CLI, VS Code, UI qua IPC).

---

### 14. `crates/app/custos-cli` (Terminal UI & User Experience)
- **Nguồn:** Trích xuất từ `Custos/apps/custos-cli/`.
- **Trách nhiệm:** Trình điều khiển dòng lệnh cao cấp với hiển thị tiến trình, render diff và duyệt tác vụ.
- **Danh sách file đã bố trí:**
  - `main.rs`, `lib.rs`.
  - `ui/`: `banner.rs`, `diff.rs`, `prompt.rs`, `spinner.rs`, `art.rs`, `assets.rs`.
  - `frame-ui/`: Bộ linh vật Owl nhận diện thương hiệu Custos.

---

## III. MA TRẬN PHÂN CÔNG & TIÊU CHUẨN NGHIỆM THU

| Miền trách nhiệm | Người phụ trách | Các crates sở hữu |
|---|---|---|
| **Bảo Mật & Nền Tảng (Platform & Security)** | **Trường** | `custos-security`, `custos-persistence`, `custos-daemon`, `custos-cli` |
| **Điều Phối & Giao Thức (Runtime & Protocols)** | **Vinh** | `custos-kernel`, `custos-workflow`, `custos-adapters-mcp` |
| **Trí Tuệ AI & Ứng Dụng (AI Systems & Packs)** | **Vĩ** | `custos-domain`, `custos-cognitive`, `custos-context`, `custos-adapters-provider`, 3 Domain Packs |

---

## IV. BƯỚC TIẾP THEO

Toàn bộ khung xương, vị trí file và các module cốt lõi đã được định vị chính xác vào `Custos_new`.  
Hệ thống sẵn sàng để:
1. Ngày mai bạn rà soát lại bố cục thư mục.
2. Tách nhỏ từng phần việc (Gate-by-Gate: Domain -> Kernel -> Persistence -> Workflow -> Cognitive -> Security -> Packs -> CLI).
3. Tinh chỉnh mã nguồn từng crate để đạt độ tương thích hoàn hảo.
