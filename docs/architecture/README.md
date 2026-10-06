# Kiến trúc Custos

Kiến trúc đích là một Agent Workspace local-first với Coding, Research và Assistant. Một UI shell kết hợp conversation/tài nguyên/artifacts; runtime giữ công việc; authority giữ quyền; evidence giữ trạng thái theo criterion. Native harness được tích hợp theo capability/assurance thực, không giả mọi tool của nó đã qua gateway.

## Trải nghiệm và tổ chức năng lực

| Tài liệu | Câu hỏi giải quyết |
|---|---|
| [SADE design và supervision](sade-design-and-supervision.md) | Bản sắc Custos và cách phối hợp S1–S2–human tối ưu kết quả/chi phí là gì? |
| [Agent Workspace và UI](agent-workspace-and-ui.md) | Người dùng mở/chạy/xem/so/duyệt/tiếp tục công việc thế nào? |
| [Capabilities và skills](capability-catalog-and-skills.md) | Feature mới nằm đâu và dùng lại giữa pack/model/harness thế nào? |
| [Domain packs](domain-packs-and-workflows.md) | Coding, Research, Assistant có contracts và verifier obligations gì? |
| [Repo Intelligence](repo-intelligence-subsystem.md) | Snapshot/index/query/context/source coverage thuộc subsystem nào? |

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
