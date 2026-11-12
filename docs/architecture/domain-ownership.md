# Domain Ownership & Boundary Ledger

> Single Source of Truth (SSOT) defining domain type ownership, architectural boundaries, and SADE Multi-Workbench isolation for Custos.

---

## 1. Governance Rules

1. **Zero-Kernel Leakage**: Types defined in `platform/domain` MUST only represent concepts shared across >= 2 workbenches or fundamental to the kernel runtime loop (Tasks, Workspaces, Authority, Outbox, Artifacts, Execution Evidence, Compute, Fleet Observation).
2. **Domain Isolation**: Any concept specific to a single domain pack (e.g., personal assistant drafts, quantitative backtests/order execution, research claim synthesis) belongs strictly to its respective workbench crate (`workbenches/<name>`) or is governed via generic SPI traits (`DomainPack`, `WorkbenchModule`).
3. **Decoupled Persistence & Services**: Workbench-specific tables, migrations, DTOs, and pure application orchestration services reside in `workbenches/<name>/src/{domain, repository, services}/`. Core SQLite persistence maintains zero domain pack schema coupling, and `apps/daemon` remains a thin transport & composition layer.
4. **Ring 2 Shared Capability Neutrality**: Cross-workbench capabilities (`notebook`, `reviews`, `artifacts`, `notes`) are routed and advertised independently of whether the optional `research` domain pack is installed or enabled.

---

## 2. SADE 3-Ring Architecture Matrix

| Ring | Role | Canonical Owners | Examples |
|---|---|---|---|
| **Ring 1: Kernel & Platform** | Zero-I/O invariants, authority, storage, AI routing, runtime, adapters, packs | `platform/{domain, core, storage, ai, runtime, adapters, components}` | `Task`, `ExecutionWorkspace`, `Permit`, `StandingGrant`, `OutboxActionItem`, `RiskLevel` (`RiskClass`), `EnvironmentSpec` (AEE), `RecipeEnvironmentSpec`, `NotebookDocument`, `ArtifactEnvelope`, `ReviewerRecord` |
| **Ring 2: Shared Capabilities** | Cross-workbench tools usable by Coding, Research, Assistant, and Quant | `apps/daemon/src/application/research/mod.rs::handle_shared_capabilities`, `ui/desktop/src/api/shared_capabilities_client.ts` | `notebook.*`, `reviews.*`, `artifacts.*`, `notes.*` (decoupled from `require_pack_enabled("research")`) |
| **Ring 3: Sovereign Workbenches** | Pluggable vertical domains owning domain models, persistence & pure services | `workbenches/{assistant, coding, research, quant}`, `ui/desktop/src/components/workbenches/<name>/` | `AssistantDraft` & `AssistantDraftPort`, `SynthesisService`, `workbenches/quant/src/services/{BacktestService, MarketService, OrderExecutionService}`, `contribution.ts` |
| **Surface: Wire Protocol, SDK & Thin Hosts** | Compiled neutral wire protocol, SDK transport, daemon auth/composition, desktop & headless CLI | `sdk/{protocol, client-rust}`, `apps/daemon/src/{api/contexts, auth, application, bootstrap}`, `apps/desktop/native`, `apps/cli`, `ui/{desktop, cli}` | `custos-protocol` (`local_api` envelope/methods/params, `wire_types`, `worker`, `harness`, `a2a`, `research`), `LocalApiClient`, `LocalApiDispatcher` (`kernel`/`env`/`providers`/`workbenches`), `apps/daemon/src/auth/` |

---

## 3. Domain Types Disposition Matrix

| Domain Path / Symbol | Consumer(s) | Ownership Classification | Architectural Disposition |
|---|---|---|---|
| `platform/domain/src/authority/{action, outbox, permit, capability}.rs` | Kernel Core, Daemon, Workbenches | **Ring 1 (Authority & Outbox)** | Consolidated in `platform/domain`. Unified `RiskClass` alias onto `RiskLevel`, with `OutboxActionItem` and `StandingGrant` separated from assistant-specific models. |
| `platform/domain/src/fleet/compute/training.rs` | Compute Adapters, Runtime, Research | **Ring 1 (Compute)** | Unified `FrozenTrainingRecipe`, `TrainingExecutionReceipt`, `TrainingStepTelemetry` alongside compute target contracts. |
| `platform/domain/src/environment/{notebook,terminal}.rs` | Runtime, Core Ports, Workbenches | **Ring 1 (Execution Environments)** | Co-located notebook and terminal session domain models with AEE environment specs (`pub use environment::{notebook, terminal}` at `custos_domain` root). |
| `sdk/protocol/src/` (`custos-protocol`) | Daemon, Core, Adapters, Research Workbench, SDK, Tests | **Surface (Neutral Wire Protocol)** | Compiled protocol crate owning `local_api` schemas/envelopes/traits, `wire_types`, `worker`, `harness`, `a2a`, `profile`, and `research` wire DTOs so `custos-sdk` stays free of kernel/workbench dependencies. |
| `platform/domain/src/environment/` & `RecipeEnvironmentSpec` | Runtime, Daemon, Research | **Ring 1 (AEE / Recipe)** | Disambiguated AEE `EnvironmentSpec` from task recipe `RecipeEnvironmentSpec` so neither shadows the other at `custos_domain` root. |
| `workbenches/assistant/src/domain/draft.rs` | Assistant Workbench, Daemon | **Ring 3 (Assistant)** | Relocated `AssistantDraft` and `AssistantDraftPort` out of `platform/{domain, core}` into `custos-workbench-assistant`. |
| `workbenches/quant/src/services/` | Quant Workbench, Daemon | **Ring 3 (Quant)** | Owns `dto.rs`, `BacktestService`, `MarketService`, and `OrderExecutionService`; enforces Zero-Mock orderbooks requiring ingested OHLCV bars. |
| `workbenches/research/src/domain/` & `services/synthesis.rs` | Research Workbench, Daemon | **Ring 3 (Research)** | Owns claim grounding (`ResearchClaim`), synthesis (`SynthesisService`), and claim-to-coding handoff preparation. |

---

## 4. UI Domain Types & Client Boundaries

- **Desktop API Layer (`ui/desktop/src/api/`)**: `daemon_client.ts` handles core daemon transport, `shared_capabilities_client.ts` (`SharedCapabilitiesClient`) serves Ring 2 (`notebook`, `reviews`, `compute`, `artifacts`, `notes`), and `research_client.ts` / `live_run.ts` isolate Ring 3 research & streaming run state.
- **Workbench Contribution Registry (`ui/desktop/src/components/workbenches/registry.tsx`)**: Composes single-source `./{coding,research,assistant,quant}/contribution` descriptors without duplicate mode tables.
- **Headless Rust CLI (`apps/cli/`) & Web CLI Shell (`ui/cli/`)**: `apps/cli/` compiles as a pure-Rust headless CLI (`custos`) driving `custos_sdk::LocalApiClient` + `ProfileResolver` without Tauri, while `ui/cli/` connects via HTTP loopback `http://127.0.0.1:<port>/api/request` or Tauri wrapper (Zero-Mock) with `bin/lib/*` modularized under 450 lines per module.
