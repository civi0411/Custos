# Git Team Delivery & PR Workflow Specification

> Scope: Standard Git collaboration, commit convention, and PR delivery workflow in the Custos workspace.

---

## 1. Branch Strategy & Target Branch

- **Feature / Topic Branches**: `feat/<topic>`, `refactor/<topic>`, `fix/<topic>` (or developer working branch).
- **Default Target Branch**: `dev` (Primary development integration branch for all PRs).
- **Release Branch**: `main` (Production releases only, governed by release tags and automated CI gates).

---

## 2. Commit Standards & Collaboration Attribution

1. **Conventional Commits**:
   - Format: `<type>(<scope>): <concise technical English summary, max 72 chars>`
   - Allowed types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`.
   - Zero decorative emojis in commit headers and descriptions.

2. **Standard Git Trailers for Collaboration**:
   - When a commit is produced collaboratively, append standard `Co-authored-by` trailer headers:
     ```text
     feat(engine): integrate pareto optimization into task supervisor

     Evaluate multi-objective utility scoring against candidate topologies.

     Co-authored-by: Collaborator Name <collaborator@domain.com>
     ```
   - Exactly one blank line must separate the commit message body from trailer blocks.
   - Strict casing: `Co-authored-by: Name <email>`.

---

## 3. Modular Thematic PR Decomposition

To maintain fast code review cycles and bisectable git histories, avoid monolithic PRs. Decompose changes into discrete architectural layers (1–2 coherent topics per PR):

1. **Batch 1 (Contracts & Protocols)**:
   - Domain schemas, traits, DTOs, wire envelopes (`platform/contracts`, `sdk/protocol`).
2. **Batch 2 (Storage & Integrations)**:
   - Persistence drivers, outbox tables, provider codecs, adapters (`platform/storage`, `platform/integrations`).
3. **Batch 3 (Engine & Workbenches)**:
   - Governed agent loop, supervisors, verification gates, vertical packs (`platform/engine`, `workbenches/*`).
4. **Batch 4 (Applications & Documentation)**:
   - Daemon composition root, CLI, desktop UI, architecture docs (`apps/*`, `docs/*`).

---

## 4. Pre-PR Quality Verification & Delivery Flow

Before opening or merging a PR, verify local gates:

```bash
# 1. Verify Rust workspace compilation
cargo check --workspace --all-targets

# 2. Run architecture boundary validation
cargo xtask check-boundaries

# 3. Run desktop smoke tests if UI was modified
node tests/desktop/canvas-smoke.mjs

# 4. Stage and commit thematic files
git add <thematic files>
git commit -m "<type>(<scope>): <thematic subject>"

# 5. Push branch and open Pull Request
git push origin <working-branch>
gh pr create --base dev --head <working-branch> --title "<thematic title>" --body "<technical summary>"
```
