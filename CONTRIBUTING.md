# Contributing to Custos

Thank you for your interest in contributing to **Custos** — the Human-Centered Agentic Work Runtime!

Custos is an ambitious, high-reliability engineering project. To maintain architectural integrity, high code quality, and strict security invariants, all contributions must adhere to the standards outlined below.

---

## 📜 Architectural Invariants (Read Before Coding)

Before submitting any code, please familiarize yourself with our core architectural documentation:
1. **[Start Here Guide](docs/00-start-here.md)**
2. **[Ten Irreversible Decisions & System Invariants](docs/reference/invariants.md)**
3. **[Architecture Decision Records](docs/adr/README.md)**

> [!IMPORTANT]
> **Key Guardrail:** "Confidence never mints capability." All state transitions belong to the Kernel; no direct tool execution without an `ExecutionPermit`.

---

## 🛠️ Development Setup

### Prerequisites
- **Rust Toolchain:** Rust 1.82+ stable (`rustup default stable`)
- **Cargo Tools:**
  ```bash
  cargo install cargo-deny cargo-nextest just
  ```
- **System Dependencies:**
  - macOS: Xcode Command Line Tools
  - Linux: `build-essential`, `pkg-config`, `libsqlite3-dev`, `bubblewrap`
- **Node.js & pnpm:** (Only if working on the VS Code extension in `apps/custos-vscode`)

### Quick Start
1. Clone the repository:
   ```bash
   git clone git@github.com:civi0411/Custos.git
   cd Custos
   ```
2. Run automated checks:
   ```bash
   cargo check --workspace
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   ```

---

## 🌿 Branching & Git Workflow

1. **Fork & Branch:** Create your feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/issue-description
   ```
2. **Conventional Commits:** All commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:
   - `feat(kernel): add task resumption reconciliation loop`
   - `fix(gateway): invalidate execution permit on payload mismatch`
   - `docs(adr): add ADR-0038 for local memory eviction`
   - `test(crash): add sigkill recovery test matrix`
3. **Keep Commits Clean:** Atomic, well-described commits. Squash messy debugging commits before requesting review.

---

## 🧪 Testing & Verification Standards

Custos enforces a strict verification culture:
- **Zero Warnings:** All code must compile with `cargo clippy -- -D warnings`.
- **License Compliance:** All third-party dependencies must pass `cargo deny check license` (MIT, Apache-2.0, or BSD only).
- **Unit & Integration Tests:** Any new feature must include tests covering both the happy path and edge/failure cases.
- **Crash Safety:** Changes to the persistence or kernel layers must include recovery tests demonstrating that state survives process termination.

---

## 📬 Submitting a Pull Request

1. Push your branch to your fork.
2. Open a Pull Request against `main`.
3. Provide a clear PR description detailing:
   - What problem does this solve?
   - Which ADR or issue does this relate to?
   - How was this verified (commands run, test results)?
4. Ensure all CI checks pass. Maintainers will review and provide feedback.
