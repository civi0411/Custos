# Codebase Architecture & Monorepo Blueprint

> **Status:** Canonical Baseline v4.0  
> **Source:** Part XI (§32-33) & Part VII (§49) Canonical Specification

Custos is organized as a unified Monorepo managed primarily by a Rust Cargo Workspace, complemented by TypeScript and Python sidecar packages where specialized runtimes are required.

---

## 1. Monorepo Directory Layout

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
├── sidecars/                       # Isolated auxiliary runtimes (JSON-RPC)
│   ├── python-judgment/            # Fast ML / heuristic evaluation sidecar
│   └── ts-claude-agent/            # Claude CLI adapter / VS Code connector
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

## 2. Dependency Direction Rules

To prevent circular dependencies and preserve modular encapsulation, Custos enforces a strict one-way dependency flow:

```text
[ Apps (custosd, custos-cli) ]
             |
             v
[ Crates: Runtime & Orchestration ]
             |
             v
[ Crates: Capability Gateway & Cognitive Runtime ]
             |
             v
[ Crates: Core Domain & Ports (Zero external dependencies) ]
```

- **`core-domain`** never depends on databases, networks, or AI provider SDKs. It is a pure Rust domain library consisting solely of structs, enums, and invariant definitions.
- **`adapters`** depend strictly on traits defined in `crates/`; they never cross-reference sibling adapters directly.

---

## 3. Language Split Rationale

- **Rust (Core TCB):** Used for Kernel, Gateway, Sandboxing, Persistence, CLI, and Daemon. Delivers sub-millisecond startup, verified memory safety without GC pauses, low memory overhead (< 50MB idle), and fine-grained OS resource control.
- **TypeScript (Clients & Connectors):** Used for the **VS Code Extension** and Claude CLI adapters in `sidecars/ts-claude-agent`.
- **Python (Local ML & Judgment):** Isolated in `sidecars/python-judgment` for fast heuristic scoring, local embeddings, and experimental evaluation.
