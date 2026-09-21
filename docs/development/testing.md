# Testing Architecture and Durability Matrix

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§36) Canonical Specification

The Custos testing system is engineered to guarantee the absolute correctness of an autonomous task runtime managing and modifying codebase repositories.

---

## 1. Test Pyramid

```text
              ▲
             / \     [ 1. Live Provider Canaries ] (~2%)
            /   \    [ 2. E2E Repo Fixture Tests ] (~8%)
           /     \   [ 3. Chaos & Crash Matrix   ] (~15%)
          /       \  [ 4. Contract & Integration ] (~25%)
         /         \ [ 5. Pure Unit & Property   ] (~50%)
        ─────────────
```

1. **Pure Unit & Property Tests:** Verify task state machine transitions, path normalization invariants, and budget accounting algorithms using the `proptest` framework.
2. **Contract & Integration Tests:** Validate backward schema compatibility, SQLite migrations, and capability adapters against deterministic mock servers.
3. **Chaos & Crash Matrix:** Simulate abrupt power loss, socket termination, and full-disk conditions to test the recovery of the SQLite WAL and Git worktrees.
4. **E2E Repo Fixture Tests:** Execute full repository bug-fixing workflows on offline canonical repositories.
5. **Live Provider Canaries:** Periodic, budget-capped tests with real OpenAI/Anthropic APIs to detect provider protocol changes early.

---

## 2. Critical Test Matrices

### Crash Matrix
Every state transition point is tested against abrupt process termination scenarios:
- Immediately prior to committing an SQLite transaction.
- Immediately after sandbox execution finishes, but before persisting the receipt.
- When a provider aborts the network connection during mid-stream token generation.
- When disk storage utilization reaches 100%.

### Path Normalization Matrix
Validates immunity against path traversal attacks:
- Validates symlinks pointing outside the workspace boundary.
- Tests traversal sequences (`../`), non-canonical Unicode representations, and race conditions where target files are deleted while worker processes are reading them.
