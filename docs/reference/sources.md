# Nguồn Tài Liệu Kỹ Thuật & Nghiên Cứu Nền Tảng (Sources & Citations)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần VI (§44) Canonical Specification

Kiến trúc của Custos được xây dựng dựa trên các tiêu chuẩn công nghiệp đã được kiểm chứng và các nghiên cứu khoa học hàng đầu trong các lĩnh vực: Software Architecture, Cognitive Science, Program Synthesis, Capability Security và Local-first Software.

---

## 1. Tiêu Chuẩn Kiến Trúc & Hợp Đồng Dữ Liệu

- **arc42 Documentation Template:** [arc42 Overview & Structure](https://arc42.org/overview/) — Khuôn mẫu chuẩn cho tài liệu hóa kiến trúc phần mềm.
- **The C4 Model:** [C4 Software Architecture Model](https://c4model.com/) — Mô hình trực quan hóa kiến trúc theo 4 cấp độ (Context, Containers, Components, Code).
- **Architectural Decision Records (ADR):** [ADR GitHub Organization](https://adr.github.io/) — Quy chuẩn ghi lại các quyết định kỹ thuật trọng yếu.
- **OpenAPI Specification:** [OpenAPI 3.1 Standard](https://spec.openapis.org/oas/latest.html) — Chuẩn mô tả API contracts.

---

## 2. Hạ Tầng Nhận Thức, System One & Fast Judgment

- **TypeSafe AI / Jev Documentation:**
  - [TypeSafe Introduction](https://docs.typesafe.ai/introduction)
  - [TypeSafe Primitives](https://docs.typesafe.ai/primitives)
  - [How to Build with System One](https://docs.typesafe.ai/concepts/how-to-build-with-system-one)
  - [Confidence Calibration](https://docs.typesafe.ai/confidence)
  - [System One Architectural Patterns](https://docs.typesafe.ai/patterns)
  - [Model Jaggedness & Jev 1.13 Evaluation](https://docs.typesafe.ai/model-jaggedness/jev-1.13)
- **Kahneman, Daniel (2011):** *Thinking, Fast and Slow* — Cơ sở lý thuyết cho việc phân tách System One (nhanh, bản năng, phán đoán) và System Two (chậm, suy luận, tính toán).

---

## 3. Nền Tảng AI Agent & SDK Nhà Cung Cấp

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

## 4. Giao Thức Mở & Hạ Tầng Local-First

- **Model Context Protocol (MCP):**
  - [MCP Architecture Specification](https://modelcontextprotocol.io/docs/2026-07-28/learn/architecture)
  - [Official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **Agent-to-Agent Protocol (A2A):**
  - [A2A 1.0 Specification](https://a2a-protocol.org/latest/specification/)
- **Lưu Trữ & Chỉ Mục Cục Bộ:**
  - [SQLite Write-Ahead Logging (WAL)](https://sqlite.org/wal.html)
  - [SQLite FTS5 Extension](https://sqlite.org/fts5.html)
- **Phân Tích Mã Nguồn & Cách Ly:**
  - [Git Worktree Documentation](https://git-scm.com/docs/git-worktree)
  - [Tree-sitter Parser Generator](https://tree-sitter.github.io/tree-sitter/)
  - [ast-grep Structural Search & Replace](https://ast-grep.github.io/)
  - [ripgrep Line-oriented Regex Search](https://github.com/BurntSushi/ripgrep)

---

## 5. Nghiên Cứu Khoa Học Về Tối Ưu Chi Phí & Tương Tác Người-Máy

- **Định Tuyến & Tối Ưu Hóa Mô Hình (Model Routing):**
  - Chen, Lingjiao et al. (2023). *FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance.* [arXiv:2305.05176](https://arxiv.org/abs/2305.05176).
  - Ong, Isaac et al. (2024). *RouteLLM: Learning to Route LLMs with Preference Data.* [arXiv:2406.18665](https://arxiv.org/abs/2406.18665).
- **Tương Tác Chọn Lọc & Leo Thang Phán Đoán (Selective Prediction & Escalation):**
  - Mozannar, Hussein & Sontag, David (2021). *Consistent Estimators for Learning to Defer to an Expert.* [arXiv:2112.06751](https://arxiv.org/abs/2112.06751).
- **Bảo Mật Hệ Thống AI Đa Tác Nhân (Agentic Security):**
  - [OWASP Top 10 for Large Language Model Applications](https://genai.owasp.org/)
  - Saltzer, J. H., & Schroeder, M. D. (1975). *The protection of information in computer systems.* (Nguyên tắc bảo mật quyền hạn tối thiểu).
