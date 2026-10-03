# Upstream Source Inventory

> **Status:** Audit in progress. This inventory records observations in the Custos checkout, not a license-compliance conclusion. Do not claim upstream SHA pinning or production integration until the missing evidence is filled in.

## Goose-derived source

| Custos path | Observed content | Runtime status | Upstream revision / attribution |
|---|---|---|---|
| `crates/custos-runtime/src/engine/agents/` | Agent loop, state machine, extension manager, MCP client and platform tools with Goose identifiers | Present in the repository; `engine` is not mounted from `runtime/src/lib.rs` | Exact Goose source SHA and file-level mapping **unknown** |
| `crates/custos-runtime/src/agent/` | Extracted machine, inference, tool and conversation primitives | Compiled module; no daemon-composed `StateMachine` call path found at audit commit `3e4dac4` | File-level origin and divergence **unverified** |
| `crates/custos-provider/src/types/` | Goose-style provider, conversation and model types coexist with `ModelProvider` | Compiled; two model interfaces require a conformance mapping | File-level origin and divergence **unverified** |
| `crates/custos-sdk/` | Bindings and wire types retain Goose identifiers | Present; runtime usage not established by this inventory | File-level origin and divergence **unverified** |

[Goose's upstream license](https://github.com/aaif-goose/goose/blob/main/LICENSE) is Apache 2.0. Custos's root `LICENSE` currently contains MIT text, while earlier README translations described Apache 2.0. This mismatch requires a human-maintained provenance and license review before a release claim; this document does not choose a license for the project.

## External gateway candidates, not imported Custos code

| Upstream | Narrow reason to inspect | Custos integration boundary | Current audit state |
|---|---|---|---|
| [decolua/9router](https://github.com/decolua/9router/blob/master/docs/ARCHITECTURE.md) | Provider/account routing, request/stream translation, fallback and usage | Optional `ModelPort` proxy profile through `custos-adapters/src/providers/`; never Task/Authority storage | Candidate only; exact SHA, license, ToS/auth handling, stream/tool/usage fidelity and latency unverified |
| [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway/blob/main/README.md) | MCP/A2A federation, model proxying and connection governance | Optional supervised protocol gateway/sidecar beyond the existing direct MCP client | Candidate only; exact SHA, license, protocol revisions, identity/secret boundaries and bypass risk unverified |
| [rsclaw-ai/cap-protocol](https://cap-protocol.org/) | Draft PTY/structured driver for CLI agents | Optional `AgentRuntimePort` adapter, not Custos's internal protocol | Watchlist; no CAP conformance or first-party manifest established for this checkout |

Before copying source or launching a sidecar, pin upstream repository and commit, inspect license/notices/dependencies, document one real user job, compare against a direct adapter on the same fixture, and verify that no proxy can claim Custos-mediated effects it cannot intercept. A vendor/provider marketing claim is not proof of Custos integration.

## Required audit fields per reused module

Record upstream repository URL and commit SHA, original path, Custos path, whether copied or substantially modified, license and notice requirements, active call path, test coverage, and whether its tools can bypass Custos authorization. Classify each module `active`, `compiled-unwired`, `dormant`, or `duplicate`. A path name or comment alone is not proof of provenance or assurance.

## Integration gates

1. Choose one internal agent loop using replay fixtures and provider/tool parity tests; do not mount the entire legacy engine merely because the files exist.
2. Keep source attribution and file-level provenance with the retained code.
3. Verify each physical effect path independently. Goose CLI, Goose-derived in-process code, and a model endpoint have different loop ownership and assurance.
4. Update this inventory when upstream revisions or Custos call paths change.
