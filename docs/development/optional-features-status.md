# Optional Features Status Ledger

> **Classification:** Normative Feature Matrix Specification  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

This ledger documents all Cargo feature flags across the workspace packages, their default states, offline CI compatibility, and architectural constraints.

| Package | Physical Path | Feature Flag | Default | Offline CI Status | Architectural Purpose & Invariant |
|---|---|---|---|---|---|
| `custos-adapters` | `platform/adapters` | `local-inference-native` | Yes | Supported | Canonical local GGUF / llama.cpp inference engine via Metal/CUDA. |
| `custos-adapters` | `platform/adapters` | `local-inference-mlx` | No | Supported (stub) | Apple Silicon MLX inference backend; returns typed error when dynamic libs absent. |
| `custos-adapters` | `platform/adapters` | `local-inference-hf` | No | Unsupported offline | HuggingFace Hub remote weight downloader; gated to prevent offline build failures. |
| `custos-adapters` | `platform/adapters` | `roaming` | No | Supported | Iroh P2P transport relay for multi-node encrypted synchronization. |
| `custos-adapters` | `platform/adapters` | `mcp-builtins` | No | Supported | In-process MCP server bundle (isolates large JS assets from default desktop binary). |
| `custos-app-daemon` | `apps/daemon` | `custos-workbench-assistant` | Yes | Supported | Compiles personal assistant outbox and message drafts handlers. |
| `custos-app-daemon` | `apps/daemon` | `custos-workbench-coding` | Yes | Supported | Compiles software engineering worktree and patch handlers. |
| `custos-app-daemon` | `apps/daemon` | `custos-workbench-research` | Yes | Supported | Compiles academic literature citation and recipe synthesis handlers. |
| `custos-app-daemon` | `apps/daemon` | `custos-workbench-quant` | Yes | Supported | Compiles quantitative backtest, risk gate, and order ticket handlers. |
| `custos-sdk` | `sdk/client-rust` | `uniffi` | No | Unsupported offline | Multi-language FFI bindings (Python, Swift, Kotlin); requires external codegen tools. |
