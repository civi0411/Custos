# Git Team Workflow Rule: `vi` -> `dev` Achievement & Delivery Flow

> Scope: Standard development and achievement-compatible workflow for Vĩ (civi0411) in the Custos workspace.

---

## 1. Context & Default Branch
- **Working Branch**: `vi` (local and `origin/vi`).
- **Default Target Branch**: `dev` (CRITICAL: Must merge into `dev` because `dev` is the remote default branch; PRs merged to other branches do not trigger achievements).
- **Team accounts**:
  - Vĩ: `civi0411 <cuuvi985@gmail.com>`
  - Vinh: `Vinh <phuvi301@gmail.com>`
  - Trường: `NhatTruong1905 <tn696199@gmail.com>`

---

## 2. Achievement Attribution Mechanics

### Scenario C: Hybrid Rotation Strategy (MANDATORY DEFAULT)
To maximize achievements naturally and sustainably across the entire team:
- **Odd Batches (PR 1, 3, 5... — Credit both badges for Vĩ)**:
  - Commit Author: Set to teammate (Vinh or Trường).
  - Trailing `Co-authored-by:`: Assigned to Vĩ (`civi0411`).
  - Vĩ opens PR via `gh pr create` and merges into `dev`.
  - **Result**: Vĩ simultaneously earns **Pull Shark** (PR Author) and **Pair Extraordinaire** (Co-author).
  ```bash
  git commit --author="Vinh <phuvi301@gmail.com>" \
    -m "<type>(<scope>): <thematic subject>" \
    -m "Co-authored-by: civi0411 <cuuvi985@gmail.com>"
  ```
- **Even Batches (PR 2, 4, 6... — Vĩ upgrades Pull Shark & awards Pair Extraordinaire to teammates)**:
  - Commit Author: Vĩ (`civi0411 <cuuvi985@gmail.com>`).
  - Trailing `Co-authored-by:`: Both Vinh and Trường included.
  - Vĩ opens PR and merges into `dev`.
  - **Result**: Vĩ upgrades **Pull Shark**, while Vinh and Trường both receive **Pair Extraordinaire**.
  ```bash
  git commit \
    -m "<type>(<scope>): <thematic subject>" \
    -m "Co-authored-by: Vinh <phuvi301@gmail.com>

  Co-authored-by: NhatTruong1905 <tn696199@gmail.com>"
  ```

---

## 3. Automatic PR Topic Decomposition Rubric

Upon completing a scope of work, agents MUST NOT bundle everything into a monolithic PR. Instead, decompose changes into architectural layers (1–2 coherent topics per PR):
1. **Batch 1 (Contracts & Core Specs)**: Domain schemas, Provider traits, DTOs, catalog & pricing specifications.
2. **Batch 2 (Adapters & Protocol Translators)**: Client implementations, wire protocol translators, fallback logic.
3. **Batch 3 (Runtime & Engine Logic)**: Cognitive loop, Context compression/pruning, token management.
4. **Batch 4 (Gateway/MCP & Documentation Triad)**: Federation routers, MCP gateways, updating `Custos.md`, `codebase-architecture.md`, and playbooks.

---

## 4. Formatting Invariants & Execution Pipeline

1. **Mandatory Blank Line**: Exactly one blank line between Commit Subject/Body and `Co-authored-by:` trailers.
2. **Exact Syntax**: Strict casing: `Co-authored-by: Name <email@domain>`.
3. **Commit & PR Standard**: 100% Technical English, Conventional Commits (`feat(...)`, `refactor(...)`), zero decorative emojis.
4. **Sequential Execution Loop**:
   ```bash
   # 1. Stage specific thematic files
   git add <thematic files>

   # 2. Commit with designated author & co-author trailers
   git commit ...

   # 3. Push working branch
   git push origin vi

   # 4. Create and merge PR directly to dev
   gh pr create --base dev --head vi --title "<thematic title>" --body "<technical summary>"
   gh pr merge --merge

   # 5. Sync upstream dev before next batch
   git pull origin dev
   ```
