# Technical Foundations and References

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§44) Canonical Specification

The Custos architecture synthesizes verified industry standards and peer-reviewed research across software architecture, cognitive science, program synthesis, capability security, and local-first computing.

---

## 1. Architectural Standards and Data Contracts

- **arc42 Documentation Template:** [arc42 Overview & Structure](https://arc42.org/overview/) — Standardized template for software architecture documentation.
- **The C4 Model:** [C4 Software Architecture Model](https://c4model.com/) — Multi-tier architectural visualization framework (Context, Containers, Components, Code).
- **Architectural Decision Records (ADR):** [ADR GitHub Organization](https://adr.github.io/) — Format for recording significant architectural decisions.
- **OpenAPI Specification:** [OpenAPI 3.1 Standard](https://spec.openapis.org/oas/latest.html) — Standard interface definition language for REST APIs.

---

## 2. Cognitive Infrastructure, System One, and Fast Judgment

- **TypeSafe AI / Jev Documentation:**
  - [TypeSafe Introduction](https://docs.typesafe.ai/introduction)
  - [TypeSafe Primitives](https://docs.typesafe.ai/primitives)
  - [How to Build with System One](https://docs.typesafe.ai/concepts/how-to-build-with-system-one)
  - [Confidence Calibration](https://docs.typesafe.ai/confidence)
  - [System One Architectural Patterns](https://docs.typesafe.ai/patterns)
  - [Model Jaggedness & Jev 1.13 Evaluation](https://docs.typesafe.ai/model-jaggedness/jev-1.13)
- **Kahneman, Daniel (2011):** *Thinking, Fast and Slow* — Theoretical framework for dual-process cognitive architecture (System One fast instinctual heuristics vs. System Two deliberate computational reasoning).

---

## 3. AI Agent Frameworks and Provider SDKs

- **OpenAI Platform:**
  - [OpenAI Codex App Server](https://learn.chatgpt.com/docs/app-server)
  - [OpenAI Codex SDK Architecture](https://learn.chatgpt.com/docs/codex-sdk)
- **Anthropic Claude Ecosystem:**
  - [Claude Code Subagents Architecture](https://code.claude.com/docs/en/sub-agents)
  - [Claude Code Memory Systems](https://code.claude.com/docs/en/memory)
  - [Claude Code Hooks Guide](https://code.claude.com/docs/en/hooks-guide)
  - [Claude Agent SDK Overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- **Google Antigravity:**
  - [Google Antigravity Platform & Capabilities](https://antigravity.google/)

---

## 4. Open Protocols and Local-First Infrastructure

- **Model Context Protocol (MCP):**
  - [MCP Architecture Specification](https://modelcontextprotocol.io/docs/2026-07-28/learn/architecture)
  - [Official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **Agent-to-Agent Protocol (A2A):**
  - [A2A 1.0 Specification](https://a2a-protocol.org/latest/specification/)
- **Local Storage and Indexing:**
  - [SQLite Write-Ahead Logging (WAL)](https://sqlite.org/wal.html)
  - [SQLite FTS5 Extension](https://sqlite.org/fts5.html)
- **Codebase Analysis and Isolation:**
  - [Git Worktree Documentation](https://git-scm.com/docs/git-worktree)
  - [Tree-sitter Parser Generator](https://tree-sitter.github.io/tree-sitter/)
  - [ast-grep Structural Search & Replace](https://ast-grep.github.io/)
  - [ripgrep Line-Oriented Regex Search](https://github.com/BurntSushi/ripgrep)

---

## 5. Academic Research: Cost Optimization and Human-AI Systems

- **Model Routing and Optimization:**
  - Chen, Lingjiao et al. (2023). *FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance.* [arXiv:2305.05176](https://arxiv.org/abs/2305.05176).
  - Ong, Isaac et al. (2024). *RouteLLM: Learning to Route LLMs with Preference Data.* [arXiv:2406.18665](https://arxiv.org/abs/2406.18665).
- **Selective Prediction and Cognitive Escalation:**
  - Mozannar, Hussein & Sontag, David (2021). *Consistent Estimators for Learning to Defer to an Expert.* [arXiv:2112.06751](https://arxiv.org/abs/2112.06751).
- **Agentic Security and Least Privilege:**
  - [OWASP Top 10 for Large Language Model Applications](https://genai.owasp.org/)
  - Saltzer, J. H., & Schroeder, M. D. (1975). *The protection of information in computer systems.* (Principles of least privilege and fail-safe defaults).
