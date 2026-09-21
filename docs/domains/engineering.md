# Engineering Domain Pack

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§18) & Part VII (§55) Canonical Specification

The Engineering Domain Pack v1 is Custos's primary focus in Horizon 1 (H1), providing a hermetic, safe, and verifiable runtime environment for software engineering workflows (bug fixing, refactoring, feature slicing).

---

## 1. Engineering Execution Pipeline

Every engineering task executes through an 8-stage pipeline:

```text
[ 1. Intake & Scope   ] ---> (Parse user prompt + Resolve target paths + Invariant check)
        |
[ 2. Repo Exploration ] ---> (AST indexing via Tree-sitter + Fast regex search via ripgrep)
        |
[ 3. Scoped ContextPack]---> (Relevance scoring + Token budget enforcement)
        |
[ 4. Change Plan       ]---> (Impact analysis + Technical strategy formulation)
        |
[ 5. Isolated Worktree ]---> (Dedicated Git worktree creation; zero mutation on base branch)
        |
[ 6. Verification Loop ]---> (Linters + Typecheckers + Automated test suites)
        |
[ 7. Evidence Bundle   ]---> (Package diffs, test receipts, and token cost metrics)
        |
[ 8. Human Review Gate ]---> (Human inspects diff preview prior to committing or merging)
```

---

## 2. Ephemeral Worker Roles

In Custos, there is no monolithic "Coding Agent" retaining permanent authority. Instead, the Kernel instantiates ephemeral role-bounded workers for specific subtasks:

| Role Name | Operational Mode | Permitted Capabilities | Effect | Risk Tier |
|---|---|---|---|---|
| `engineering.explorer` | Deterministic / Fast | `repo.list_files`, `git_status`, `ripgrep` | Read-only | **R0** (Safe) |
| `engineering.planner` | Deliberative (LLM) | `repo.read_symbol`, `search_fts5` | Read-only | **R0** (Safe) |
| `engineering.patcher` | Agentic Loop | `repo.read_symbol`, `apply_patch_to_worktree` | Local mutation | **R1** (Controlled) |
| `engineering.test_author`| Deliberative (LLM) | `write_test_draft` | Local mutation | **R1** (Controlled) |
| `engineering.verifier`| Deterministic runner | `cargo test`, `ruff`, `mypy`, `pytest` | Sandboxed run | **R0** (Isolated) |
| `engineering.reviewer`| Deliberative (LLM) | `git_diff_summary` | Read-only | **R0** (Safe) |

---

## 3. Repository Intelligence

Rather than blindly transmitting whole codebases to cloud LLMs, Custos integrates native static analysis utilities executing directly on the host:
- **`BurntSushi/ripgrep`:** Sub-second regex and text searching across multi-gigabyte repositories.
- **`tree-sitter/tree-sitter` & `ast-grep`:** AST parsing to extract symbols, functions, struct declarations, and cross-file references.
- **`git worktree`:** Instantly provisions isolated working trees without duplicate disk overhead, ensuring the developer's working directory is never polluted.
