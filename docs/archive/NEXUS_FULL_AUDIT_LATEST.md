# Nexus Full Architecture Audit

Generated deterministically from the active source indexes.

## Custos inventory

- Indexed files: 2,196
- Source/document files: 2,016
- Source/document lines: 790,490
- AST symbols: 9,815
- Cargo packages: 41
- Dependency cycles: 0

### Package topology

| Package | Lines | Symbols | Dependencies | Dependents | Orphan |
|---|---:|---:|---:|---:|---|
| `custos-adapters-mcp` | 100 | 8 | 2 | 1 | no |
| `custos-download-manager` | 718 | 26 | 0 | 2 | no |
| `custos-local-inference` | 10,634 | 537 | 3 | 1 | no |
| `custos-mcp` | 11,785 | 264 | 0 | 2 | no |
| `custos-providers` | 18,540 | 843 | 2 | 3 | no |
| `custos-roaming` | 3,238 | 154 | 0 | 0 | yes |
| `custos-judgment-contracts` | 25 | 2 | 0 | 3 | no |
| `custos-adapter-judgment-jev` | 41 | 4 | 1 | 0 | yes |
| `custos-adapter-judgment-onnx` | 44 | 4 | 1 | 0 | yes |
| `custos-adapter-judgment-rules` | 44 | 4 | 1 | 0 | yes |
| `custos-adapter-provider-antigravity` | 53 | 5 | 1 | 0 | yes |
| `custos-adapter-provider-claude` | 53 | 5 | 1 | 0 | yes |
| `custos-adapter-provider-codex` | 53 | 5 | 1 | 0 | yes |
| `custos-adapter-provider-fake` | 163 | 9 | 0 | 3 | no |
| `custos-adapter-provider-local-model` | 62 | 6 | 1 | 0 | yes |
| `custos-adapter-sandbox-linux-bubblewrap` | 23 | 3 | 0 | 0 | yes |
| `custos-adapter-sandbox-macos-seatbelt` | 23 | 3 | 0 | 0 | yes |
| `custos-cli` | 2,642 | 105 | 6 | 0 | yes |
| `custos-daemon` | 391 | 25 | 8 | 0 | yes |
| `custos-local-api` | 72 | 6 | 0 | 0 | yes |
| `custos-bridge` | 285 | 10 | 3 | 2 | no |
| `custos-domain` | 1,646 | 116 | 0 | 2 | no |
| `custos-kernel` | 1,022 | 64 | 1 | 5 | no |
| `custos-provider-sdk` | 325 | 20 | 0 | 10 | no |
| `custos-provider-types` | 276,355 | 1,308 | 0 | 4 | no |
| `custos-sdk-types` | 3,300 | 294 | 0 | 3 | no |
| `custos-persistence` | 1,157 | 59 | 2 | 4 | no |
| `custos-packs-assistant` | 89 | 7 | 0 | 0 | yes |
| `custos-packs-engineering` | 312 | 7 | 0 | 0 | yes |
| `custos-packs-research` | 2,792 | 90 | 0 | 0 | yes |
| `custos-agent` | 2,253 | 158 | 1 | 2 | no |
| `custos-cognitive` | 323 | 23 | 1 | 1 | no |
| `custos-context` | 2,621 | 142 | 0 | 1 | no |
| `custos-context-management` | 1,388 | 68 | 1 | 2 | no |
| `custos-memory-service` | 30 | 2 | 0 | 0 | yes |
| `custos-security` | 1,366 | 84 | 0 | 2 | no |
| `custos-session` | 302 | 17 | 2 | 3 | no |
| `custos-workflow` | 1,029 | 48 | 2 | 1 | no |
| `custos-tests-contract` | 95 | 2 | 2 | 0 | no |
| `custos-tests-e2e` | 497 | 2 | 5 | 0 | no |
| `xtask` | 45 | 3 | 0 | 0 | no |

### Largest files

| File | Lines |
|---|---:|
| `crates/core/custos-provider-types/src/canonical/data/canonical_models.json` | 238,114 |
| `docs/goose/package-lock.json` | 19,722 |
| `ui/pnpm-lock.yaml` | 13,767 |
| `crates/runtime/custos-engine/acp-schema.json` | 9,012 |
| `crates/core/custos-provider-types/src/formats/openai.rs` | 5,788 |
| `crates/core/custos-provider-types/src/canonical/data/canonical_mapping_report.json` | 5,273 |
| `ui/desktop/src/i18n/messages/es.json` | 4,776 |
| `ui/desktop/src/i18n/messages/fr.json` | 4,776 |
| `ui/desktop/src/i18n/messages/hi.json` | 4,776 |
| `ui/desktop/src/i18n/messages/id.json` | 4,776 |
| `ui/desktop/src/i18n/messages/it.json` | 4,776 |
| `ui/desktop/src/i18n/messages/ja.json` | 4,776 |
| `ui/desktop/src/i18n/messages/ko.json` | 4,776 |
| `ui/desktop/src/i18n/messages/ms.json` | 4,776 |
| `ui/desktop/src/i18n/messages/pt.json` | 4,776 |

## Goose inventory

- Indexed files: 2,182
- Source/document files: 1,974
- Source/document lines: 998,341
- AST symbols: 13,817
- Cargo packages: 16
- Dependency cycles: 0

### Package topology

