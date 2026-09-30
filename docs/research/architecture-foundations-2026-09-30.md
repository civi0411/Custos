# Research foundations for the Custos architecture

**Document ID:** RESEARCH-ARCH-01
**Status:** Dated research synthesis; non-normative until adopted by architecture or ADR
**Reviewed:** 2026-09-30
**Canonical architecture:** [`../architecture/definitive-product-architecture.md`](../architecture/definitive-product-architecture.md)

This review tests the submitted architecture proposal against primary papers,
standards, official framework documentation, and the current Custos tree. A
paper result is evidence for an experiment, not a universal production promise.

## Decision synthesis

| Source | Reliable lesson | Custos adoption | Guardrail |
|---|---|---|---|
| [ReAct](https://arxiv.org/abs/2210.03629) | Interleaving reasoning and environment observations can improve interactive work | Worker loop may alternate observations and proposals | Reasoning text is never authority; effects require `ActionIntent` and permit |
| [SWE-agent](https://arxiv.org/abs/2405.15793) | Agent-computer interface design materially affects software-agent performance | Keep small, typed repository and verifier tools | No raw host shell as the production interface |
| [Agentless](https://arxiv.org/abs/2407.01489) | A simple localization-repair-validation pipeline is a strong baseline | Single-worker, bounded workflows are the default | Multi-agent must beat this baseline on accepted outcome, cost and recovery |
| [RouteLLM](https://arxiv.org/abs/2406.18665) | Learned routing can trade cost against response quality | `RoutePlanner` may use learned candidate scores | User pin, privacy, capability and budget are hard filters before scoring |
| [AdaptOrch](https://arxiv.org/abs/2602.16873) | DAG structure can inform topology selection | Width, depth and coupling may generate topology candidates | Recent preprint; shadow evaluation only, never a default production claim |
| [Lost in the Middle](https://arxiv.org/abs/2307.03172) | Long context is not used uniformly | Compile, rank and seal a bounded `ContextPack` | Never dump an entire repository or transcript by default |
| [MemGPT](https://arxiv.org/abs/2310.08560) | Tiered context and memory movement can support long-running work | Separate working context, Task outcomes, source knowledge and personal memory | Summaries and indexes remain derived, versioned views |
| [Jev-Mem](https://arxiv.org/abs/2609.23986) | A fast controller can reduce memory-management cost | Candidate design for retrieval control-plane experiments | Very recent preprint and LLM-judge metric; no critical-path adoption yet |
| [System One coherence study](https://arxiv.org/abs/2609.33971) | Typed decision probabilities can disagree across decompositions | Evaluate accuracy, calibration and coherence per question family | S1 must abstain and cannot issue permits or close evidence |
| [Option-label sensitivity in Jev](https://arxiv.org/abs/2609.26758) | Schema-valid choices can follow label polarity rather than the bound rubric | Pin neutral option IDs/rubrics and test permutations on Custos fixtures | Zero type errors does not mean correct or safe decisions |
| [Typed decision-model evidence audit](https://arxiv.org/abs/2609.32160) | Early studies do not establish an independent accuracy gain for typed readouts | Compare rules, label-probability baselines, Jev/Laya and abstention on identical tasks | Nine-day early-literature review; no universal backend ranking |
| [Indirect prompt injection](https://arxiv.org/abs/2302.12173) | Retrieved content can manipulate an LLM-integrated application | Treat repository, web, email, PDF and MCP content as untrusted data | Provenance tags, channel isolation, least capability and output validation |
| [Temporal durable execution](https://docs.temporal.io/temporal) | Recovery requires persisted execution history and deterministic resume semantics | Adopt durable checkpoints, leases and explicit retry semantics | Do not add Temporal as a dependency until single-node requirements justify it |
| [Local-first software](https://martin.kleppmann.com/2019/10/23/local-first-at-onward.html) | Local ownership, offline availability and user control are architectural properties | SQLite/CAS are canonical for the first deployment profile | Multi-device sync is a later protocol, not shared SQLite over a network filesystem |
| [W3C PROV-O](https://www.w3.org/TR/prov-o/) | Provenance links entities, activities and agents | Map Source/Artifact/Evidence/Worker relations to a small provenance vocabulary | Do not require RDF in the trusted core; export it when interoperability is useful |
| [in-toto](https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias) and [SLSA provenance](https://slsa.dev/spec/v1.2/provenance) | Verifiable materials, products, actors and steps form an evidence chain | Evidence records bind subject digests, verifier identity and producing activity | A hash proves identity/integrity, not semantic correctness |
| [NIST AI RMF](https://www.nist.gov/itl/ai-risk-management-framework) and [SSDF](https://csrc.nist.gov/pubs/sp/800/218/final) | AI and software risk must be governed, measured and managed throughout the lifecycle | Threat models, evals, incident records and secure release gates are first-class | Compliance labels do not replace failure-path testing |

## Protocol and upstream disposition

| Input | What to reuse | What Custos must retain |
|---|---|---|
| [MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28) | Tool/resource transport, capability discovery, authorization profile | Exact local payload authority, data classification, receipts and evidence |
| [ACP v2](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/overview.mdx) | Editor-to-coding-agent session, updates, cancellation and permission requests | Custos Task identity, durable Run, native-tool assurance label |
| [A2A v1](https://a2a-protocol.org/latest/specification) | Remote agent discovery, task transport, streaming and artifacts | Translation between remote task and Custos Task; no identity or authority conflation |
| [Goose architecture](https://github.com/aaif-goose/goose/blob/main/documentation/docs/goose-architecture/goose-architecture.md) | Mature loop, MCP/ACP mechanics, provider streaming and client UX | Kernel, persistence, permits, evidence and completion remain Custos-owned |
| [9Router architecture](https://github.com/decolua/9router/blob/master/docs/ARCHITECTURE.md) | Optional OpenAI-compatible transport, translation, health/fallback experiments | One active transport per attempt; transformations and fallbacks recorded in provenance |
| [agentgateway](https://agentgateway.dev/docs/standalone/latest/documentation/about/) | Optional LLM/MCP/A2A connectivity, auth, policy, traffic and observability edge | It is a replaceable sidecar and never the canonical Task, budget or evidence store |
| [OpenTelemetry](https://opentelemetry.io/docs/concepts/signals/) | Trace, metric and log correlation vocabulary | Audit/evidence records are durable product state, not disposable telemetry |

The operational extraction and admission method for the three upstream systems
is defined in [`upstream-dissection.md`](upstream-dissection.md).

## Claims intentionally quarantined

- Vendor speed, price and token-saving ratios are hypotheses until reproduced on
  Custos fixtures with pinned model, hardware, prompt, cache and pricing date.
- A typed output is schema-valid, not necessarily correct, calibrated or secure.
- LLM-as-a-judge can be a signal, but not the sole completion verifier.
- Multi-agent topologies are not a maturity level. They are candidates evaluated
  against the single-worker baseline.
- MCP, ACP and A2A solve different interoperability boundaries and do not replace
  the Local API or each other.
- Nine named protocols are not nine mandatory dependencies. Protocols enter only
  when a shipping user journey and conformance suite require them.

## Current repository consequence

The trusted core remains Rust. TypeScript belongs to UI/IDE/client surfaces;
Python belongs to repository intelligence, evaluation and bounded sidecars;
SQL owns additive migrations; JSON Schema owns cross-process payloads; YAML owns
validated pack declarations. The concrete topology is defined in
[`../architecture/polyglot-repository-topology.md`](../architecture/polyglot-repository-topology.md).

## Immediate durability alert

The current `rusqlite 0.31` bundled dependency resolves to SQLite 3.45.0.
SQLite documents a WAL-reset corruption bug affecting 3.7.0 through 3.51.2,
fixed in 3.51.3 and selected backports. Before a durability release, Custos must
upgrade to a fixed SQLite build, add a runtime version assertion, and execute
concurrent writer/checkpoint crash tests. See the official [SQLite WAL
documentation](https://www.sqlite.org/wal.html#the_wal_reset_bug).

## Second-pass review: differentiated developer workflows

Reviewed on 2026-09-30 against the submitted Vietnamese specification and the
current daemon/port/completion source. The following are design deductions from
primary sources, not claims of implemented capability or exhaustive competitive
coverage. Sources on moving branches need commit pins before code extraction.

| Primary source | Observed mechanism or result | Custos design consequence |
|---|---|---|
| [LangGraph persistence](https://docs.langchain.com/oss/python/langgraph/persistence) | Checkpoints, interruption recovery, and cross-thread stores already exist | Persistence alone is not a defensible differentiation claim; connect recovery to authority and evidence applicability |
| [OpenHands SDK architecture](https://docs.openhands.dev/sdk/arch/overview) | Typed tools, events, workspace abstractions, and security policies | Reuse interface/isolation lessons; compare complete workflows rather than claiming competing agents lack safety |
| [Anthropic research system](https://www.anthropic.com/engineering/multi-agent-research-system) | Parallel research benefits independent investigation but incurs coordination and token costs | Parallelize separable source investigations; coding writes need explicit conflict handling and an equal-budget baseline |
| [Build Systems a la Carte](https://www.microsoft.com/en-us/research/wp-content/uploads/2018/03/build-systems-5ab0f42d0f937.pdf) | Dependency tracking and rebuilding strategies are established incremental-computation ideas | Treat assessment reuse as dependency-sensitive computation; incomplete capture forces conservative invalidation |
| [FActScore](https://arxiv.org/abs/2305.14251) | Fine-grained factual support is evaluated at atomic-claim level | Store claim/support references and separate located citations from semantic support |
| [tau-bench](https://arxiv.org/abs/2406.12045) | Evaluates goal-state correctness and repeated-run reliability in tool/user interactions | Assistant evals check actual connector state, recipient/time/payload, duplicate effects, and repeatability |
| [SQLite synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous) | Synchronization affects database writes through connection/schema settings | Do not promise per-table FULL/NORMAL/OFF durability inside one canonical database |
| [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) and [AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation) | Process grouping/resource control and application isolation have different responsibilities | A Job Object alone is not the sandbox boundary; test filesystem, process, credential, and network denial |
| [MCP security practices](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices) | Authorization, confused-deputy, token, and network threats require protocol-specific controls | Discovery metadata is untrusted; bind account/audience/scope and preserve local payload authorization |

## Corrections to the supplied specification

### Submitted-section traceability

| Submitted section | Final treatment | Normative location |
|---|---|---|
| 1. Product | Keep Task-centered local-first developer work cycle; verify differentiation empirically | Architecture 1, 13, 20 |
| 2. Ten invariants | Keep authority/evidence/recovery; separate human acceptance from machine verification | Architecture 2, 4-5, 16 |
| 3. Ten layers | Recast as five logical responsibility planes over the current eleven crates; correct dependency direction | Architecture 1, 3, 7, 10 |
| 4. Twelve objects and SQL | Keep conceptual objects; reject extra Task states and unreviewed SQL as migration | Architecture 4-5; C-01 to C-04 review |
| 5. Seven flows | Retain intent/context/reasoning/authority/effect/evidence/recovery, add failure windows and cross-pack handoff | Runtime flows F1-F8 |
| 6. Jev and Jev-Mem | Optional typed judgment/memory-controller experiments; require abstention, rubric and calibration evals | Architecture 6, 15-16 |
| 7. OI and five frameworks | Single Runtime coordinator; filter hard constraints before ranking; one worker by default | Architecture 6.1/6.4, 19-20 |
| 8. Three packs | Keep templates and verifiers; compute risk per operation; tool lists are proposed namespaces | Architecture 9, 16 |
| 9. Nine protocols | Local API and needed MCP first; ACP/A2A only for admitted workflows; rest watch-list | Architecture 7, 17 |
| 10. Security | Enforce permit/effect/sandbox/egress/secret boundaries; no single filter is sufficient | Architecture 2-3, 8, 18 |
| 11. Eight gates | Reorder around reproducibility, P0 trust/durability closure and one vertical slice | Architecture 12; sprint status |
| 12-13. Principles and slogan | Keep constraints, remove numeric architecture/market superlatives | Architecture 1-2, 13, 20 |

### Orchestration source admission

| Source checked on 2026-09-30 | What is actually reusable | Custos admission |
|---|---|---|
| [AdaptOrch preprint](https://arxiv.org/abs/2602.16873) | DAG width/depth/coupling as candidate features; paper reports 12-23% gains in its settings | Shadow route planner after single-worker baseline; no automatic topology claim |
| [AgentConductor preprint](https://arxiv.org/abs/2602.17100) | Feedback-driven topology learning; paper reports up to 14.6% pass@1 and 68% token reduction on studied datasets | Offline research only; competition-code result does not establish safer real-repo effects |
| [DyTopo preprint](https://arxiv.org/abs/2602.06039) and [CADTopo implementation](https://github.com/code0-tech/cadtopo) | Need/offer descriptors and per-round selective communication | Optional candidate generation; manager output is untrusted and every round remains bounded |
| [Jarvis Core 0.16.1](https://pypi.org/project/jarvis-agent-core/) | Deterministic tier selection, cheapest eligible candidate, bounded escalation | Extract policy idea into Runtime; no Python runtime dependency required |
| [Mahoraga implementation](https://github.com/pockanoodles/Mahoraga) | Local-judge-cloud cascade and reproducible benchmark artifacts | Evaluate on Custos fixtures including false accepts; its reported 76.5% cost saving is not transferable |

These sources address different optimization problems; they are not five
services to install. Hard safety/privacy constraints precede every model or
topology ranking, and only recorded outcomes can update a future route policy.

### Design corrections

| Submitted design | Resolution |
|---|---|
| Trusted core described as four additional crates | Keep four logical responsibilities within existing `custos-core`; preserve the eleven-crate workspace |
| Enlarged Task enum with Ready/Verifying/Limited/Reconciling | Retain the current Task enum; derive those UI states from independent Run, Effect, Approval, Criterion, and Outcome records |
| Different SQLite synchronous modes assigned to tables | One deliberate canonical durable writer profile; disposable cache/telemetry may use separate storage |
| Usage events classified entirely as disposable telemetry | Budget reservations and settlement are authoritative state; only exported metrics may be disposable |
| Capability execute accepts intent without explicit permit | Bind validated permit, attempt identity, and expected versions at dispatch; current trait already accepts a permit |
| Model usage and capability receipt as global accessor state | Usage/receipt belong to a particular request/attempt; concurrent calls cannot share ambiguous mutable result slots |
| Permit check forbids consumed and revoked timestamps together | Preserve both lifecycle facts when applicable; enforce eligibility and atomic consumption rather than deleting revocation history |
| Evidence SQL omits explicit Task/revision identity | Require subject/Task/criterion revision, trusted verifier identity, artifacts, receipts, environment, and lineage through reviewed C-04 |
| All engineering effects are low-risk and research is read-only | Risk depends on action, target, data and recipient: tests execute code; research can download, execute notebooks, or publish |
| Exit code, citation, or approval presented as enough verification | Bind checks to declared criteria; approval establishes authority, receipt establishes observed execution, semantic correctness remains separate |
| Runtime calibration inferred from one confidence value | Monitor labeled outcomes and held-out task classes; individual responses can fail validation but do not establish calibration |
| Repeat identical S1 questions to recover truth | Correlated samples may reinforce error; bound retry by measured value and retain deterministic fallback or abstention |
| Nine protocols and three MCP processes required | Add adapters per shipping flow; expose logical pack namespaces without forcing separate servers |
| ContinuationPacket implies cross-device safe execution | Require ownership fencing, artifact transfer, uncertainty resolution, and fresh local authorization |
| Fully specified / unique / strongest language | Treat architecture as a versioned design with unresolved contracts and measurable product hypotheses |

## Research admission levels

- Established mechanism: dependency tracking, explicit state machines, durable
  intent, versioned provenance, typed boundaries, least privilege.
- Supported experiment: single-worker repair baselines, learned model routing,
  claim-level evaluation, bounded parallel research.
- Early experiment: AdaptOrch topology selection, Jev-Mem, Jev/Laya judgments.
  Their evaluation settings and limitations do not transfer automatically.
- Verified only as source-reported, not independently reproduced: the
  AgentConductor abstract's benchmark numbers, Jarvis Core release features,
  CADTopo mechanisms, and Mahoraga's benchmark method/result. Their results
  are not admitted as Custos performance promises.
- Not independently established in this pass: the attachment's exact Jev
  pricing/latency across endpoints, hidden-uncertainty anecdote, community
  audit ratios, and Jev-Mem transfer to Custos workloads. None is a required
  product contract.

The defensible hypothesis is a joined Research-to-Code-to-Assistant workflow
whose results carry dependencies, assurance, cost, and recovery state. Test it
by changing sources, merging patches, switching providers, cancelling work, and
killing processes at dispatch boundaries. Report improvements only after
matched comparisons with competing or simplified baselines; architecture
novelty and superior performance remain unproven.
