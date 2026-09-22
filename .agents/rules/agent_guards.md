# Custos — Agent Rules (Project-Level Supplement)

> Extends the master [`AGENTS.md`](../../AGENTS.md) with precise, actionable agent guards for this repository.

---

## Guard 1 — Always Read AGENTS.md First

Before modifying any file in this repository, read [`AGENTS.md`](../../AGENTS.md) in its entirety. It is the authoritative behavioral contract.

## Guard 2 — Ownership Verification Before Any Edit

Before editing any file, confirm:
1. Which team member owns this file (Vi / Truong / Vinh)?
2. Am I currently assisting that team member?
3. If no — stop and surface the cross-boundary concern.

## Guard 3 — Git Operations Require Explicit Confirmation

The following require the human operator to explicitly write approval before execution:

| Operation | Required Confirmation |
|---|---|
| `git add` | List files and confirm |
| `git commit` | Show message, wait for "yes" |
| `git push` | Confirm branch and remote |
| `git rebase / merge` | Confirm source + target |
| `git reset --hard` | Double-confirm (HIGH RISK) |
| `git push --force` | **PROHIBITED** unless operator types it |

## Guard 4 — Language Enforcement

Any file under `docs/` or `dev_docs/` must be 100% Technical English. If you detect non-English prose in a file you are about to modify, flag it and refuse to extend it unless correcting to English.

## Guard 5 — Anti-Pattern Check Before Code Generation

Before generating or suggesting any Rust code, verify:
- [ ] No `.unwrap()` or `.expect()` in non-test paths
- [ ] All errors use `thiserror` typed errors
- [ ] No new external crates without human authorization
- [ ] No sidecar code touching SQLite directly
- [ ] No provider SDK imported into `crates/` or `apps/`

## Guard 6 — Commit Message Format

Every commit message MUST follow Conventional Commits:
```
<type>(<scope>): <imperative summary, ≤72 chars, Technical English>
```
Reject any commit message containing: emojis, Vietnamese, vague summaries (`"update"`, `"fix stuff"`), or file paths as the summary.
