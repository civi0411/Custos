# Provider and adapter boundaries

This document defines the implementation boundary between `custos-provider` and
`custos-adapters`. It is a convergence contract for the current codebase, not a
claim that every listed integration is production-ready.

## 1. Canonical ownership

| Concern | Canonical owner | Rule |
|---|---|---|
| One model inference attempt | `custos-provider` | `ModelPort` and structured turn values describe input, streamed output, usage and attribution. |
| Rich conversation representation | `custos-provider` | Preserve text, images, reasoning, tool requests and tool results without flattening them into a prompt string. |
| Model metadata and cost calculation | `custos-provider` | Metadata is versioned and source-labelled. A bundled catalog is a baseline, not live availability or billing truth. |
| Vendor wire conversion | `custos-provider` while pure | Pure encode/decode may remain here. HTTP, credential and filesystem access do not belong here. |
| Provider connection and transport | `custos-adapters/src/providers` | Own endpoints, authentication, HTTP/WebSocket clients, retries and provider-specific streaming. |
| Local inference | `custos-adapters/src/local_inference` | Implements the same model-attempt contract as a remote provider; it does not own an agent loop. |
| Native agent harness | `custos-adapters/src/harness` | Codex, Claude Code, Goose and OpenCode retain their own loop and report honest mediation capabilities through `AgentRuntimePort`. |
| MCP and A2A wire protocols | `custos-adapters/src/mcp`, `a2a`, `roaming` | Translate untrusted external protocol data. They do not grant authority or complete Tasks. |
| File, process, sandbox and worktree effects | `custos-adapters/src/sandbox`, `workspace.rs` | Execute only after the trusted layer has created the required permit or dispatch claim. |
| Routing policy | `custos-runtime` and `custos-core` | User pin, privacy, budget and capability constraints are filtered before an adapter is selected. |
| Concrete composition | `custos-daemon` | The daemon is the only production composition root. |

## 2. Current interface convergence

The repository currently contains two model interfaces:

1. `port.rs::ModelProvider` (`ModelPort`) with a legacy flat
   `ProviderRequest` and a structured `execute_turn` entry point.
2. `types/base.rs::Provider`, retained from the richer provider stack, with
   structured conversation, tools, reasoning, streaming and detailed usage.

The target is one structured model-turn contract. `ModelTurnRequest` and
`ModelTurnEvent` are the convergence vocabulary. The richer `Provider` stack is
not dead code: it contains the fidelity needed to implement that contract. The
flat `generate` path remains a compatibility path until the daemon, runtime and
all required adapters use structured turns.

The compatibility bridge must fail closed when flattening would lose tools,
images, reasoning state or other non-text content. It must never silently claim
native streaming when it only emits a terminal response after unary generation.

## 3. Adapter implementation status

| Area | Status | Interpretation |
|---|---|---|
| `providers/router.rs` | Composed compatibility path | The daemon uses it for configured remote and local model API connections. It is not the final routing-policy owner. |
| `providers/providers/*` | Rich implementation family | Retain and bridge incrementally to structured `ModelPort`; do not duplicate new vendor behavior in the flat router indefinitely. |
| `providers/translator/*` | Experimental structured converters | Has unit fixtures but is not the production transport path. Compare against the rich format implementations before selecting one converter per vendor. |
| `providers/{claude,codex,local_model}` | Compatibility wrappers or fail-closed placeholders | Must not be advertised as live merely because their Rust type exists. Native Codex belongs to `harness/codex.rs`. |
| `providers/fallback.rs` | Implemented but not composed | Use only after typed failure classification and partial-stream replay rules are tested. |
| `providers/catalog.rs` and `providers/pricing.rs` | Current desktop projection | Transitional. Converge with provider-owned metadata and cost semantics; do not keep two literal pricing tables. |
| `local_inference/*` | Rich local backend | Retain. Bridge it to the same structured turn contract rather than creating a local-agent abstraction. |
| `harness/*` | Composed native harness registry | Provider-governed unless interception is proven for the concrete execution path. |
| `mcp/adapters/client.rs` | Mock-capable scaffold | Current `call_tool` behavior is not a production remote MCP dispatch. |
| `mcp/adapters/gateway_wrap.rs` | Fail-closed mediation scaffold | Correctly refuses effect dispatch without a durable claim; it is not yet a complete MCP capability path. |
| `a2a/mod.rs` | Protocol values and dispatch helpers | Not an end-to-end remote agent transport. |
| `roaming/*` | Peer transport | Distinct from A2A task semantics; `roaming/a2a.rs` currently simulates delegation. |
| `sandbox/seatbelt`, `sandbox/bubblewrap` | Placeholders | Their existence is not evidence of OS isolation. |

