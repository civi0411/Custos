# Kiến Trúc Codebase & Monorepo Blueprint

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần XI (§32-33) & Phần VII (§49) Canonical Specification

Custos được tổ chức theo cấu trúc Monorepo thống nhất quản lý bởi Cargo Workspace của Rust, kết hợp các client packages bằng TypeScript và Python khi cần thiết.

---

## 1. Cấu Trúc Thư Mục Monorepo Chi Tiết

```text
custos/
├── Cargo.toml                      # Root Cargo workspace manifest
├── rust-toolchain.toml             # Pinned Rust version (1.82+ stable)
├── deny.toml                       # cargo-deny rules (licenses, advisories)
├── justfile                        # Command runner (build, test, lint)
│
├── apps/                           # Executable binaries & user-facing apps
│   ├── custosd/                    # Trusted local daemon
│   ├── custos-cli/                 # Thin command-line interface
│   └── custos-vscode/              # Official VS Code extension (TypeScript)
│
├── crates/                         # Core domain logic & internal libraries
│   ├── core-domain/                # Pure types, domain entities, invariants
│   ├── task-kernel/                # Task state machine, lifecycle, transactions
│   ├── workflow-runtime/           # Command/event dispatch, leases, outbox
│   ├── policy-engine/              # Grants, approval rules, Cedar mapping
│   ├── capability-gateway/         # The ONLY side-effect execution entrance
│   ├── cognitive-runtime/          # RDC protocol, Cognitive Arbiter
│   ├── judgment-contracts/         # System One schemas & interfaces
│   ├── deliberation-contracts/     # System Two worker schemas & prompts
│   ├── context-compiler/           # Slicing, scoring, ContextPack builder
│   ├── evidence-engine/            # Verifier runners, completion gates
│   ├── memory-service/             # 5 memory layers, promotion pipeline
│   ├── repo-intelligence/          # Tree-sitter AST, ripgrep, Git operations
│   ├── artifact-store/             # Content-addressed storage (CAS)
│   ├── persistence-sqlite/         # SQLite migrations, event store, queries
│   ├── local-api/                  # Axum IPC / JSON-RPC server
│   ├── provider-sdk/               # ProviderPort traits, test harnesses
│   ├── domain-pack-sdk/            # Domain Pack manifests, role definitions
│   └── observability/              # OpenTelemetry tracing, metrics, redaction
│
├── adapters/                       # Pluggable integration implementations
│   ├── providers/                  # AI Provider integrations
│   │   ├── codex/                  # OpenAI Codex adapter
│   │   ├── claude/                 # Anthropic Claude adapter
│   │   ├── antigravity/            # Google Antigravity adapter
│   │   └── local-model/            # llama.cpp / Ollama local adapter
│   ├── judgments/                  # System One backend implementations
│   │   ├── rules/                  # Fast deterministic heuristic rules
│   │   ├── onnx/                   # Local embeddings / ONNX SLM
│   │   └── jev/                    # TypeSafe Jev adapter
│   ├── sandboxes/                  # Execution containment
│   │   ├── macos-seatbelt/         # sandbox-exec wrapper for macOS
│   │   └── linux-bubblewrap/       # bwrap sandbox for Linux
│   └── tools/                      # Native tool implementations (Git, shell, FS)
│
├── domain-packs/                   # Specialized domain packages
│   ├── engineering/                # Coding agent, worktrees, lint/test verifiers
│   ├── research/                   # Claim-evidence, literature scan, Obsidian
│   └── personal/                   # Autonomy ladder, calendar/email connectors
│
├── schemas/                        # Versioned JSON Schemas & Protobufs
│   └── custos/v1/                  # Canonical schema specifications
│
├── tests/                          # Integration & System-wide Test Suites
│   ├── contract/                   # Schema compatibility tests
│   ├── crash/                      # Kill/restart & disaster recovery tests
│   ├── fixtures/                   # Sample repos & mock provider responses
│   └── e2e/                        # End-to-end task execution tests
```

---

## 2. Quy Tắc Chiều Phụ Thuộc (Dependency Direction Rules)

Để tránh hiện tượng phụ thuộc vòng (*circular dependencies*) và duy trì tính mô-đun cao, Custos áp dụng nghiêm ngặt quy tắc phụ thuộc một chiều:

```text
[ Apps (custosd, custos-cli) ]
             │
             ▼
[ Crates: Runtime & Orchestration ]
             │
             ▼
[ Crates: Capability Gateway & Cognitive Runtime ]
             │
             ▼
[ Crates: Core Domain & Ports (Zero external dependencies) ]
```

- **`core-domain`** không bao giờ được phụ thuộc vào database, network hoặc AI provider SDKs. Nó là thư viện Rust thuần túy chỉ chứa structs, enums và invariants.
- **`adapters`** phụ thuộc vào các traits được định nghĩa trong `crates/`, không bao giờ được tham chiếu chéo lẫn nhau.

---

## 3. Lý Do Phân Chia Ngôn Ngữ (Language Split Rationale)

- **Rust (90% Codebase):** Sử dụng cho Kernel, Gateway, Sandboxing, Persistence, và CLI. Mang lại tốc độ khởi động tức thì, độ tin cậy bộ nhớ tuyệt đối (*memory safety*), không tốn RAM và kiểm soát tài nguyên hệ điều hành hạt mịn.
- **TypeScript (Client & Tooling):** Sử dụng độc quyền cho **VS Code Extension** và một số mock tools để tận dụng hệ sinh thái phong phú của trình soạn thảo.
- **Python (Optional Sidecars):** Chỉ dùng cho các script kiểm thử benchmark học thuật hoặc nghiên cứu data science cục bộ nếu cần.