| Package | Lines | Symbols | Dependencies | Dependents | Orphan |
|---|---:|---:|---:|---:|---|
| `goose` | 318,967 | 8,663 | 8 | 2 | no |
| `goose-acp-macros` | 338 | 7 | 0 | 1 | no |
| `goose-agent` | 2,253 | 158 | 1 | 1 | no |
| `goose-cli` | 33,688 | 1,053 | 4 | 0 | yes |
| `goose-context-management` | 1,388 | 68 | 1 | 2 | no |
| `goose-download-manager` | 718 | 26 | 0 | 2 | no |
| `goose-local-inference` | 10,634 | 537 | 3 | 1 | no |
| `goose-mcp` | 11,785 | 264 | 0 | 2 | no |
| `goose-provider-types` | 276,345 | 1,307 | 0 | 4 | no |
| `goose-providers` | 18,541 | 843 | 2 | 3 | no |
| `goose-roaming` | 3,238 | 154 | 0 | 1 | no |
| `goose-sdk` | 3,643 | 191 | 3 | 0 | yes |
| `goose-sdk-types` | 3,300 | 294 | 0 | 3 | no |
| `goose-test` | 278 | 13 | 0 | 0 | yes |
| `goose-test-support` | 292 | 25 | 0 | 1 | no |
| `v8` | 16 | 0 | 1 | 0 | no |

### Largest files

| File | Lines |
|---|---:|
| `crates/goose-provider-types/src/canonical/data/canonical_models.json` | 238,114 |
| `crates/goose/src/dictation/whisper_data/tokens.json` | 114,853 |
| `documentation/package-lock.json` | 19,722 |
| `ui/pnpm-lock.yaml` | 13,767 |
| `crates/goose/acp-schema.json` | 9,012 |
| `crates/goose/src/agents/agent.rs` | 6,161 |
| `crates/goose-provider-types/src/formats/openai.rs` | 5,788 |
| `crates/goose-provider-types/src/canonical/data/canonical_mapping_report.json` | 5,273 |
| `crates/goose/src/session/session_manager.rs` | 5,025 |
| `ui/desktop/src/i18n/messages/es.json` | 4,776 |
| `ui/desktop/src/i18n/messages/fr.json` | 4,776 |
| `ui/desktop/src/i18n/messages/hi.json` | 4,776 |
| `ui/desktop/src/i18n/messages/id.json` | 4,776 |
| `ui/desktop/src/i18n/messages/it.json` | 4,776 |
| `ui/desktop/src/i18n/messages/ja.json` | 4,776 |

## Goose capability transfer matrix

| Capability | Decision | Target | Goose | Native Custos | Imported Goose in Custos |
|---|---|---|---:|---:|---:|
| `reentrant_operation_machine` | **PORT** | `custos-workflow/worker_machine` | 2 | 1 | 0 |
| `inference_tool_loop` | **PORT** | `custos-workflow/operations` | 2 | 6 | 0 |
| `context_compaction` | **ADAPT** | `custos-context/compaction` | 12 | 9 | 1 |
| `mcp_stdio` | **PORT** | `custos-adapters-mcp` | 1 | 12 | 1 |
| `mcp_streamable_http` | **PORT** | `custos-adapters-mcp` | 1 | 12 | 1 |
| `tool_approval_reconstruction` | **ADAPT** | `custos-security + custos-workflow` | 2 | 12 | 2 |
| `bounded_retry` | **ADAPT** | `workflow-runtime/operations` | 2 | 3 | 2 |
| `steering_and_cancellation` | **PORT** | `custos-workflow/operations` | 2 | 0 | 5 |
| `developer_tools` | **CHARACTERIZE** | `adapters/tools` | 6 | 0 | 6 |
| `provider_wire_formats` | **REFERENCE** | `adapters/providers/http` | 12 | 10 | 0 |
| `acp_agent_runtime` | **PORT** | `adapters/providers/agent` | 2 | 10 | 0 |
| `recipes_and_skills` | **ADAPT** | `packs + application workflow compiler` | 12 | 0 | 12 |
| `subagent_execution` | **REWRITE** | `workflow-runtime/scheduler` | 3 | 0 | 2 |
| `session_storage` | **REFERENCE** | `custos-session + custos-persistence` | 1 | 12 | 0 |

## Exact source-copy footprint

- Shared content hashes: 1,426
- Custos source/document files exactly matching Goose: 1,437
- Matching bytes inside Custos: 23,208,082

This metric detects exact content reuse; it does not prove architectural integration or runtime reachability.

## Custos structural findings

### Orphan packages
- `custos-adapter-judgment-jev`
- `custos-adapter-judgment-onnx`
- `custos-adapter-judgment-rules`
- `custos-adapter-provider-antigravity`
- `custos-adapter-provider-claude`
- `custos-adapter-provider-codex`
- `custos-adapter-provider-local-model`
- `custos-adapter-sandbox-linux-bubblewrap`
- `custos-adapter-sandbox-macos-seatbelt`
- `custos-cli`
- `custos-daemon`
- `custos-local-api`
- `custos-memory-service`
- `custos-packs-assistant`
- `custos-packs-engineering`
- `custos-packs-research`
- `custos-roaming`

### Consolidation recommendations
- Merge wire-only judgment and deliberation contracts into explicit cognitive ports unless independent versioning is proven.
- Keep domain contracts cohesive instead of splitting one entity across many micro-crates.
- Replace workflow-runtime scaffolding with the re-entrant operation machine before adding schedulers.
- Move SQLite, provider, protocol, and OS concerns under adapters; keep application use cases independent.
- Treat packs as declarative products and MCP/ACP as edge protocols, never as the internal bus.

## Machine-readable companion

Use the JSON output from the same audit run for file-level agent planning and regression comparison.
