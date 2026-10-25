# Optional Features Status Ledger

| Crate | Feature | Default | Status | Offline CI Gate | Notes |
|---|---|---|---|---|---|
| `custos-adapters` | `local-inference-native` | Yes | Supported | Passes in default `cargo check` | Canonical GGUF / llama.cpp inference engine |
| `custos-adapters` | `local-inference-mlx` | No | Supported (stub) | Passes in `--features local-inference-mlx` | Fallback stub returns typed unsupported error when `safemlx` is not vendor-bundled |
| `custos-adapters` | `local-inference-hf` | No | Unsupported offline | Explicit `compile_error!` | Requires external `hf-hub` client; gated to avoid accidental offline build breakage |
| `custos-adapters` | `roaming` | No | Supported | Passes in `--features roaming` | Iroh & Iroh-relay peer-to-peer transport |
| `custos-adapters` | `mcp-builtins` | No | Supported | Passes in `--features mcp-builtins` | Autovisualiser & Memory MCP built-ins; isolates large 3.3MB JS assets from default desktop build |
| `custos-sdk` | `uniffi` | No | Unsupported offline | Explicit error (`askama` download / `compile_error!`) | Python/Kotlin uniffi bindings require askama CLI templating |
