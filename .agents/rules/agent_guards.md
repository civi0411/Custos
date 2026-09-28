# Custos change guards

These checks supplement, but never override, [`AGENTS.md`](../../AGENTS.md).

## Before editing

- Identify the requested outcome and the smallest owning surface.
- Read the closest active documentation and accepted ADR.
- Inspect `git status --short` and the diff for every target file.
- Confirm ownership and required reviewers; stop before changing a shared
  contract when the required human decision is absent.
- Confirm whether the file is active, historical, vendor, generated, or an
  untracked user-created file.

## While editing

- Use additive, reviewable changes; do not rewrite unrelated user work.
- Do not run global formatters, code generators, dependency updates, or broad
  search-and-replace unless explicitly in scope.
- Keep domain contracts free of I/O and concrete adapters.
- Keep clients thin and the daemon as the production composition root.
- Keep model routing, external-agent lifecycle, MCP capability transport, and
  effect authority as distinct contracts.
- Use typed errors and fail closed at authority, path, egress, and effect
  boundaries. Follow existing crate error conventions.

## Before handoff

- Review `git diff --check` and the exact changed-file list.
- Run the smallest relevant format, compile, lint, contract, or process test.
- Report commands and limitations exactly; never convert partial evidence into
  a repository-wide success claim.
- Do not stage or commit the result without explicit approval.
