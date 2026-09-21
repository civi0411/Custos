# Custos Documentation — Start Here

> **Status:** Canonical Baseline v4.0-draft  
> **Last Updated:** 2026-09-21  
> **Scope:** Architecture, Product, Implementation, Governance

Chào mừng bạn đến với hệ thống tài liệu chính thức của **Custos** — Human-Centered Agentic Work Runtime.

Tài liệu này được biên soạn theo tiêu chuẩn kết hợp giữa **arc42**, **C4 model** và **ADR (Architectural Decision Records)** nhằm cung cấp góc nhìn đầy đủ từ triết lý sản phẩm, mô hình nhận thức, giao thức hệ thống đến chi tiết mã nguồn triển khai.

---

## 🧭 Bản Đồ Đọc Theo Vai Trò (Reading Pathways)

Tùy thuộc vào mục đích của bạn trong dự án, hãy chọn lộ trình đọc phù hợp:

### 1. Kỹ Sư Hệ Thống & Backend (Rust / Systems Engineers)
Trọng tâm: Kernel, State Machine, Capability Gateway, Sandbox, Storage.
- Bắt đầu với [Kiến Trúc Tổng Thể](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/overview.md) và [Vòng Đời Task](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/task-lifecycle.md)
- Tìm hiểu cách thực thi an toàn tại [Capability Gateway](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/capability-gateway.md)
- Đọc cơ chế lưu trữ bền vững tại [Persistence & Storage](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/persistence.md)
- Nắm vững xử lý lỗi và hồi phục tại [Crash Recovery](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/crash-recovery.md)
- Xem kiến trúc monorepo và công nghệ tại [Codebase & Monorepo](file:///Users/mac/Project/AgentHub/Custos/docs/development/codebase.md)

### 2. Kỹ Sư AI & Cognitive Architects (AI / Prompt / Agent Engineers)
Trọng tâm: Cognitive Control Fabric, System One, Providers, Context & Memory.
- Đọc [Cognitive Control Fabric](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/cognitive-fabric.md) (System One vs System Two, RDC protocol)
- Nắm chuẩn tích hợp model tại [Provider Interoperability](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/provider-interop.md)
- Quản lý bộ nhớ và context tại [Context & Memory Architecture](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/context-memory.md)
- Tìm hiểu các Domain Packs chuyên biệt: [Engineering Pack](file:///Users/mac/Project/AgentHub/Custos/docs/domains/engineering.md), [Research Pack](file:///Users/mac/Project/AgentHub/Custos/docs/domains/research.md), [Personal Pack](file:///Users/mac/Project/AgentHub/Custos/docs/domains/personal.md)

### 3. Kỹ Sư An Ninh & Platform (Security & Platform Engineers)
Trọng tâm: Trust Boundaries, Exact-Payload Approvals, Sandboxing, Secrets.
- Xem [Mô Hình Đe Dọa (Threat Model)](file:///Users/mac/Project/AgentHub/Custos/docs/security/threat-model.md)
- Xem [Mô Hình Quyền Hạn (Capability Model)](file:///Users/mac/Project/AgentHub/Custos/docs/security/capability-model.md)
- Xem [Quyền Riêng Tư & Dữ Liệu (Privacy)](file:///Users/mac/Project/AgentHub/Custos/docs/security/privacy.md)
- Xem [Triển Khai Local-First & Sandbox](file:///Users/mac/Project/AgentHub/Custos/docs/architecture/deployment.md)

### 4. Product Managers, QA & Contributors
Trọng tâm: Định vị sản phẩm, Invariants, Lộ trình, Kiểm thử, Đóng góp.
- Đọc [Bản Sắc Sản Phẩm](file:///Users/mac/Project/AgentHub/Custos/docs/product/identity.md) và [Phạm Vi Sản Phẩm (Scope & MVP)](file:///Users/mac/Project/AgentHub/Custos/docs/product/scope.md)
- Xem [Capability Map](file:///Users/mac/Project/AgentHub/Custos/docs/product/capability-map.md)
- Xem [Thuật Ngữ Cốt Lõi](file:///Users/mac/Project/AgentHub/Custos/docs/reference/concepts.md) và [Nguyên Tắc & Bất Biến](file:///Users/mac/Project/AgentHub/Custos/docs/reference/invariants.md)
- Xem [Lộ Trình 16 Tuần](file:///Users/mac/Project/AgentHub/Custos/docs/development/roadmap.md) và [Kiến Trúc Kiểm Thử](file:///Users/mac/Project/AgentHub/Custos/docs/development/testing.md)
- Tham khảo hướng dẫn đóng góp tại [CONTRIBUTING.md](file:///Users/mac/Project/AgentHub/Custos/CONTRIBUTING.md)

---

## 🗂️ Mục Lục Hệ Thống Tài Liệu (Documentation Directory)

```text
docs/
├── 00-start-here.md                 # Tài liệu này (Bản đồ đọc & định hướng)
│
├── product/                         # Nền tảng và định vị sản phẩm
│   ├── identity.md                  # Bản sắc, persona, JTBD, lợi thế cốt lõi, non-goals
│   ├── scope.md                     # Phạm vi, release horizons (H1/H2/H3), MVP Definition of Done
│   └── capability-map.md            # Bản đồ năng lực 6 miền & bảng ưu tiên tính năng
│
├── architecture/                    # Kiến trúc kỹ thuật chi tiết
│   ├── overview.md                  # Tổng quan 7 lớp, 7 planes, 3 membranes, C4 diagrams
│   ├── task-lifecycle.md            # Domain model, Task State Machine, SQLite schema, kịch bản runtime
│   ├── cognitive-fabric.md          # System One vs System Two, RDC protocol, Pluggable Backends
│   ├── capability-gateway.md        # ExecutionPermit, Tool Registration, MCP client, Sandboxing
│   ├── evidence-verification.md     # 6 Lớp Evidence, Verification Pipeline, Outcome Bundle
│   ├── communication.md             # Giao tiếp nội bộ, Star Topology, CP Message Envelope
│   ├── provider-interop.md          # ProviderPort, model adapter, ContinuationPacket, switching
│   ├── context-memory.md            # ContextPack scoring, 5 tầng memory, promotion pipeline
│   ├── persistence.md               # SQLite + WAL, Event Store, CAS, Outbox, Backup
│   ├── crash-recovery.md            # Ma trận 7 lỗi, Continuation Contract, Reconciliation loop
│   └── deployment.md                # Local daemon, Sandbox macOS/Linux, CLI & VS Code connector
│
├── domains/                         # Các gói nghiệp vụ chuyên môn (Domain Packs)
│   ├── engineering.md               # Coding Agent pipeline, Worktrees, Repo intelligence
│   ├── research.md                  # Research Agent, Claim-Evidence matrix, Obsidian export
│   └── personal.md                  # Assistant Agent, Autonomy Ladder, Connectors
│
├── security/                        # An ninh, an toàn và bảo mật
│   ├── threat-model.md              # STRIDE threat model, ranh giới tin cậy, phòng vệ injection
│   ├── capability-model.md          # Capability grants, Exact-payload human approval, Secrets
│   └── privacy.md                   # Không egress telemetry mặc định, data retention
│
├── development/                     # Quy trình phát triển & kỹ thuật
│   ├── codebase.md                  # Cấu trúc monorepo, phụ thuộc crates, công nghệ sử dụng
│   ├── roadmap.md                   # Kế hoạch 16 tuần, 10 lát cắt dọc, ma trận nghiệm thu
│   ├── testing.md                   # Kim tự tháp kiểm thử, crash/path/approval matrices
│   ├── oss-adoption.md              # 5 chế độ nhận OSS, scorecard, 7 spike bắt buộc, supply-chain
│   └── observability.md             # OpenTelemetry tracing, metrics, local structured logging
│
├── reference/                       # Tra cứu tham chiếu
│   ├── concepts.md                  # Từ điển thuật ngữ toàn diện (20+ khái niệm cốt lõi)
│   ├── invariants.md                # 10 quyết định không đảo ngược, 7 nguyên tắc, 8 invariants
│   ├── naming.md                    # Quy ước đặt tên chuẩn xác (25+ danh mục)
│   ├── comparisons.md               # So sánh với LangGraph, Temporal, AutoGen, Claude Code, v.v.
│   └── sources.md                   # Nguồn tài liệu kỹ thuật và nghiên cứu học thuật nền tảng
│
└── adr/                             # Quyết định kiến trúc (Architecture Decision Records)
    └── README.md                    # Mục lục 37 quyết định kiến trúc từ ADR-0001 đến ADR-0037
```

---

## 📌 Nguyên Tắc Tài Liệu (Documentation Rules)

1. **Thực tế trên hết:** Mọi sơ đồ, cấu trúc thư mục, và trait code đều trỏ trực tiếp đến thành phần thực thi được.
2. **Hợp đồng tường minh:** Mọi thành phần công khai (public interfaces) đều có hợp đồng kiểu dữ liệu và định nghĩa trách nhiệm.
3. **Bằng chứng xác thực:** Mọi tuyên bố về hiệu năng, tiết kiệm chi phí đều phải có benchmark chứng minh hoặc được ghi chú rõ ràng là giả thuyết (*hypothesis*).
4. **Đồng bộ phiên bản:** Mã nguồn, schema cơ sở dữ liệu và tài liệu luôn đi cùng một phiên bản phát hành (*canonical baseline*).
