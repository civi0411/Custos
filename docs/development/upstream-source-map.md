# Upstream Source Inventory

> **Status:** Audit in progress. This inventory records observations in the Custos checkout, not a license-compliance conclusion. Do not claim upstream SHA pinning or production integration until the missing evidence is filled in.

## Goose-derived source

| Custos path | Observed content | Runtime status | Upstream revision / attribution |
|---|---|---|---|
| `crates/custos-runtime/src/engine/agents/` | Agent loop, state machine, extension manager, MCP client and platform tools with Goose identifiers | Present in the repository; `engine` is not mounted from `runtime/src/lib.rs` | Exact Goose source SHA and file-level mapping **unknown** |
| `crates/custos-runtime/src/agent/` | Extracted machine, inference, tool and conversation primitives | Compiled module; no daemon-composed `StateMachine` call path found at audit commit `3e4dac4` | File-level origin and divergence **unverified** |
| `crates/custos-provider/src/types/` | Goose-style provider, conversation and model types coexist with `ModelProvider` | Compiled; two model interfaces require a conformance mapping | File-level origin and divergence **unverified** |
| `crates/custos-sdk/` | Bindings and wire types retain Goose identifiers | Present; runtime usage not established by this inventory | File-level origin and divergence **unverified** |

[Goose's upstream license](https://github.com/aaif-goose/goose/blob/main/LICENSE) is Apache 2.0. Custos's root `LICENSE` now declares AGPL-3.0; that project-level declaration does not fill the missing file-level origin, modification and notice records for Goose-derived source. Complete those records and obtain a human-maintained license review before a release or reuse claim; this inventory is not a legal conclusion.

## External gateway candidates, not imported Custos code

[OrCa source study](orca-source-study.md) inspects local upstream commit `3f6225deeb08a82448c5f0b0725073401629d462` (MIT, Lovecast Inc.) as an **architecture/UX reference**, not as code imported into Custos. Its Electron/Node runtime, SQLite orchestration tables and React state are not Custos's Task Kernel or DB. Before any selective copy, record exact original path, preserve the applicable MIT copyright/license notice, audit dependencies, behavior parity, platform assumptions and tests; the presence of an MIT license alone is not an integration decision or a blanket compatibility conclusion.

[Open Science source study](open-science-source-study.md) inspects local `ai4s-research/open-science` commit `04b64817c12e7fdbe0e052bfa1aeaf8802feecde` as a Research Workbench reference. Its OpenCode sidecar, Tauri commands, file/JSONL provenance, notebook kernels and React pane tree are not Custos canonical Task, authority, evidence or session state. Selective reuse must preserve its license and dependency provenance and must enter Custos through the same ports, IDs and realization gates as every other adapter or resource.

| Upstream | Narrow reason to inspect | Custos integration boundary | Current audit state |
|---|---|---|---|
| [decolua/9router](https://github.com/decolua/9router/tree/ce4460ef79382bfddb4aa5fc0ff9f3cb0d5f95a8) | Model/catalog capabilities, transport translation, connection/fallback, quota/usage/price, diagnostics, scoped search/media and dashboard interactions | [First-party source-to-feature plan](9router-custos-integration-plan.md) & [System Design Blueprint](system-design-9router-agentgateway-custos.md) đặt selected behavior vào Rust core (custos-provider, custos-adapters); không ủy thác Task/Authority ra ngoài. | Local source inspected at `ce4460ef79382bfddb4aa5fc0ff9f3cb0d5f95a8` (MIT). Bóc tách catalog, pricing, RTK và fallback cascade vào Rust native. |
| [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway/blob/main/README.md) | MCP/A2A federation, model proxying, CEL guardrails and connection governance | [System Design Blueprint](system-design-9router-agentgateway-custos.md): Hấp thụ crates/llm conversion, MCP Federation Hub (streamable HTTP/SSE/mergestream) và Google A2A protocol vào `custos-adapters`. | Apache 2.0. Đã mổ xẻ crates/llm, mcp/handler.rs, a2a/mod.rs; không chạy như external proxy mù quáng, lấy patterns chuyển dịch thành module native trong Custos. |
| [rsclaw-ai/cap-protocol](https://cap-protocol.org/) | Draft PTY/structured driver for CLI agents | Optional `AgentRuntimePort` adapter, not Custos's internal protocol | Watchlist; no CAP conformance or first-party manifest established for this checkout |

Before copying source or launching a sidecar, pin upstream repository and commit, inspect license/notices/dependencies, document one real user job, compare against a direct adapter on the same fixture, and verify that no proxy can claim Custos-mediated effects it cannot intercept. A vendor/provider marketing claim is not proof of Custos integration.

## Required audit fields per reused module

Record upstream repository URL and commit SHA, original path, Custos path, whether copied or substantially modified, license and notice requirements, active call path, test coverage, and whether its tools can bypass Custos authorization. Classify each module `active`, `compiled-unwired`, `dormant`, or `duplicate`. A path name or comment alone is not proof of provenance or assurance.

## Integration gates

1. Choose one internal agent loop using replay fixtures and provider/tool parity tests; do not mount the entire legacy engine merely because the files exist.
2. Keep source attribution and file-level provenance with the retained code.
3. Verify each physical effect path independently. Goose CLI, Goose-derived in-process code, and a model endpoint have different loop ownership and assurance.
4. Update this inventory when upstream revisions or Custos call paths change.

## Cross-source Nexus audit: OrCa and Open Science

The local Nexus index was rebuilt in a disposable database for this audit so that observations were not mixed with an older shared index. It indexed the three local checkouts as follows:

| Checkout | Indexed files | AST symbols | Call references | Search chunks | Important limitation |
|---|---:|---:|---:|---:|---|
| Custos | 1,185 | 10,696 | 71,037 | 12,683 | Rust structure is queryable; generated and dormant source must still be separated from active call paths. |
| OrCa | 33,117 | 124 | 608 | 133,165 | OrCa is primarily TypeScript, while this Nexus build only provides structural AST indexing for Rust and Python. TypeScript findings therefore require path-targeted source inspection, not symbol counts. |
| Open Science | 784 | 1,782 | 13,937 | 5,195 | Rust and Python cores are structurally visible; UI behavior still requires direct TypeScript inspection. |

These numbers measure discoverability, not integration progress. A large matching file set does not mean that the corresponding behavior is compiled, composed, reachable, or correct in Custos.

### One realization ladder for code, documentation, and UI

Every imported, reimplemented, or upstream-inspired capability must use the same five-level status vocabulary:

| Level | Meaning | Evidence required |
|---|---|---|
| `inventory` | Source, design, or candidate behavior has been located. | Pinned repository/revision/path, license note, and a stated Custos user job. |
| `compiled` | Custos code for the capability is included in a built target. | Mounted module and a successful package/build check. Dormant source does not qualify. |
| `composed` | The daemon composition root binds the implementation to a stable port and Local API operation. | Active construction path, capability advertisement, and no parallel owner of canonical state. |
| `exercised` | A real client can reach the path with production-shaped input and honest failure behavior. | Contract or end-to-end fixture through the same API used by Desktop/CLI. A mock-only path does not qualify. |
| `verified` | The behavior meets its domain criterion, authority boundary, recovery semantics, and UI truth contract. | Acceptance, failure, restart/reconcile, and relevant security/evidence tests. |

Documentation may say `target` in addition to these levels, but `target` means no implementation claim. UI capability discovery must derive from the composed backend and must not infer a higher level from a route, component, or feature flag.

### Thematic placement: absorb behavior, not upstream folder trees

OrCa and Open Science solve different parts of the same supervised environment. Custos should not create an `orca/` subsystem and an `open-science/` subsystem. Their useful behaviors belong to five existing themes:

| Theme | Custos owner | OrCa behavior to absorb | Open Science behavior to absorb |
|---|---|---|---|
| Workspace host | `custos-runtime::{workspace,terminal}`, platform adapters | Isolated workspace lifecycle, terminal/process ownership, file/diff/browser resources, local/remote host affinity | Project environment, kernel working directory, dataset and artifact locations |
| Agent execution | `custos-provider`, `custos-adapters::harness`, runtime agent execution | Structured launch, native agent sessions, continuation, cancellation, unknown launch reconciliation | OpenCode/ACP-style runtime as an optional harness, not as the Task Kernel |
| Coordination | `custos-core` Task/Run contracts, `custos-runtime::workflow`, persistence | Dispatch claims, mailbox, worker lease, attention and Kanban projections | Experiment/run lifecycle, review barriers, negative and uncertain runs |
| Research provenance | `custos-packs::research`, research persistence and verification | No separate scientific truth model; reuse workspace/run primitives | Sources, methods, executions, artifacts, trajectory, provenance, annotation, reviewer assessment |
| Experience projection | `custos-bridge`, `custos-sdk`, `custos-app`, `ui/desktop` | Resource tabs, agent/workspace attention, terminal/diff/browser views | Pane tree, notebook, run/trajectory/provenance/artifact inspectors and selection-driven actions |

The invariant is one set of canonical identities: Task and Session have many-to-many bindings; Run/Node, Resource, Attempt, Artifact and Evidence records carry stable references back to that work. A pane, terminal, notebook kernel, agent harness, browser, or remote host is a resource or execution backend attached to these identities; it is never a second task database.

## OrCa matching ledger

| Behavior inspected in OrCa | Current Custos evidence | Realization level | Required correction or next gate |
|---|---|---|---|
| Workspace and worktree lifecycle | `workspace` and workflow lease modules exist; the current lease implementation creates/copies directories and tracks ownership in memory. | `composed` only for the narrower local workspace behavior; Git-worktree parity is `target`. | Define `ExecutionWorkspace` lifecycle and host affinity, use real repository/base revisions where required, persist lease ownership, and test crash cleanup and stale-base behavior. |
| Structured agent launch and continuation | Provider/harness registries and native harness adapters are daemon-composed. | `composed`; individual harnesses vary between `exercised` and `target`. | Persist `LaunchAttempt`, capability snapshot, native session reference, continuation mode, cancellation and `unknown` reconciliation. Never equate a provider response with Task completion. |
| Dispatch claim, worker ownership and mailbox | `WorkflowDispatcher` consults persisted WorkerRuns but keeps active dispatch claims in a process-local map. | `compiled` and partially `composed`. | Move claim/epoch/idempotency to a transactional repository before calling it atomic; add durable mailbox/event delivery and restart contention fixtures. |
| Attention board and Kanban | Desktop has task/run/workspace projections but no single durable attention model equivalent to an acknowledged event stream. | UI fragments are `exercised`; the unified behavior is `target`. | Project attention from canonical events with reason, severity, reveal target, acknowledge actor and cursor; do not store a separate board truth. |
| Terminal, file, diff and agent resources | Engineering panes and runtime resource APIs exist. | Mixed `composed`/`exercised`. | Make every resource advertise operations and assurance; use stable `ResourceId`; preserve the same Task/Run context across pane changes. |
| Browser, remote fleet and headless automation | Metadata/listing paths and panes exist; important effect paths intentionally fail closed or remain degraded. | `composed` for discovery; effects are `target` unless a real adapter proves otherwise. | UI must disable unavailable operations. Add connector-specific attempt, authority, receipt and reconcile tests before upgrading status. |
| Legacy Goose-derived runtime engine | `custos-runtime/src/engine/` remains present but is not mounted from `runtime/src/lib.rs`. | `inventory`/dormant. | Select behavior file by file. Do not mount the tree wholesale or create a third agent-loop owner. |

## Open Science matching ledger

| Behavior inspected in Open Science | Current Custos evidence | Realization level | Required correction or next gate |
|---|---|---|---|
| Pane tree and scientific resources | Research panes exist for literature, claims, notebook, methods, runs, synthesis, artifacts and inspectors; the shared container still behaves mostly as a tab strip. | `exercised` UI projection. | Introduce a persisted, versioned layout projection with stable resource identities and optional split/nesting; layout restoration must not resume execution. |
| Source, claim and method ledger | Research repository/API/UI paths exist. | `composed`, with selected flows `exercised`. | Bind every claim and method to source revisions, selection spans and assessment versions; expose provenance completeness rather than treating a link as support. |
| Runs and provenance | Research executions and artifacts are represented, but lineage completeness and criterion linkage are uneven. | `compiled`/partially `composed`. | Make `ResearchRun` an execution projection over Task/Run/Attempt IDs; record environment, inputs, outputs, metrics, negative/uncertain results and artifact edges. |
| Notebook kernel | The coordinator executes cells through a newly spawned Python process and a temporary script; UI language currently suggests persistent cell state. | `exercised` as isolated cell execution, not as a persistent kernel. | Either label it truthfully as isolated execution or add a supervised kernel/session with epoch, admission policy, cwd/data scope, resource budget, interrupt/restart and provenance receipts. |
| Artifact inspection | Several research viewers and artifact panels exist. | Mixed `compiled`/`exercised`. | Add an artifact renderer registry by media/schema, immutable artifact revision, source/run lineage, unsupported fallback, and export/handoff policy. |
| Trajectory and reviewer | Types and UI fragments exist, but review decisions are not consistently connected to completion criteria. | `inventory` to partial `composed`. | Project trajectory from canonical events; store typed reviewer assessments separately from evidence facts; completion remains criterion-specific. |
| OpenCode/ACP integration pattern | Useful as an agent-runtime reference. | `inventory`. | Treat it as an optional `AgentRuntimePort` adapter. It must not own Custos Task state, authority, memory, research provenance, or UI session truth. |

## Canonical ownership and dependency direction

| Concern | Canonical owner | May depend on | Must not own |
|---|---|---|---|
| IDs, contracts, FSM and authority/evidence decisions | `custos-core` | Domain values and ports | React state, provider SDKs, SQLite implementations |
| Model inference semantics and autonomous harness contract | `custos-provider` for `ModelPort`; `custos-core::contracts::harness` for `AgentRuntimePort` | Domain values and core-facing ports | Workflow scheduling, pack acceptance, UI layout |
| Scheduling, workspace, terminal, context and execution coordination | `custos-runtime` | Core/provider ports and pack contracts | Provider-specific wire details, canonical DB implementation |
| LLM, agent, MCP, browser, kernel and platform edges | `custos-adapters` | Declared ports | Task success, grants, canonical projections |
| Coding, Research and Assistant semantics | `custos-packs` | Core/runtime-facing contracts | Protocol transport and direct canonical writes |
| SQLite/CAS/projections | `custos-persistence` | Core storage contracts | Product routing and UI decisions |
| Local API and generated clients | `custos-bridge`, `custos-sdk` | Versioned application commands/events | New domain truth or adapter behavior |
| Composition and process lifecycle | `custos-daemon` | Every concrete implementation needed to wire the app | Domain policy hidden in bootstrap code |
| Desktop host and workbench UI | `custos-app`, `ui/desktop` | Local API capability/resource projections | Direct DB access, fabricated success, implicit grants |

No feature earns a new crate merely because it came from an upstream product. Split a crate only when dependency direction, platform compilation, independent release/conformance, or security isolation demands it.

### Target module shape inside the existing crates

This is a thematic extraction plan, not a claim that the files already exist and not a request for new crates:

```text
crates/
├── custos-core/src/
│   ├── task/                 # contract revisions, FSM, completion
│   ├── execution/            # Run/Node/Attempt identities and invariants
│   ├── authority/            # grants, permits, exact effects
│   ├── evidence/             # criterion verification and staleness
│   └── capability/           # capability/resource descriptors and assurance
├── custos-provider/src/      # ModelPort, stream, usage and model capability semantics
├── custos-runtime/src/
│   ├── workflow/             # plan/compile/schedule/claim/reconcile
│   ├── workspace/            # ExecutionWorkspace and host coordination
│   ├── agent/                # governed worker loop and native-harness coordination
│   ├── terminal/             # PTY lifecycle tied to Workspace/Resource IDs
│   ├── notebook/             # compute-session coordination; no scientific truth
│   ├── context/              # scoped context/continuation compilation
│   └── projection/           # attention and rebuildable resource projections
├── custos-packs/src/
│   ├── engineering/          # repo/diagnose/patch/verify semantics
│   ├── research/             # source/method/run/artifact/reviewer semantics
│   └── assistant/            # identity/draft/effect/automation semantics
├── custos-adapters/src/
│   ├── providers/            # direct/local model transports
│   ├── harness/              # Claude/Codex/Goose/OpenCode-style runtimes
│   ├── compute/              # Python/R/Jupyter/local/SSH/HPC implementations
│   ├── tools/                # filesystem/shell/git and deterministic capabilities
│   ├── mcp/                  # external tool/resource protocol only
│   └── platform/             # browser, remote host, sandbox and OS primitives
├── custos-persistence/src/   # repositories grouped by canonical aggregate
└── custos-daemon/src/
    ├── api/                  # handlers grouped by task/execution/resource/research/connections
    ├── local_api/            # envelopes/transports/typed client grouped the same way
    └── runtime.rs            # composition only

ui/desktop/src/
├── components/shell/         # navigation, conversation and Task anchor
├── components/views/         # resource catalog and pane-layout projection
├── components/workspaces/engineering/
├── components/research/
├── components/assistant/
└── api/                      # generated/typed Local API client; no domain truth
```

The immediate structural pressure is in `custos-daemon/src/api.rs` and `local_api/lib.rs`, not in the number of crates. Extract handlers and typed client groups mechanically while preserving one dispatcher, one schema surface and identical contract fixtures. Do not combine this move with semantic redesign of Task, Run or authority.

## Dependency-ordered realization plan

The current packet definitions and acceptance gates live in the [backend–desktop convergence plan §7](sade-frontend-backend-convergence-plan.md#7-packets-triển-khai-có-thứ-tự-và-đầu-ra-kiểm-được). The source-matching dependency is:

1. **Ingress and UI truth.** Restrict Local API and physical execution to admitted callers/scopes; capability operations drive pane controls. An isolated Python script is labelled as such.
2. **One live conversation.** A real provider or native harness receives source-backed context and returns a persisted answer, attempt and usage status. This supplies the baseline for cost and quality.
3. **Coding and Research vertical slices.** Use one workspace owner and a trusted patch/test gate for Coding; validate source/claim/run/reviewer links and transactional handoff for Research.
4. **Cross-lens continuity and Assistant effects.** Reopen the same work across three lenses, with selected-context receipt; add exact outbound permits/outbox before real send.
5. **Delegated orchestration.** Persist claim/lease epochs, mailbox and uncertain launch reconciliation before parallel worker dispatch. A direct read-only query does not wait for full multiworker parity.
6. **Measured optimization and optional adapters.** Evaluate S1/OI against the strong single-worker baseline, then add browser/fleet/automation/protocol connectors by user job and independent physical-effect gates.

### Upstream Browser & Mobile Emulator (Phone Simulator) Absorption

In OrCa upstream, the **Browser** (`src/main/browser`) and **Mobile Emulator** (`src/main/emulator`, `skills/orca-emulator`, `skills/orca-emulator-android`) serve two core operational roles that Custos absorbs:

1. **Coding Loop Visual Feedback & Mobile Responsiveness:**
   - **Dual Execution Surfaces**: Coding agents require both Desktop Browser and Mobile Phone Simulator (iPhone 15 Pro, Pixel 8, iPad Mini) preview canvases to detect responsive layout breaks, viewport clipping, and mobile touch targets.
   - **Accessibility Tree (AX Tree) Extraction**: Rather than guessing raw coordinates, OrCa normalizes the accessibility tree (`snapshot-ax-tree-walk`, `serve-sim-accessibility-tree`), allowing Coding LLMs to perceive headings, buttons, and inputs as structured nodes for accurate UI generation and debugging.
   - **Visual Proof Closure**: Simulator and browser render receipts feed into Custos invariant evidence (`ProofClosure`).
2. **Searching & Research Agent Runtime:**
   - Live browser navigation enables deep web search, documentation exploration (MDN, GitHub, arXiv), and structured scraping that standard text-only search endpoints cannot deliver.
3. **Custos Architecture Mapping:**
   - In Custos, these are projected in `ui/desktop` as `BrowserWorkbenchPane` and `MobileSimulatorWorkbenchPane` under Engineering and Shared resources, governed by `custos-adapters/src/platform/` (CDP and `simctl`/`adb` bridges).

## What must not be copied

- Do not copy OrCa's Electron process ownership, SQLite schema, global stores or mock/demo data into Custos canonical state.
- Do not copy Open Science's run/provenance files as a second event log or let a notebook kernel become the research truth owner.
- Do not retain two agent loops, two resource registries or three transcript stores for the three workbenches.
- Do not expose a pane merely because a component exists; expose it only when the backend advertises its resource and allowed operations.
- Do not label an observed native-agent effect `custos-mediated`, or a provider response `verified`, without the corresponding interception and criterion evidence.
