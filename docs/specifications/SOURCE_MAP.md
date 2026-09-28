# Source Extraction Map

> **Research snapshot:** Paths and source counts below must be revalidated against a pinned checkout before use in an implementation or license decision. See the [research register](../research/README.md).

## 1. Extraction Semantics

| Decision | Meaning |
|---|---|
| `KEEP` | copy from legacy Custos with only namespace and packaging changes |
| `ADAPT` | retain behavior and tests; reshape around new ports |
| `PORT` | reimplement observed behavior from Goose with Custos-native contracts |
| `REWRITE` | preserve requirements, replace implementation |
| `REJECT` | do not import into Custos New |

Every copied file requires original path, revision, license, modifications, and a characterization test in `third_party/provenance.toml`.

## 2. Legacy Custos Components

| Source | Decision | Destination | Required proof |
|---|---|---|---|
| `crates/core-domain` | ADAPT | `crates/domain` | pure dependency check; state tests |
| `crates/task-kernel` | KEEP/ADAPT | `crates/task-kernel` | CQRS and stale-epoch tests |
| `crates/persistence-sqlite` | ADAPT | `adapters/persistence-sqlite` | migration and crash tests |
| `crates/authority-engine` | ADAPT | `crates/authority` | revoked permit and payload binding |
| `crates/capability-gateway` | ADAPT | `crates/capability-gateway` | no-permit causes zero effects |
| `crates/context-compiler` | ADAPT | `crates/context-engine` | token budget and provenance |
| `crates/repo-intelligence` | KEEP/ADAPT | context adapter/module | fixture accuracy benchmark |
| `crates/evidence-engine` | ADAPT | `crates/evidence` | stale and unsupported evidence |
| `crates/provider-sdk` | REWRITE | `crates/provider-port` | capability and stream conformance |
| `crates/cognitive-runtime` | ADAPT | `crates/cognitive-runtime` | S1/S2 route evaluation |
| `crates/workflow-runtime` | REWRITE | `crates/workflow-runtime` | lifecycle, lease, cancellation |
| `apps/custosd` | REWRITE | `apps/custosd` | restart and reconciliation |
| `apps/custos-cli` | ADAPT | `apps/custos-cli` | API-only mutation path |
| existing domain packs | ADAPT later | `packs/` | schema and workflow conformance |

## 3. Goose Components at `9adae14b`

| Goose source | Decision | Custos New destination | Imported idea |
|---|---|---|---|
| `goose-agent/src/machine.rs` | PORT | `workflow-runtime/worker_machine` | ordered re-entrant steps |
| `goose-agent/src/operation.rs` | PORT | `workflow-runtime/operation` | applicable/applied/yielded outcome |
| `goose-agent/src/inference.rs` | PORT | `cognitive-runtime/worker_inference` | streaming, cancellation, retry signals |
| `goose-agent/src/tool.rs` | PORT | `workflow-runtime/tool_operation` | tool-call/result lifecycle |
| `goose/src/agents/state_machine/*` | CHARACTERIZE | operation modules | ordering and recovery behavior |
| `agents/extension_manager/stdio.rs` | ADAPT/PORT | `adapters/protocols/mcp` | MCP child-process lifecycle |
| `extension_manager/streamable_http.rs` | ADAPT/PORT | MCP HTTP adapter | sessions, reconnect, auth |
| `agents/mcp_client.rs` | PORT | `crates/protocol-port` | tool/resource/prompt surface |
| `goose-provider-types/src/base.rs` | REFERENCE | provider-port tests | message/tool usage semantics |
| `goose-providers/src/*` | REFERENCE | provider HTTP adapters | wire-format fixtures |
| `context_mgmt/*` and `goose-context-management` | PORT | `context-engine` | compaction lifecycle |
| `session/session_manager.rs` | CHARACTERIZE | persistence tests | durable resume semantics |
| `config/permission.rs` | REFERENCE | authority tests | UX and policy precedence |
| `platform_extensions/developer/*` | PORT selectively | tool adapters | edit/shell/tree behavior |
| `platform_extensions/analyze/*` | BENCHMARK | repo intelligence | parser/graph comparison |
| `summon.rs` | REWRITE | bounded worker spawning | scoped child execution |
| `orchestrator.rs` | REWRITE | workflow scheduler | coordination behaviors |
| Goose UI, branding, roaming | REJECT | none | outside Custos identity |

The earlier proposed `goose-mcp/src/stdio_client.rs`, `sse_client.rs`, and `streamable_http.rs` paths are invalid at this revision and must not be used.

## 4. Source Isolation

- `third_party/goose/` is used only when exact source copying is justified.
- Ported code carries behavior provenance, not Goose public types.
- Reference-only source is never copied into the shipping workspace.
- Apache-2.0 notices remain attached to copied or substantially derived code.
- No dependency points from Custos New to `../goose` or legacy `../Custos`.
