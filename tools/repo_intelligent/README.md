# Custos Nexus (Repo Intelligence)

Custos Nexus is the repository intelligence utility maintained with Custos. It indexes source structure, symbols, imports, call references and searchable chunks into a local SQLite database (`nexus_index.db`).

Nexus acts as the **Context & Memory Layer** for Custos agents (including the Goose-derived engine), providing AST analysis, call-graph tracking, and contextual repository maps.

## Capabilities

1. **AST & Call Graph Indexing**: Parses Rust and Python ASTs using Tree-sitter, extracting classes, structs, traits, functions, signatures, docstrings, and call edges.
2. **Deep Repo Map**: Generates compressed, token-budget friendly architectural maps with AST symbol signatures and subpath filtering.
3. **Model Context Protocol (MCP)**: Exposes 10 native tools over stdio for any agent (Custos, Goose, Claude Desktop, Cursor).

## Quickstart

```bash
# In tools/repo_intelligent:
uv sync
uv run nexus index --repo custos
uv run nexus stats
```

### CLI Commands

```bash
# 1. Generate Deep AST Map (entire repo or filtered by subpath)
uv run nexus map -r custos
uv run nexus map -p crates/runtime/custos-engine -d 6 -m 8

# 2. Inbound and Outbound Call Graphs
uv run nexus callers "promote" -r custos
uv run nexus callees "select_best" -r custos

# 3. Execution Flow Path
uv run nexus flow "promote" "execute_create" -r custos

# 4. Symbol & File Deep Dive
uv run nexus symbol "ReasoningTier" -r custos
uv run nexus file "crates/core/custos-bridge/src/service.rs" -r custos

# 5. Full-text & AST Search
uv run nexus search "human_gate" -r custos
```

## MCP Server Integration

A root `.mcp.json` is provided in the repository root. Any MCP-compatible agent can interact directly with Nexus:

```bash
uv run nexus mcp
```

### Supported MCP Tools:
- `nexus_map`: AST-powered Deep Repo Map with optional `path_prefix`, `max_depth`, and `max_symbols_per_file`.
- `nexus_overview`: High-level architectural overview, crate topology, and entry points.
- `nexus_symbol`: Deep 360-degree inspection of symbols (code, signature, callers, callees).
- `nexus_callers`: Trace all inbound callers of a function.
- `nexus_callees`: Trace all outbound functions called within a body.
- `nexus_flow`: Find shortest call path between two functions.
- `nexus_file`: Breakdown of all definitions inside a specific file.
- `nexus_crate`: Crate dependencies and exported symbols.
- `nexus_search`: AST & FTS5 full-text search across symbols and docstrings.
- `nexus_read`: Line-accurate source viewer with line numbers.
