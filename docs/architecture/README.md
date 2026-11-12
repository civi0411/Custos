# Kiến trúc Custos

Kiến trúc đích là một Agent Workspace local-first với 4 bàn làm việc chuyên biệt: Coding, Research, Assistant và Quant. Một UI shell kết hợp conversation/tài nguyên/artifacts; runtime giữ công việc; authority giữ quyền; evidence giữ trạng thái theo criterion; AEE bảo đảm môi trường thực thi tất định và honest assurance. Native harness được tích hợp theo capability/assurance thực, không giả mọi tool của nó đã qua gateway.

## Trải nghiệm và tổ chức năng lực

| Tài liệu | Câu hỏi giải quyết |
|---|---|
| [Agent Execution Environment (AEE)](agent-environment.md) | Kiến trúc môi trường thực thi của agent, honest assurance và 7 cổng bất biến `ENV-01..ENV-07` |
| [Domain Ownership & Boundary Ledger](domain-ownership.md) | Phân định ranh giới sở hữu kiểu dữ liệu miền, Zero-Kernel Leakage và cách ly workbench-kernel |
| [SADE Foundations: Multi-Domain Kernel](sade-foundations.md) | Trích xuất generic supervised execution kernel từ Orca, đối chiếu 7 primitives × 4 domains và các invariant gates |
| [SADE Agent Lake Ecosystem: Living Commons](sade-agent-lake-ecosystem.md) | Thiết kế "Ao Hồ SADE" cho các agent đa dạng cùng chung sống, trao đổi tài nguyên URR, bus sự kiện và cộng sinh phát triển |
| [SADE Platform Deep Audit & Refactoring](sade-platform-deep-research-and-refactor.md) | Kiểm toán toàn diện 538 files thuộc 8 platform crates, chuẩn hóa URR, EventBus mở rộng và cấu trúc OOP/Hexagonal |
| [Desktop ↔ core interaction map](desktop-core-interaction-map.md) | UI trên nhánh refactor đã push tương tác với host/API/runtime/data thế nào, và phần nào chưa đạt? |
| [SADE design và supervision](sade-design-and-supervision.md) | Bản sắc Custos và cách phối hợp S1–S2–human tối ưu kết quả/chi phí là gì? |
| [Agent Workspace và UI](agent-workspace-and-ui.md) | Người dùng mở/chạy/xem/so/duyệt/tiếp tục công việc thế nào? |
| [Capabilities và skills](capability-catalog-and-skills.md) | Feature mới nằm đâu và dùng lại giữa pack/model/harness thế nào? |
| [Domain packs & workbenches](domain-packs-and-workflows.md) | Coding, Research, Assistant, Quant có contracts và verifier obligations gì? |

## Execution, trust và data

| Tài liệu | Trách nhiệm |
|---|---|
| [Task lifecycle](kernel-and-task-lifecycle.md) | State/revision/session và continuation |
| [Authority và gateway](authority-and-capability-gateway.md) | Scope/grant/permit/effect/receipt/reconcile |
| [Evidence và completion](evidence-and-completion-gate.md) | Criterion verification, stale/unknown/waiver |
| [Context và memory](data-context-and-memory.md) | Source/CAS/retrieval/prompt/projection ownership |
| [Security](security-and-threat-defense.md) | Trust boundaries, privacy, sandbox và threat fixtures |
| [OI/S1/S2/Meta](cognitive-fabric-and-orchestration.md) | Work allocation trong constraints, bounded S1 support, offline optimization |
| [Protocol/connectivity](protocol-and-connectivity-hubs.md) | Local API/IPC/HTTP, MCP, ACP, A2A và optional CAP |

UI là client; daemon là composition root; runtime điều phối qua ports; packs giữ semantics; adapters/persistence giữ concrete edges. Worktree là checkout separation, không OS sandbox. Link/citation/exit code là facts, không tự chứng minh semantic correctness.

Để triển khai, đọc [migration plan](../development/workspace-restructuring-plan.md) và [catalog](../development/codebase-architecture.md). Tài liệu master [Custos.md](../../Custos.md) giữ quyết định tổng; topic docs không chứng nhận feature đã chạy.
