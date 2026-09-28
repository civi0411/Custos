# Custos — Sovereign Agentic Work Operating System

> **Documentation update (2026-09-27):** This README contains older architectural and implementation claims, including complete crash recovery and gateway enforcement, that have not been re-verified in the current dirty checkout. Start with the [documentation hub](docs/README.md), [current-status rules](docs/status/README.md), [target architecture](docs/architecture/target-architecture.md), and [development workboard](dev_docs/SPRINT_STATUS.md). Do not use this README alone as an implementation certificate.

> **Custos** là hệ điều hành tác tử tự chủ (Local-First Sovereign Agentic Operating System) dành cho nhà phát triển phần mềm, nghiên cứu và quản trị tác vụ.  
> Đơn vị chân lý (Unit of Truth) của Custos là **Durable Task** có kiểm chứng bằng chứng, không phải là một phiên chat tạm thời.

---

## 1. Triết Lý & Nguyên Tắc Thiết Kế

1. **Models Propose — System Disposes:** Mô hình AI (System 1 / System 2) chỉ đề xuất kế hoạch và mã lệnh; Task Kernel kiểm soát trạng thái; Authority Engine kiểm soát quyền hạn; Capability Gateway kiểm soát hiệu ứng phụ; Evidence Engine kiểm chứng kết quả hoàn thành.
2. **Local-First & Durable:** Toàn bộ trạng thái Task, Span, Continuation, và Audit Log được lưu trữ bền vững trong SQLite (WAL mode). Hệ thống phục hồi 100% sau khi crash (`kill -9`) mà không lặp lại các side-effect đã thực hiện.
3. **No Unsandboxed Side-Effects (Invariant I1):** Mọi thao tác sửa file, chạy lệnh terminal đều phải đi qua `GatewayTool` và được cách ly trong Git Worktree hoặc OS Sandbox (macOS Seatbelt / Linux Bubblewrap).
4. **Outbound Prompt Sanitization (Invariant I7):** Mọi request gửi tới Model Provider đều qua `GatewayProvider` để rà soát rò rỉ API key, private token và dữ liệu nhạy cảm.
5. **Repo Intelligence First:** Coding Agent hiểu sâu mã nguồn nhờ AST Parser (Tree-sitter), Call Graph 2 chiều và Token-budgeted Neighborhood Slicing thay vì đọc code mò mẫm.

---

## 2. Bản Đồ Kiến Trúc Hệ Thống (Master Architecture Topology)

Toàn bộ monorepo `Custos` được tổ chức thành 6 tầng chuẩn mực cùng Subsystem Trí tuệ Mã nguồn:

