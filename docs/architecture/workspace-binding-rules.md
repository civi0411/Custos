# Workspace Binding Rules & Physical Scope Policy

> **Authority:** Custos SADE Architecture Specification  
> **Topic:** Physical vs. Logical Workspace Bindings across Workbench Lenses

---

## 1. Core Principles

Previously, UI invariants assumed every operational session strictly bound to a Git worktree. In the topic-driven multi-workbench architecture (Assistant, Coding, Research, Quant), this assumption is overly restrictive when users only need conversational assistance, paper reading, or standalone backtesting.

**Canonical Invariants:**
1. **Coding Workbench (`lens: coding`)**: Strictly bound to a physical Git repository or worktree whenever file editing, patching, or git commits are executed.
2. **Assistant Workbench (`lens: assistant`)**: Operates at Logical Project or Global scope. Does not require a Git checkout. Chat interactions, scheduling, or lookups operate without a worktree.
3. **Research Workbench (`lens: research`)**: Binds to a `NotebookKernelBinding` and paper store (`~/.custos/data/research/papers/`). Does not mandate a Git repository.
4. **Quant Workbench (`lens: quant`)**: Binds to the analytical market data directory (Parquet store) and strategy configuration. Version control of strategy source is optional and not mandatory for offline backtesting.

---

## 2. Physical Binding Matrix by Workbench

| Workbench Lens | Required Binding Type | Bound Physical Resource | Mandatory Git Worktree |
|---|---|---|:---:|
| **Assistant** | Logical Project Scope | Session journal, user preferences | No |
| **Coding** | Physical Worktree Binding | Git checkout, PTY terminal, worktree path | Yes (for mutations) |
| **Research** | Compute & Document Binding | Jupyter/Python Kernel, PDF paper store | No |
| **Quant** | Analytical Data Binding | Parquet store, DuckDB analytics file | No |

---

## 3. Cross-Workbench Scope Transitions

When a user or agent transitions across lenses within the same session:
- **Assistant to Coding**: The host resolves or prompts for a workspace/worktree path before the coding agent executes file-mutation tools.
- **Coding to Research**: The research agent inherits generated artifacts without being locked to the git worktree; independent document analysis proceeds directly.
- **Unified Session Journal**: Conversation history remains in a single append-only SQLite `session_journal` regardless of workspace binding state.
