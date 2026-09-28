# Custos — Claude Code Guidelines

Follow [`AGENTS.md`](AGENTS.md) for repository-wide rules. The documentation authority, implementation status, target architecture and current repository map are indexed from [`docs/README.md`](docs/README.md).

## Workspace

- Rust workspace packages live under `crates/`; the actual membership is the Cargo workspace, not every directory containing a manifest.
- External clients and services live in `ui/`, `services/`, `oidc-proxy/` and `packages/` and communicate through versioned contracts.
- Schemas, tests, evals, examples and workflow recipes are separate from production runtime code.
- See [`docs/development/repository-structure.md`](docs/development/repository-structure.md) before moving packages or changing dependency direction.

## Engineering rules

- Preserve existing user changes and keep implementation work scoped; do not claim a target design is already implemented.
- Keep domain contracts independent of persistence, provider SDKs, network clients and runtime orchestration.
- Put provider/vendor wire formats and external protocol bindings in adapters; keep clients thin and prevent direct database access outside the daemon composition.
- Use typed errors and explicit validation at external input boundaries. Follow the crate's established error conventions instead of mechanically imposing new dependencies.
- Do not add dependencies or third-party orchestrators without explicit approval and an architecture rationale.
- Update ADRs and the repository map when a structural boundary changes.
- Never stage, commit, push, merge, or rewrite Git history without the explicit operation-specific authorization required by `AGENTS.md`.

## Verification

Use the smallest relevant checks for the changed scope. `scripts/check_deps.sh` checks hard dependency boundaries against Cargo metadata and reports known transitional edges; `--strict` treats those listed edges as failures. Run workspace checks only when the change and environment warrant them, and report exactly what was or was not verified.