```text
Custos/
├── crates/
│   ├── core/                           # TẦNG 0: CORE DOMAIN & PROTOCOLS
│   │   ├── custos-domain/              # Entity, Invariant, Task, ActionIntent, Receipt
│   │   ├── custos-kernel/              # CQRS TaskKernel, State Machine, Leases
│   │   ├── custos-bridge/              # Cầu nối Session-to-Task (promote, steer)
│   │   ├── custos-provider-sdk/        # Model Provider Port & Request/Response Types
│   │   └── custos-provider-types/      # Wire format catalog cho 15+ LLM providers
│   │
│   ├── infrastructure/                 # TẦNG 1: PERSISTENCE & STORAGE
│   │   └── custos-persistence/         # SQLite TaskStore (WAL), Repositories, FsArtifactStore
│   │
│   ├── runtime/                        # TẦNG 2: RUNTIME ENGINES
│   │   ├── custos-session/             # Interactive Session Manager (Bare / Assisted mode)
│   │   ├── custos-workflow/            # Reentrant Worker Operation Machine & DAG Executor
│   │   ├── custos-cognitive/           # System One (Reflex Router) & System Two (Deliberation)
│   │   ├── custos-context/             # Token-Aware Context Compiler & Repo Direct Reader
│   │   ├── custos-context-management/  # Structured Context Compaction & Token Pruning
│   │   ├── custos-security/            # Authority Engine, Permits, Evidence Pipeline
│   │   ├── custos-memory-service/      # Promoted Memory (Provenance, Scope, Expiry)
│   │   ├── custos-agent/               # Ephemeral Worker Lifecycle & State Machine
│   │   └── custos-engine/              # Execution Platform Extensions (edit, shell, tree)
│   │
│   ├── adapters/                       # TẦNG 3: ADAPTERS & INTEGRATIONS
│   │   ├── custos-adapters-mcp/        # GatewayTool & GatewayProvider Choke Points
│   │   ├── custos-mcp/                 # External MCP Client (stdio, SSE, streamable HTTP)
│   │   ├── custos-providers/           # HTTP Provider Implementations (Claude, OpenAI, Gemini...)
│   │   ├── custos-local-inference/     # Offline Local Model Inference (llama.cpp engine)
│   │   ├── custos-roaming/             # Cloud Sync & Roaming Adapter
│   │   ├── custos-download-manager/    # Model & Artifact Download Manager
│   │   ├── providers/                  # Specialized Model Adapters (fake, local, codex)
│   │   ├── sandboxes/                  # OS Sandboxes (macos-seatbelt, linux-bubblewrap)
│   │   └── judgments/                  # Fast Judgment Adapters (jev, onnx, rules)
│   │
│   ├── packs/                          # TẦNG 4: DOMAIN PACKS
│   │   ├── custos-packs-engineering/   # Coding Agent Pipeline (Explorer, Planner, Patcher)
│   │   ├── custos-packs-research/      # Research Agent (Citation & Claim Verification)
│   │   └── custos-packs-assistant/     # Assistant Agent (Personal Task Execution)
│   │
│   └── app/                            # TẦNG 5: USER-FACING APPLICATIONS
│       ├── custos-cli/                 # Terminal UI & Interactive CLI (`custos`)
│       ├── custos-daemon/              # Durable Background Supervisor (`custosd`)
│       ├── custos-local-api/           # Local HTTP & IPC JSON-RPC Dispatcher
│       └── custos-vscode/              # VS Code Extension Backend
│
├── tools/repo_intelligent/             # INTERNAL SUBSYSTEM: REPO GRAPH ENGINE
│   ├── src/indexer/                    # Tree-sitter Parsers (Rust, Python, TS) + SQLite FTS5
│   ├── src/graph/                      # Inbound/Outbound Call Graph, BFS Flow Pathfinding
│   ├── src/mcp_server.py               # Optional MCP Server Port (cho Cursor / Claude Desktop)
│   └── nexus_index.db                  # Codebase Knowledge Graph SQLite (FTS5 + WAL)
│
├── tests/                              # TEST SUITES & VERIFICATION
│   ├── contract/                       # Invariant I1 - I7 Contract Tests
│   ├── e2e/                            # End-to-End Vertical Slice Tests
│   └── crash/                          # Abrupt Kill-9 Crash Recovery Tests
│
├── evals/                              # AGENT BENCHMARKS & EVALUATION FIXTURES
├── ui/                                 # DESKTOP ELECTRON / REACT UI
└── xtask/                              # WORKSPACE BUILD & AUTOMATION RUNNER
```

---

## 3. Hướng Dẫn Vận Hành & Lệnh Thao Tác

### Kiểm tra toàn bộ Workspace:
```bash
cargo check --workspace
cargo test --workspace
```

### Chạy Subsystem Repo Intelligence:
```bash
# Quét và index lại toàn bộ mã nguồn
uv run --project tools/repo_intelligent repo-intel index

# Soi chiếu 360 độ một Symbol (AST, code thật, callers, callees)
uv run --project tools/repo_intelligent repo-intel symbol TaskService::execute_advance --repo custos

# Trích xuất lát cắt ngữ cảnh vừa khít ngân sách Token cho LLM
uv run --project tools/repo_intelligent repo-intel slice execute_advance --budget 3000
```

### Khởi chạy Custos CLI & Daemon:
```bash
cargo run -p custos-daemon -- --database custos.db
cargo run -p custos-cli -- status
```

---

## 4. Tài Liệu Kỹ Thuật

- [ARCHITECTURE.md](ARCHITECTURE.md): Chi tiết mô hình đối tượng, luồng thực thi và ranh giới layer.
- [AGENTS.md](AGENTS.md): Bộ quy tắc bất khả xâm phạm dành cho AI coding agents và lập trình viên.
- [dev_docs/README.md](dev_docs/README.md): Quy chuẩn cộng tác 3 thành viên (Vi, Truong, Vinh).
- [dev_docs/MODULE_OWNERSHIP.md](dev_docs/MODULE_OWNERSHIP.md): Ma trận phân công trách nhiệm từng crate.