## 4. Catalog, connection and attempt identities

The following objects must remain distinct:

- `ProviderConnection`: endpoint, authentication reference, account identity and
  transport configuration.
- `ModelDescriptor`: model identity and advertised capabilities from a
  source-labelled catalog.
- `ModelAttempt`: one bounded request with requested and actual route, stream,
  usage certainty and terminal state.
- `AgentHarnessSession`: a native agent lifecycle with its own continuation,
  tool mediation and cost visibility.

The bundled canonical registry is the fallback metadata baseline. A live probe
is evidence that an endpoint advertised a model at a recorded time. An exercised
conformance fixture is evidence for a capability. A provider receipt or bill is
the strongest source for actual cost. None of these sources replaces the others.

## 5. Convergence sequence

1. Keep the daemon on one compatibility model route and prevent placeholder
   providers from being selected as production implementations.
2. Expand conformance fixtures for text, tools, multimodal input, reasoning,
   usage, cancellation and interrupted streams.
3. Bridge one rich provider implementation to `ModelTurnRequest/Event` without
   losing provider-specific semantics.
4. Move local inference through the same bridge.
5. Replace duplicate literal pricing with one source-labelled rate projection
   and one pure cost calculator.
6. Compose fallback only after typed error and replay-safety tests pass.
7. Narrow wildcard compatibility exports after all callers migrate.
8. Remove a compatibility implementation only after call-site inventory,
   fixtures and product-path tests prove it is no longer required.

## 6. Acceptance gates

A provider adapter is not `live` until it proves request translation, real
stream ordering, tool-call assembly, actual model attribution, usage certainty,
cancellation behavior and error mapping. A native harness is not
`custos-mediated` unless Custos intercepts every relevant effect before dispatch.
An MCP or A2A module is not `complete` merely because its DTOs serialize.


### Provider credential secure storage

Provider API keys and OAuth access/refresh tokens are daemon-owned secrets. The core defines an internal credential-vault contract; adapters implement the existing OS keyring, persistence owns metadata and recoverable operations, and daemon composes the service. Desktop remains a Local API client. No secret may be written to new SQLite records, outbox, public responses or frontend storage. Client Local API keys are excluded.

A database-stable UUID namespaces immutable per-account credential generations. Long bundles use 1024-character ASCII keyring segments and a final manifest with binding, length and SHA-256; the JSON limit is 256 KiB. Keyring calls run through spawn_blocking outside database locks. Credential metadata transactions temporarily use SQLite synchronous FULL before opening the transaction and restore the prior writer setting afterwards. Save verifies the new bundle before an atomic metadata/config/outbox commit; cleanup of the previous generation follows commit. Durable pending operations support crash recovery and generation checks. Delete deactivates credentials before keyring cleanup. Locked/unavailable/corrupt stores fail closed without plaintext or cross-account fallback.

Legacy provider tokens migrate automatically and idempotently only after keyring read-back verification. Secure deletion and WAL truncation are tracked; existing backups and physical disk erasure are outside that guarantee. Keyring failure retains legacy data for retry but prevents provider use. Explicit environment credentials remain supported when no stored account exists. OAuth callbacks, refresh, probes and inference share the same service. UI closes only after successful commit, clears entered secrets on close/success and receives metadata only.
### Desktop model connection attribution

The existing ModelSelector preserves the Copilot route namespace before model-family inference. Claude, GPT and Gemini names served through Copilot belong to the GitHub Copilot group. Copilot is included in both display ordering and provider filters; model IDs sent to the daemon retain their original route namespace.
