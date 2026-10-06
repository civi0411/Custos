# Capability catalog và tổ chức skills

**Kiến trúc đích.** Áp dụng chung Coding, Research, Assistant; không tạo crate mới. Tài liệu này phân loại nơi đặt code, không tự thay đổi các trait đang tồn tại. [Master](../../Custos.md), [workspace UI](agent-workspace-and-ui.md), [catalog vật lý](../development/codebase-architecture.md) và [migration plan](../development/workspace-restructuring-plan.md) cần được đọc cùng nhau.

## 1. Phân biệt trước khi thêm tính năng

| Khái niệm | Nội dung | Nơi chịu trách nhiệm |
|---|---|---|
| Capability/tool | Một operation typed: repo query, process, PDF parse, mail send | Contract hiện hữu; I/O implementation ở adapters/persistence |
| Skill instruction | Cách làm một job bằng capabilities, examples/references | Declarative pack; loader ở runtime |
| Domain service | Semantics/rubric: diagnosis, claim reconciliation, identity ambiguity | `custos-packs/src/<pack>/` |
| Shared subsystem | Index, retrieval/context, budget, lifecycle, memory | Runtime/core/persistence theo trách nhiệm |
| Worker | Loop thực thi goal trong scope/budget | Runtime dùng ModelPort hoặc AgentRuntimePort |
| Protocol exposure | MCP tools/resources, ACP agent, remote A2A | Adapter + daemon composition, không chứa business logic duplicate |

Không dùng từ “skill” để gom mọi thứ. Một file `SKILL.md` không cấp grant; một MCP tool không nhất thiết là agent; một local worker không cần A2A. [Agent Skills specification](https://agentskills.io/specification) mô tả package instructions/resources theo progressive disclosure. [MCP tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools) mô tả bề mặt invocation; tool annotations không được tin như policy security.

## 2. Repo Intelligence là subsystem; repo_explain là skill

Repo Intelligence sở hữu snapshot-aware inventory/query/relations/coverage. `repo_explain`, `impact_analysis`, `diagnose` là consumers. RAG là retrieval/context consumer; MCP là export adapter. Không nhúng LLM loop bắt buộc trong indexer.

| Trách nhiệm | Vị trí đích trong cấu trúc đang có |
|---|---|
| Snapshot/span/query metadata thuần | `custos-domain/src/repo/` |
| Scope/preconditions/ports | `custos-core/src/repo/` và contracts hiện hữu |
| Snapshot→query→bounded ContextPack coordination | `custos-runtime/src/context/repo_intelligence/` |
| DB/index/CAS implementation | `custos-persistence` |
| Git/fs/parser/LSP/optional sidecar I/O | `custos-adapters` |
| Engineering diagnosis/skill semantics | `custos-packs/src/engineering/` |
| Recipes/rubrics/skill instructions | `custos-packs/declarative/engineering/` |
| Python indexer đang tồn tại | `tools/repo_intelligent/`: compatibility implementation, chưa xóa |

Giữ source anchors và omissions thay vì chỉ trả summary mất nguồn. Syntax match không là compiler-resolved call graph. Query cap/pagination/token budget xử lý nhiều kết quả; không cần agent hóa chỉ vì trả nhiều files. Index writes là derived infrastructure activity với scope/lifecycle, không đồng nghĩa mutation source repo. Canonical Task/effect DB không được sidecar ghi trực tiếp. Xem [Repo Intelligence](repo-intelligence-subsystem.md).

## 3. Mở rộng cả ba miền

| Tính năng | Shared foundation | Pack semantics | Concrete edge |
|---|---|---|---|
| Repo explain/impact | Source snapshot + query + context | Engineering anchors/rubric | Git/parser/fs |
| PDF claim extraction | Source/CAS/locators + context | Research claims/support/coverage | PDF/OCR/browser |
| Dataset audit | Artifact versions + bounded compute | Research leakage/split/license checks | Dataset readers/process |
| Draft/schedule | Personal source/time primitives | Assistant identity/intent/consent | Contacts/calendar |
| Send notification | Effect attempt/reconcile | Assistant exact draft/recipient | Mail/message connector |
| Context/cache optimization | Context compiler, attempt usage, privacy-scoped cache | Pack source coverage obligation | Provider cache support |

Cost optimization dùng chung trong runtime và budget policy/core; pack cung cấp task features/criteria. Không làm ba “cheap agent” độc lập, không tự đổi pinned model, không cache approval hay replay effect. Native harness usage không quan sát đủ thì báo estimated/unknown và assurance tương ứng.

## 4. Một catalog logic, không một folder chứa hết code

Catalog nối skill/task kind → capability requirements → context recipe → worker support → verifier profile → UI artifact views. Dùng pack manifest/registry hiện hữu làm entry point; không dựng database/registry thứ hai nếu không cần.

Thông tin cần quản lý: stable namespaced ID, version/schema, input/output, read/write/effect class, source/privacy requirements, timeout/cancel/retry, budget estimate, provenance obligations, supported execution paths, assurance limits, verifier và fixtures. Đây là checklist cho manifest/contract audit, không yêu cầu thêm một schema song song ngay lập tức.

Namespacing ví dụ `engineering.repo_explain`, `research.paper_read`, `assistant.draft_message`; transport adapter map tên khi protocol cần, không đổi semantic ID ngầm. Capability registry quyết định availability; authority quyết định allowed; model ranking không được gộp hai câu hỏi đó.

## 5. Dùng với hai đường thực thi

**ModelPort:** Custos worker load metadata → chọn skill khi cần → compile sources → invoke capabilities qua authority/runtime → validate typed outputs → verifier. Chỉ load instruction/reference phù hợp token budget; capability declarations không có nghĩa tự cấp phép.

**AgentRuntimePort:** handshake profile → cung cấp scope/context/recipe theo cơ chế harness hỗ trợ → offer mediated tools/MCP subset nếu interception thật → nhận artifact/events → verify ở Custos. Harness không hỗ trợ skill injection/context reuse thì ghi limitation, không giả compatible. Native tools ngoài gateway có assurance khác.

**Remote agent:** A2A chỉ khi có remote delegation, identity/auth/egress và task/artifact lifecycle phù hợp; không đổi mọi local subprocess thành A2A. [A2A core concepts](https://a2a-protocol.org/latest/topics/key-concepts/) cung cấp vocabulary remote agents/tasks/artifacts, không thay Task authority của Custos.

## 6. Quy trình thêm skill mới

1. Định nghĩa job/input/output/criterion; kiểm skill/capability đã có trước.
2. Xác định instruction, domain service, shared subsystem hay I/O adapter. Chọn owner theo bảng, không theo ngôn ngữ code.
3. Extend manifest/registry, contract hiện hữu; chỉ thêm module khi trách nhiệm thật chưa có.
4. Kiểm availability, scope/privacy, effect policy và profile cả model/native-agent paths.
5. Viết success/error/stale/cancel/unknown fixtures; verifier độc lập với output tự nhận.
6. Nối artifact view và docs/catalog; thêm golden contract tests nếu payload đổi.

Python giữ parser/OCR/ML dependency cần thiết; Rust giữ policy/lifecycle/hot path. Không ép port toàn bộ Python sang Rust hoặc bỏ folder đang dùng trước conformance tests.
