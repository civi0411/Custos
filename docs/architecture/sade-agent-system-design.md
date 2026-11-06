# SADE agent system and modular workbench interaction contract

**Status:** target architecture with an audited current-state map. A named module, pane, or upstream feature is not proof of an end-to-end capability. The product intent is defined in [Custos.md](../../Custos.md); the physical code map is [codebase-architecture.md](../development/codebase-architecture.md).

**Master specification integration:** product/package/UI contracts now live directly in master §1.2; one assembly and host/application boundaries in §2.3; data ownership and durability in §§6.3/6.5; reasoning/execution in §14.15; physical feature placement and package/resource lifetime in §15.10; notebook/training/GPU recovery in §17.8; and the aligned P00–P11 delivery/acceptance summary in §18.8. This topic expands those decisions; it is not a prerequisite link that substitutes for their presence in the master. Production-import metadata was refreshed at `rft/c4c0c35` (12 product Cargo packages); semantic findings below retain their stated audit scope and must be revalidated before code changes. This consolidation changes documentation only.

**Current assembly reading entry:** §§10–13 map the current crates into cohesive blocks, one composition path, workbench contributions and measurable resource behavior. The historical snapshot in §7 does not override the current source follow-up: `workflow/task_runtime` and daemon `api` are directory modules, the legacy `engine` tree is absent, and Coding/Research/Quant are optional specialist workbenches around Base Assistant. Implementation is tracked in [P00–P11](../development/modular-workbench-refactor-plan.md#28-work-breakdown-chi-tiết-p00p11), not a separate campaign.

## 1. Product contract

Custos offers one supervised work environment: Base Assistant/Copilot, with Coding, Research and Quant specialist workbenches enabled according to distribution and installed capability. A person can begin in any available lens, continue the same Task in another lens, and inspect which sources, model or harness, workers, effects, and evidence produced an outcome. The system optimizes accepted work per cost within the user's scope, quality threshold, provider pin, privacy, and time budget. The earlier three-lens diagrams below describe the shared interaction pattern; they are not a restriction on the current optional workbench set.

The reusable unit is a **Task with versioned context and outcome**, not a chat transcript or a specific agent session. Each lens has its own session list and remembers its last selected conversation. A person may explicitly expose an existing complete session in another lens or create a linked continuation there. A short read-only question may stay ephemeral. Starting a durable run, write, experiment, background job, or outbound action materializes the Task contract. The UI never treats switching lenses, opening a pane, or restoring layout as permission to execute.

```mermaid
flowchart TD
    UI["Conversation + three workbench lenses"] --> API["Versioned Local API"]
    API --> K["Task Kernel: contract, authority, evidence, outcome"]
    K --> R["Runtime: route, context, worker graph, scheduler"]
    R --> C["Shared retrieval and context service"]
    R --> W["Custos worker loop + ModelPort"]
    R --> H["Native harness + AgentRuntimePort"]
    R --> P["Engineering / Research / Assistant pack"]
    P --> V["Pack verifier"]
    W --> E["Governed capability dispatch"]
    H --> N["Native tool path with declared assurance"]
    E --> V
    N --> V
    V --> K
    K --> S["Canonical events, snapshots, artifact refs"]
    S --> UI
```

The diagram shows responsibility, not a claim that every native tool is mediated. Each action reports `custos-mediated`, `provider-governed`, `observe-only`, or `unknown` from its tested execution path.

## 2. Five distinct things called an agent

| Concept | Meaning | Owner | Example |
|---|---|---|---|
| Model endpoint | One inference request, stream, usage and tool-call proposal | `ModelPort` contract; concrete provider adapter | Local coder model, cloud model, model proxy |
| Custos worker | Custos-owned reasoning/tool loop using a model endpoint | `custos-runtime/src/agent/` | Coding implementer using a local coder model |
| Native harness | External program owning its own loop and possibly its tools | `AgentRuntimePort`; adapter in `custos-adapters/src/harness/` | Codex, Claude Code, OpenCode |
| Worker role | A bounded job in one Task's execution graph | Workflow runtime and pack recipe | Read-only repo scout, claim reader, verifier |
| Remote agent | Separate identity and trust domain reached over a network protocol | Optional A2A adapter | Independently operated specialist service |

The user chooses a model or a harness in the same executor picker, with a visible type label and capability disclosure. Selecting a local coding model creates a Custos worker; it does not magically turn the model endpoint into an autonomous harness. A subagent is a child `WorkerRun` with a smaller source scope, tool set, budget, deadline, and output contract. In-process child workers do not require A2A. A2A is only for a truly remote agent boundary.

### Worker lifecycle

`proposed → admitted → queued → running → waiting_human | reconciling → observed_terminal → verified | limited` is the conceptual lifecycle. The current FSM/types must be reused and extended only after contract audit. `observed_terminal` says the worker finished; the Task passes only after criterion-specific verification. A worker launched in an uncertain external window is reconciled before any retry. A child worker cannot enlarge its parent's grant, source scope, egress, or aggregate budget.

Every worker has: `task_id`, `run_id`, `worker_run_id`, parent worker if any, role, pack, input refs and snapshot, model/harness and actual version, allowed capabilities, write set, deadline, budget reservation, output schema, event cursor, assurance, and terminal observation. A Task may run one direct worker, a sequential chain, or a bounded graph. Fan-out needs independent inputs or disjoint writes, a declared join rule, and a measured benefit; it is never the default for a simple chat turn.

### Agent interaction visible to the human

The Task strip shows active worker count, actual executor, locality, spend and unresolved attention. Opening a worker card shows its instruction/job summary, source scope, workspace or dataset, tools, progress events, output artifact, and what it cannot see. The person can ask a follow-up to the parent, steer a child only if that harness really supports steering, cancel a run with a separate `requested` versus `observed stopped` state, or request a new candidate. A worker's question, approval request, crash, unknown effect, or blocked join appears in the attention queue with a `Reveal` action that opens the precise pane and object.

## 3. Shared retrieval and context service

“RAG hub” means a shared **logical retrieval and context pipeline**, not a new crate, MCP server, graph database, or always-on agent. Its public job is to turn a scoped question plus source inventory into a versioned `ContextPack` and `ContextReceipt`. The receipt states what was selected, omitted, redacted, stale, or unavailable; it also records the actual egress target. No retrieved content becomes an instruction or grant.

```mermaid
flowchart LR
    Q["Task question + pack recipe"] --> F["Scope and privacy filter"]
    F --> I["Versioned source inventory"]
    I --> X["Exact/lexical candidates"]
    X --> O["Optional structural/semantic expansion"]
    O --> D["Dedup, diversity and freshness"]
    D --> B["Token allocation + redaction"]
    B --> P["ContextPack + ContextReceipt"]
    P --> M["Model or native harness"]
```

The cheap path is exact IDs, symbol/path/DOI/contact lookup, and lexical search. AST/reference expansion, embeddings, reranking, or an S1 judgment are optional decisions with measured recall, latency and cost. S1 can rank candidates, detect ambiguity, or recommend another retrieval pass; policy code checks scope, privacy, source freshness and egress. S2 uses the resulting material to reason; a citation or source locator must be reopenable independently of the answer.

| Pack recipe | Canonical source and useful retrieval | Output and UI proof |
|---|---|---|
| Coding | Git HEAD + selected dirty bytes, worktree/index generation, exact path/symbol/trace, references and tests | Source spans and base hash; code map, omissions, changed-source warning |
| Research | DOI/URL/PDF version, page/section/table/figure spans, dataset/method/run refs, counterevidence | Passage viewer, parse coverage, claim relation and source version |
| Assistant | User-authorized notes, stable contact/event IDs, timestamped connector snapshot and personal preferences | Identity card, freshness, privacy/egress and exact payload preview |

The same source registry and context compiler support all three packs, while pack recipes supply selection and verifier obligations. The service must not collapse a research claim into a fact or choose a recipient solely from an untrusted document. Its indexes are rebuildable projections; canonical source records, artifacts and evidence remain under Task/persistence ownership. Coding uses admitted file/search/Git capabilities; no dedicated Repo Intelligence scanner, graph, query coordinator or index service is part of the product.

### Retrieval modes and budgets

`none` for context already supplied; `single` for one scoped lookup; `iterative` when source coverage is insufficient; `parallel-read` for independent corpora; `abstain` for inaccessible or contradictory evidence. The route records why it expanded search, the maximum calls/tokens/time, and a stop reason. Native harnesses may have their own retrieval; Custos records the sources it passed in and what it can observe, and labels hidden retrieval or tool activity accordingly.

## 4. Pack-owned worker recipes

Pack manifests describe jobs and evidence, while the runtime owns scheduling and the kernel owns authority and completion. A skill is a reusable bounded operation/recipe with inputs, outputs, capability request, resource estimate, and provenance obligations. A skill may call deterministic code or a model; that does not make it an autonomous agent. An autonomous specialist needs a worker role, lifecycle, event stream and budget.

| Pack | Direct path | Optional specialist workers | Acceptance and stop condition |
|---|---|---|---|
| Coding | One pinned model/harness reads repo, explains or proposes patch | Read-only locator, implementer in owned worktree, independent verifier/reviewer | Anchors reopen; patch base/scope valid; trusted behavior tests or review tied to criterion. Stop on stale base, write conflict or missing oracle. |
| Research | One reader answers from a versioned passage | Independent corpus scouts, claim extractor, contradiction finder, experiment worker, reviewer | Locator, extraction and semantic support are separate; run inputs/environment/output lineage visible. Stop at parse gap, contradictory support or unverifiable claim. |
| Assistant/Copilot | One conversational worker answers or drafts | Identity resolver, schedule planner, personal recall, bounded automation worker | Stable recipient/time and exact effect receipt. Draft never means sent; timeout after dispatch is uncertain and requires reconciliation. |

The pack can suggest a worker graph. OI chooses among admissible direct/single/bounded-graph candidates using expected quality, total cost, latency, risk and uncertainty. Hard user constraints filter before optimization. S1 may supply typed, abstainable signals; it cannot issue permits, mark claim support as true, or force an expensive graph. Meta analyzes traces offline and proposes a new policy version after evaluation. The strongest S2 model remains selectable and pinnable; no S1 shortcut silently downgrades it.

## 5. Desktop interaction contract

The desktop has one quiet, typography-first shell. The three lenses are recognized by icon, title, resource set, and one restrained accent; no separate neon theme or decorative AI dashboard. The conversation remains a persistent pane or dock. The currently bound Task and the destination of the next message are visible near the composer. Resource panels use daemon `resource_id` and `supported_operations`; unavailable actions are disabled with a reason.

| Surface | What the human sees | What an action does |
|---|---|---|
| Global rail | Copilot, Coding, Research icons with tooltip and keyboard shortcut | Changes lens/layout only; retains Task and conversation |
| Task strip | Goal, Task state, run/worker count, actual executor, budget, source freshness, attention count | Opens precise Task or attention detail |
| Conversation | Turns with Task binding; selected source/artifact chips; agent event summaries | Sends a turn in the selected Task or an explicit `No Task` conversation |
| Resource canvas | Typed source, code, diff, notebook, artifact, draft, calendar or terminal pane | Opens an existing object; pane restore has no side effect |
| Inspector | Context receipt, worker graph, criteria/evidence, effect history, cost and unknowns | Reveals provenance and permits a scoped correction/waiver command |
| Attention queue | Questions, exact approvals, conflicts, uncertain effects and completed outcomes | Reveal, answer, approve exact payload, reject, or acknowledge separately |

### Three concrete journeys

1. **Copilot → Coding:** user asks about a bug in chat; source chips show read material. `Work on this in Coding` opens the lens with the same Task and session. `Create patch step` previews repo, allowed paths, behavior criterion, executor and cost; acceptance creates a new workflow obligation. Agent edits in an owned workspace, then diff/tests/evidence appear. `Apply` checks base and scope; `Push` is a distinct effect.
2. **Research → Coding:** user selects two passages, a claim and an experiment result. `Implement selected findings` previews exact source/claim IDs, caveats, stale items and redactions. A linked continuation creates an Engineering brief and can open Coding without launching a worker. The source viewer and original chat remain accessible. Coding's model receives only selected, permitted refs, not the entire research transcript or a mail grant.
3. **Coding/Research → Copilot:** user selects an accepted patch summary or reviewed brief and asks for an update to a colleague. Copilot drafts, resolves the stable recipient, displays exact body/attachments and privacy route, then asks approval to send. Editing any payload byte invalidates prior approval. `Sent`, `failed`, and `uncertain` come from receipt/reconciliation, not the agent's prose.

The switcher must distinguish `Open lens`, `Open full chat in…`, `Continue as linked chat`, `Add pack step`, and `Fork child Task`. `Open lens` restores that lens's last conversation. `Open full chat in…` adds the existing session to the destination lens's history and opens the same canonical journal, without rewriting original turns or changing the Task contract. `Continue as linked chat` creates a destination session and a lineage manifest; the source timeline can be displayed in full as read-only context. The destination model receives only what the context compiler admits within its budget. The UI reports delivered turn IDs, summaries, omitted bytes, redactions and unsupported native hidden state; “full history visible” must not be phrased as “every token sent to the model.” `Add pack step` and `Fork child Task` write Task lineage and obligations separately. No transfer copies an active permit.

Research notebook code is for exploring data, reproducing methods and demonstrating an analysis. Sending a Research finding to Coding creates an Engineering brief with selected code/data/claim/artifact references; it does not apply notebook cells as a repository patch. The Coding agent validates the target repo and behavior criterion before implementation.

### Empty, error and recovery states

Every pane can show `loading`, `available`, `degraded`, `unavailable`, `stale`, or `offline`, with a meaningful next action. An empty research library offers `Add source`; an absent repo offers `Choose workspace`; a missing connector offers `Connect account` or `Work with a draft`; an unsupported native steer control is hidden. A daemon disconnect keeps the last rendered data labeled stale and blocks mutations until resync. On reconnect, the client resumes by event cursor and command ID, then reconciles pending attempts before offering retry. Closing a pane or switching lenses never cancels a run.

## 6. Backend/API slices and ownership

| Responsibility | Existing home | Target contract and current gap |
|---|---|---|
| Task, criteria, authority, evidence | `custos-domain`, `custos-core` | Canonical IDs/revisions and gates; no pack or model writes success directly. |
| Model and harness contracts | `custos-provider`; `custos-core/src/contracts/harness.rs` | One inference attempt versus an autonomous harness; capability/version/assurance per actual path. |
| Agent loop, worker graph, OI | `custos-runtime/src/agent`, `workflow`, `oi` | Persisted child-worker ownership, bounded graph and join; no second agent-loop authority. |
| Shared context | `custos-runtime/src/context`; pack recipes in `custos-packs` | Versioned inventory, permission filter, retrieval, ContextPack/Receipt, freshness and cost; existing lexical compiler is a component, not the full service. |
| External reads/effects | `custos-adapters` | Provider, harness, filesystem, MCP, browser, connector and notebook adapters with real capability discovery and receipt semantics. |
| State and recovery | `custos-persistence` | Task/Run/Attempt, source/artifact/evidence, event cursor, worker claim and uncertain effect reconciliation. |
| API composition | `custos-daemon`, `custos-bridge`/SDK | Stable command/query/event DTOs; daemon binds actor and resolves workspace/harness; client never writes DB. |
| Experience | `ui/desktop`, thin host `custos-app` | Shared shell, three lens presets, resource/attention registry, Task binding and explicit handoffs. |

An internal retrieval query is a runtime service call, not MCP. MCP is an adapter for external tools/data and may expose a reviewed subset outward. ACP is an editor-to-agent integration option. A2A is a remote-agent boundary. None of these protocols is required to move from Copilot to Coding or to run local subagents. The protocol hub only provides connection discovery, lifecycle, identity and conformance at those boundaries; Task authority remains in the kernel.

### Proposed commands and projections (map onto existing Local API before adding schema)

`GetTaskWorkspace(task_id)`, `ListResources(workspace_id)`, `WatchEvents(after_seq)`, `CompileContext(task_id, selected_refs, destination_pack, dry_run)`, `StartRun(task_id, workspace_id, executor_id, expected_revision)`, `GetWorkerGraph(run_id)`, `ContinueWithSelectedRefs(source_session_id, task_id, destination_pack, ref_ids, mode)`, `ApproveExactEffect(intent_digest)`, and `GetOutcome(task_id)` describe behavior, not a declaration that these route names exist. Reuse existing DTOs and commands wherever semantics already match. Each write has actor, `command_id`, expected revision and idempotent result; reads include freshness/version. The UI does not synthesize a successful run from a completed stream.

## 7. Verified state of this checkout

| Capability | Code evidence | Assessment |
|---|---|---|
| Three lens navigation and many resource panes | `ui/desktop/src/app/studio/page.tsx`, `OrcaTabbedContainer.tsx`, Research and Engineering pane folders | Present as UI; resource operations now follow daemon descriptors. Session history was previously one shared list and explicit full-session transfer was absent; the per-lens projection is a separate implementation gate. |
| Task-backed native launch | `custos-runtime/src/workflow/task_runtime.rs`, daemon `api.rs`, desktop `AgentsWorkbenchPane.tsx` | Present for the current native path; launch completion is not Task acceptance. |
| Custos-owned worker loop | `custos-runtime/src/agent/runtime_port.rs` and `machine.rs` | Code exists and a governed harness is composed. Its full multi-turn/tool/evidence parity with native harnesses is not established by those files. |
| Shared context | `custos-runtime/src/context/compiler.rs` | Existing context compiler remains; dedicated repo-analysis modules have been removed. |
| Pack specialization | `custos-packs/src/{engineering,research,assistant}` | Manifests/skills and some verifiers exist; several declared features are not proven production flows. |
| Research workbench | Literature/Claims/Notebook/Runs/Synthesis/Artifact panes and daemon research API | Partial real surfaces; provenance completeness, claim support and sessionful notebook semantics need per-path proof. |
| Native OpenCode lifecycle | `custos-adapters/src/harness/opencode.rs` | Bounded CLI path exists. Production server events, continuation, steer and cancel are not connected; capability profile must say so. |
| Parallel worktree integration | Workspace and lease modules, existing implementation plan | Target: durable ownership, conflict-safe integration, crash recovery and user-visible candidate comparison. |

The earlier Goose-derived `custos-runtime/src/engine/` inventory is historical: that tree is absent in the current follow-up checkout. Do not count it as current code or propose deleting it again. Review pinned upstream code and conformance fixtures before adapting any OpenCode or other native harness transport; a similarly named local checkout is not evidence of compatible binary/API behavior.

## 8. Build sequence and release gates

1. **One trusted path per pack:** Coding repo explanation/patch, Research source question/claim, Copilot draft. Each starts from chat, binds a Task as needed, returns reopenable artifact/criterion state and resumes after daemon restart.
2. **Context receipt:** scope/freshness/egress filtering, source IDs and omissions across all three pack recipes. Compare exact/lexical baseline to optional AST/semantic/S1 expansion on accepted outcomes and cost.
3. **Worker graph:** one child read-only scout, persisted ownership and events, bounded budget, explicit join/unknown. Then add worktree-isolated writer candidates and human comparison. Do not infer internal delegation safety from a prompt or from an A2A name.
4. **Research depth:** source/passages/claim support, experiment attempt and artifact lineage, review card and selected handoff. A citation with wrong support and a failed run with partial output are mandatory fixtures.
5. **Assistant effects:** stable identity, exact draft, fake connector outbox, uncertain reconciliation, then real connector. A draft and an externally observed send have different status and UI copy.
6. **Workbench continuity:** switch lens without mutation; add step/fork with manifest; reopen original chat and exact source/artifact; compare accepted result, spend and human time against a pinned single-worker baseline.

For every gate report what is implemented, exercised, degraded or unavailable. A screenshot or compiled module is not an end-to-end test. Performance claims use task mix, model, hardware, source version, cold/warm cache, paid usage and unknown spend.

## 9. Primary source anchors and interpretation

- [Orca orchestration](https://github.com/stablyai/orca/blob/main/docs/site/content/docs/cli/orchestration.mdx) describes Run/Task/Dispatch/worker supervision; [Orca worktrees](https://github.com/stablyai/orca/blob/main/docs/site/content/docs/model/worktrees.mdx) shows checkout-scoped resources and candidate work. Custos adapts those interaction patterns to its own Task/authority/evidence contracts.
- [OpenCode agents](https://opencode.ai/docs/agents/) describes primary agents, subagents and tool permissions. It is evidence for OpenCode's interface, not proof that Custos currently observes or mediates every native child action.
- [Open Science repository](https://github.com/ai4s-research/open-science/blob/master/README.md) describes notebook, run, provenance and reviewer surfaces. The pinned [local source study](../development/open-science-source-study.md) identifies what its code actually records and what is best effort.
- [The original RAG paper](https://arxiv.org/abs/2005.11401) motivates combining a generator with retrieved material. Custos's scope, provenance, freshness and per-pack recipes are product design decisions requiring its own evaluation.

## 10. Cohesive assembly blocks inside the current crates

A block is a responsibility with owned state, inputs, outputs and lifetime. It is not necessarily a crate, service, folder, process or agent. Keep the existing canonical crate groups and 12 product Cargo packages; use modules to express cohesive responsibilities, and split a build unit only when measured dependency/release constraints justify it.

| Block | Current home and public seam | Owns | Must not own |
|---|---|---|---|
| Domain vocabulary | `custos-domain/src/{task,session,authority,evidence,workspace,research,quant,shared,...}` | Persisted identities, pure records/invariants and deterministic math | HTTP, process, DB handles, plugin loading |
| Trusted policy | `custos-core/src/{kernel,authority,evidence,oi}` and `contracts/` | State transitions, grants/permits, budget/evidence gates, boundary interfaces | Concrete driver construction or native provider wire mechanics |
| Inference contract | `custos-provider/src/{port,turn,events,bridge,types}` | One canonical model attempt/stream, usage certainty, legacy semantic translation | Credentials, file acquisition, runtime process authority |
| Supervised execution | `custos-runtime/src/{workflow,agent,session}` | Run/worker graph execution, dispatch ownership, checkpoints, continuation and observation | A second Task database or native harness's hidden loop |
| Context and optimization | `custos-runtime/src/{context,cognitive,oi}` | Permission-filtered context, bounded judgments, candidate topology and route proposals | Self-issued grants, unmeasured mandatory orchestration, unmanaged I/O paths |
| Resource lifecycles | `custos-runtime/src/{workspace,terminal,notebook,compute,packages}` | Scoped live PTY/kernel/job/worker handles and leases | Window/tab layout, arbitrary SQL or provider credentials |
| External mechanisms | `custos-adapters/src/{providers,harness,sandbox,workspace,terminal,notebook,compute,packages,market,tools,mcp,a2a,...}` | Physical file/network/process/device/connector boundaries and conformance | Goal/criteria completion or a parallel policy authority |
| Domain specialization | `custos-packs/src/{engineering,research,assistant,quant}` | Jobs, input/output/rubrics, role/context recipes and verifiers | Direct subprocess/HTTP/SQLite ownership disguised as skill semantics |
| Canonical storage | `custos-persistence/src/{repositories,store,artifacts}` | Durable topic records, transaction/command/event ownership, CAS metadata and storage implementations | Deciding model route or treating a renderer import as installed capability |
| Application composition | `custos-daemon/src/{runtime,application,api}` | Construct/share concrete instances; orchestrate use cases; validate/route API and publish projections | Another agent loop, per-workbench store or default native driver creation inside transport |
| Session linkage | `custos-bridge/src/{port,service}` | Explicit promote/attach/steer behavior using existing services | Independent worker scheduler or transcript clone |
| Product clients | `custos-sdk`, `custos-app/{cli,desktop}`, desktop renderer | Versioned client transport, host/native UI lifecycle and presentation | Direct DB/effect access or internal backend composition |

### 10.1. Current assembly defects to retire

The following observations are source-backed, not a request to move every file immediately:

1. `LocalApiDispatcher::new` constructs native terminal/Python/file coordinators, a default all-pack registry and an MCP hub. `CustosRuntime::bootstrap` constructs some of the same responsibilities and replaces them through `with_*`. Construct the selected concrete graph once in daemon composition, then inject the dispatcher dependencies. A transport constructor must not secretly choose production capability.
2. `TaskRuntime::new` permits many missing dependencies and relies on a sequence of `with_*`. Preserve builders where useful for tests, but require a validated ready composition before production start; explicitly disabled capability is different from accidentally missing mandatory store/kernel.
3. Package inventory query errors currently fall back to all four components in bootstrap. Fail closed for optional execution and expose the storage error; an error is not evidence that every pack is enabled.
4. `runtime/cognitive/skills` includes concrete HTTP/file/process mechanics; domain-pack skills call its process helper. Separate the semantic skill from the physical capability using existing ports and adapter homes, preserving scope/output/time validation. Avoid introducing one universal giant ToolManager or a trait for each trivial helper.
5. `SandboxedProcess` filters environment and captures `Command::output`; its name does not establish OS file/network isolation. Use actual adapter/SandboxPort guarantees and bounded streams; never infer assurance from class name or `requires_permit` metadata.
6. `custos-provider` compatibility types contain filesystem image acquisition and file-backed catalog load/save, plus reqwest error translation. Separate I/O from pure format conversion; keep the legacy bridge while exercised callers need it. A `reqwest` error adapter is not itself an HTTP request, so classify actual behavior before moving dependencies.
7. `llama-cpp-2`/sys are currently non-optional adapter dependencies, `local-inference-native` is an empty feature, and bootstrap calls native runtime initialization eagerly. A lightweight API-only Base needs a supported feature/build path and lazy runtime initialization or a verified engine subprocess. No savings claim until build contents and startup are measured.
8. Root glob re-exports and compatibility aliases broaden the apparent public API. Narrow new caller imports to topic paths; retire aliases only after export/config/fixture callers are migrated. Do not rename serialized IDs to match Rust folder aesthetics.

Full source evidence and severity are tracked in the [crate assembly follow-up](../development/workbench-core-and-controls-audit.md#crate-assembly-follow-up). P00–P11 remains the execution queue.

### 10.2. OOP principles expressed in Rust

- **Encapsulation:** mutation through the owning service/repository with expected version and validated transition. No public field bag shared so every consumer may modify canonical state.
- **Composition:** daemon injects one compatible kernel/store/runtime graph; a worker receives only the ports it needs. Do not make `CodingRuntime`, `ResearchRuntime`, `QuantRuntime` inherit another all-powerful runtime.
- **Polymorphism:** traits at real variation boundaries: ModelPort, AgentRuntimePort, resource/compute/storage ports. Use concrete types for deterministic helpers and enums for a genuinely closed set. Trait dispatch is not a downloadable native ABI.
- **Interface segregation:** a historical artifact reader does not need order execution or the entire SqliteTaskStore API. Reuse current narrow ports; composite `StoragePort` can remain an assembly convenience without becoming every worker's dependency.
- **Substitution:** replacement must preserve cancellation, error, usage certainty, source/authority scope and observation semantics, not merely return the same JSON shape.
- **Dependency inversion:** core/runtime declare needs; adapters/persistence supply mechanics; daemon assembles. HTTP/stdio are external boundaries, not mandatory serialization between every in-process block.

[Rust trait objects](https://doc.rust-lang.org/book/ch18-02-trait-objects.html) support common-behavior abstraction. [Translation boundaries](https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer) can preserve Custos semantics when adapting an upstream system; they need not be new services. These are design tools, not proof of performance or a reason to wrap every function.

## 11. One assembly path and explicit lifetime ownership

```mermaid
flowchart TD
    HOST[Desktop or CLI host] --> SDK[Typed Local API client]
    SDK --> API[API transport and topic application services]
    BOOT[Daemon bootstrap and supervisor] --> DATA[Canonical storage and credential references]
    BOOT --> POLICY[Shared trusted policy services]
    BOOT --> EXEC[Validated runtime and resource coordinators]
    BOOT --> CAP[Verified inventory and capability snapshot]
    BOOT --> API
    API --> POLICY
    API --> EXEC
    API --> CAP
    EXEC --> MODEL[ModelPort]
    EXEC --> HARNESS[AgentRuntimePort]
    EXEC --> TOOL[Scoped capability and compute ports]
    MODEL --> AD[Concrete adapters]
    HARNESS --> AD
    TOOL --> AD
    EXEC --> PACK[Pack recipes and verifier obligations]
    EXEC --> DATA
    POLICY --> DATA
    API --> DATA
```

This shows runtime wiring, not Rust import direction. Ports are defined inward. Application read/query services may use scoped repositories; canonical mutation remains through the owning transition/transaction service. A lifecycle owner can be shared by many consumers without duplicating process handles or execution authority.

Bootstrap sequence:

1. Resolve profile and managed roots; open canonical storage, migrate and reconcile prior command/run/effect/package uncertainty.
2. Resolve credential backend and connection references; record unavailable dependencies without inventing readiness.
3. Load verified installation/profile metadata and construct a capability projection. Optional inventory failure must not expand capability.
4. Construct shared policy and execution services with narrow injected ports. Mandatory dependencies are validated; optional ones have explicit unavailable state.
5. Register enabled first-party pack semantics and executor descriptors; do not load model weights, start Python, open broker sockets or spawn harnesses just because a descriptor exists.
6. Construct API/application services over those instances, then expose transport and event replay. Background maintenance has cancellation, owner and join/shutdown semantics.
7. Acquire expensive resource on admitted use; settle attempt/output/usage, release or retain according to resource policy; reconcile unknown external results before retry.

| Lifetime | Objects | Who releases it |
|---|---|---|
| Daemon/profile | Store, credential service, policy, capability projection, connection descriptors | Daemon shutdown supervisor |
| Installed component version | Verified executable/assets and dependency references | Package manager after lease/reference checks |
| Project/resource | Scoped workspace binding, PTY/kernel or external job tracking | Owning resource service, not React unmount |
| Task/Run/attempt | Contract revision, reservation, chosen executor, observation and evidence | Runtime settlement/reconciliation |
| UI view | Layout/focus, editable draft buffer, subscription/render handle | UI lifecycle; does not own remote execution |

Shutdown distinguishes stop admission, request supported cancellation/drain, persist/reconcile pending state, flush required writes and release owned processes. Do not kill a user-owned Ollama service or declare a remote billed job stopped merely because its connection closed.

## 12. Workbench contribution, skill and executor assembly

| Extension unit | What it adds | Example | What it cannot imply |
|---|---|---|---|
| UI contribution | Resource renderers, navigation defaults, commands and inspector views | Research PDF/notebook/run viewers | Executable permission or package trust |
| DomainPack | Task/job taxonomy, recipes/roles, context and verification obligations | Engineering patch rubric, Quant backtest validation | Its own Task DB or always-running agent |
| Skill/capability | Typed bounded operation declaration and semantic result | Parse source, targeted test, calculate metric | Authority solely because profile says read-only or requires-permit |
| Executor | Actual reasoning/computation lane with observed capability | API model + Custos loop, native harness, parser worker, remote training | New workbench or shared hidden model state |
| Package component | Versioned distributable bytes/environment/assets | Document parser, local inference runtime | Grant, ready process or valid evidence |

A Coding skill remains semantically in `custos-packs/engineering`; its process/filesystem action uses existing capability/sandbox ports and adapter implementation. Shared HTTP/file mechanisms live at adapter boundaries; pack-specific selection/transformation/rubrics stay in their domain. The same notebook engine may serve Research and Quant by capability without forcing installation of all Research UI. No skill folder is a second agent framework.

Model provider API and native coding agent are both first-class executor choices. A local coder model uses a Custos-owned loop; Codex/Claude-style harness uses its own supported loop. Optional tools exposed via MCP are adapters into the same admitted capability path. A2A is reserved for a distinct remote agent trust boundary, not local worker/thread calls. Preserve native tools' honest assurance and cost visibility.

Adding a workbench should require: an approved descriptor/compatibility contract, domain recipe/verifier contribution, UI contribution, optional engines/assets and evaluation fixtures. It should not require edits scattered across multiple hardcoded switches, another global store, a new DB or another root agent. First-party trusted policy can remain host-compiled until independently replaceable policy has an explicit safe contract; installation does not imply dynamic loading of arbitrary Rust code.

## 13. Smoothness, cost and assembly acceptance

Performance has separate budgets: startup/build footprint, interactive rendering, execution resource use and model/connector spend. Do not combine them into one “lightweight” label.

- **Startup:** keep metadata/descriptor registration cheap; eliminate duplicate hidden driver construction and eager native inference setup. Measure API-only Base versus native-local profile on the same release build/hardware.
- **Interactive UI:** event cursors and scoped live observers; high-frequency tokens/output avoid whole-app projection churn. Dispose subscriptions while preserving backend resource identity.
- **Execution:** bounded queues and streams, global concurrency/reservations, explicit stateful leases, per-attempt cancellation/deadlines, real OS enforcement reporting. An async function containing synchronous filesystem/process work still needs hot-path/blocking review.
- **Data:** short atomic canonical writes; immutable large artifact references; page/slice history queries; derived cache rebuildability. Do not hold writer lock during model inference, downloads or backtests.
- **Model economics:** context source coverage, exact search/cache, optional S1/OI under hard constraints; count failed/parallel attempts and verifier/human time. Native harness spend remains unknown unless transport exposes reliable accounting.
- **Distribution:** Cargo features control compilation/optional dependencies, not runtime installation. A feature with no cfg/dependency effect does not remove native code. [Cargo feature semantics](https://doc.rust-lang.org/cargo/reference/features.html).

Assembly acceptance requires: one production construction path; no all-pack fallback on inventory failure; no physical skill execution outside the admitted capability boundary; one shared lifecycle owner per resource; replaceable executor conformance; same historical data across package/lens changes; scoped UI events; actual Base contents and measured resource/cost results. Keep unit/integration tests and compatibility adapters until their callers are retired. The complete implementation/caller/data/test order is [P00–P11](../development/modular-workbench-refactor-plan.md#28-work-breakdown-chi-tiết-p00p11).
