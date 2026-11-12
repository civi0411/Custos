# Custos — Copilot / Codex Instructions

Read `AGENTS.md` at the repository root completely before inspecting or changing this repository. It is the SINGLE source of truth for agent behavior and coding boundaries.
Read `Custos.md` for the absolute architectural Source of Truth.
Read `docs/development/codebase-architecture.md` for the authoritative physical file-by-file codebase catalog.

**Core Directives:**
1. **Mandatory Documentation Triad:** Any architectural change or new subsystem must be synchronized across:
   - `Custos.md` (SSOT: philosophy, invariants, core contracts).
   - `docs/` (Topic engineering specs: architecture, reference, development).
   - `docs/development/codebase-architecture.md` (Physical AST index and file catalog, inserted at the exact crate).
   Never implement architectural code changes before updating the triad.
2. **Preserve 3-Ring Modular Boundaries:** Packages live under `platform/`, `workbenches/`, `sdk/`, `apps/`, `tests/`, and `xtask/`. Keep `platform/domain` (`custos-domain`) completely free of persistence, network, and provider I/O (Zero-I/O), keep SQLite single-writer inside `platform/storage` (`custos-persistence`), and keep kernel layers free of vertical workbench dependencies (`cargo xtask check-boundaries`).
3. **Consult Codebase Architecture Map:** Before editing or adding files, inspect `docs/development/codebase-architecture.md` to establish a shared mental model of file responsibilities and owners (Vĩ, Trường, Vinh).
4. **Rust Standards:** No `panic!()` or `.unwrap()` in production code. Use typed errors with `thiserror`. Add `tracing` spans with correlation IDs.
5. **Zero Formatting Junk:** Do not use decorative emojis anywhere. Do not embed artificial version numbers or dates in documentation headers.
6. **Git Safety:** Never suggest running destructive git commands or auto-commit without explicit permission.

Silence is safer than speculation.
