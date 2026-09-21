# Custos — Claude Code Guidelines

> **Master Architecture & Rules:** See [AGENTS.md](./AGENTS.md) for full project rules and invariants.

## Essential Commands
- **Check Workspace:** `cargo check --workspace`
- **Run Tests:** `cargo test --workspace`
- **Lint Check:** `cargo clippy --workspace --all-targets -- -D warnings`

## Key Boundaries
1. **Rust Core:** `crates/`, `apps/`, `adapters/` are strictly Rust. Never place Python or TypeScript in these crates.
2. **Sidecars Only:** External runtimes live in `sidecars/` (Python for Judgment/ML, TypeScript for Claude/VS Code).
3. **Pure SE Mode:** When working on DB (`persistence-sqlite`), API (`local-api`), or CLI (`custos-cli`), behave strictly as a backend systems engineer. Treat AI responses as arbitrary JSON payloads.
4. **Zero Bloat:** Do not add third-party orchestrators (LangChain, LangGraph, etc.) or unsolicited Rust dependencies.
5. **Production Error Handling:** No `.unwrap()` or `.expect()` in non-test Rust code. Use `thiserror` and return `Result<T, E>`.

