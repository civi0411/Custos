# Custos — Claude Code Guidelines

Read `AGENTS.md` completely before inspecting or changing this repository. It is the SINGLE source of truth for agent behavior and coding boundaries.
Read `Custos.md` for the absolute architectural Source of Truth.
Read `docs/development/codebase-architecture.md` for the authoritative physical file-by-file codebase catalog.

**Core Directives for Claude:**
1. **The Documentation Triad is Mandatory:** Whenever a new architectural decision or structural change is agreed upon, you MUST synchronously update:
   - `Custos.md` under the matching topic chapter (Part 1-18).
   - `docs/` under the corresponding topic specification (`docs/architecture/`, `docs/reference/`, `docs/development/`).
   - `docs/development/codebase-architecture.md` inserting the file/structs/traits at the exact matching crate and table.
   Never implement architectural code changes before updating this documentation triad.
2. **Never Guess Architecture:** Before touching code, inspect `docs/development/codebase-architecture.md` to see where files live, their architectural responsibilities, and crate owners.
3. **Strict Crate Boundaries:** Respect the 11 canonical product crates. `crates/custos-domain` MUST have Zero-I/O. `crates/custos-bridge` MUST NEVER access `crates/custos-persistence` directly. Put vendor formats strictly in `crates/custos-adapters`.
4. **Rust Quality:** Zero `.unwrap()`, zero `.expect()`, zero `panic!()` in production code. Return typed `Result<T, E>` with `thiserror`.
5. **No Decorative Emojis:** Never use emojis in code comments, markdown files, commit logs, or headings.
6. **Git Safety:** Never stage, commit, push, merge, or rewrite Git history without explicit, operation-specific authorization from the human operator.
7. **No Forbidden Frameworks:** Do not add dependencies like LangChain or LlamaIndex. Do not invent new crates outside the 11 canonical product crates.

Silence is safer than speculation.
