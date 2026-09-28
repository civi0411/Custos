# Custos Nexus

Custos Nexus is the repository intelligence utility maintained with Custos. It indexes source structure, symbols, imports, call references and searchable chunks into a local SQLite database. A sibling Goose checkout can be indexed as an optional upstream reference, but is not required to inspect Custos.

Nexus supports source-backed architecture investigation and cross-reference analysis. Its findings are navigation evidence, not proof that a runtime path is wired or correct.

## Capabilities

1. Repository ingestion parses Rust and Python ASTs with tree-sitter and indexes other supported text formats.
2. The local index stores files, symbols, imports, call references and FTS5 data in `nexus_index.db` beside this README.
3. CLI and MCP surfaces provide overview, symbol, caller, callee, flow and file inspection.

## Setup

```bash
uv sync
uv run nexus index --repo custos
uv run nexus stats
uv run nexus --help
```

Set `CUSTOS_REPO_ROOT`, `CUSTOS_NEXUS_DB`, or `CUSTOS_NEXUS_EVIDENCE_DB` only when an alternate checkout or database location is required. Defaults are derived from this tool's location; no developer-specific absolute path is required.
