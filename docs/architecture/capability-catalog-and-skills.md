# Capability Catalog & Skill Organization Architecture

> **Classification:** Capability & Skill Architecture Specification  
> **Source of Truth:** Authoritatively governed by [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Architecture Overview](README.md).

---

## 1. Disambiguation of Capabilities, Skills, and Services

To prevent architectural ambiguity, Custos enforces strict boundaries between capabilities, skills, and services:

| Concept | Definition & Semantics | Canonical Architectural Owner |
|---|---|---|
| **Capability / Tool** | Typed, bounded physical operation (e.g. file query, process execution, PDF parse, email dispatch) | Declared in `platform/contracts`, executed in `platform/integrations` |
| **Skill Instruction** | Declarative instructions guiding an agent on how to execute a specific job using capabilities | Declarative markdown manifests in `workbenches/<pack>/skills/` |
| **Domain Service** | Domain semantics, business logic, and rubrics (e.g. claim verification, code AST diagnosis) | `workbenches/<pack>/src/services/` |
| **Shared Subsystem** | Generic mechanisms: context compilation, token budgeting, process lifecycle, memory CAS | `platform/engine` & `platform/storage` |
| **Worker Loop** | Reasoning and execution loop pursuing a Task within allocated budget | `platform/engine/src/execution/` |
| **Protocol Exposure** | Protocol surfaces: MCP Tools/Resources, ACP Agent, Remote A2A | `platform/integrations/src/protocols/` & `apps/daemon` |

> [!IMPORTANT]
> A `SKILL.md` instruction file provides guidance; it never grants execution authority. An MCP tool declaration exposes an interface; it does not authorize side-effects. All mutations mandate signed `ExecutionPermit` tokens issued by the kernel.

---

## 2. Cross-Workbench Capability Matrix

| Feature | Shared Platform Foundation | Domain Pack Semantics | Concrete Adapter Edge |
|---|---|---|---|
| **Code Explanation / Impact** | Source snapshot + AST query + context compiler | `workbenches/coding` rubric | Git worktree, parser, filesystem |
| **Scientific Claim Extraction** | Source CAS hashes + passage locators | `workbenches/research` claims | Document parser, headless browser |
| **Dataset & Bias Audit** | Artifact revisions + compute admission | `workbenches/quant` / `research` | Dataset readers, PBO verifier |
| **Draft & Calendar Scheduling** | Personal facts + time primitives | `workbenches/assistant` intent | Contacts, CalDAV / local calendar |
| **Notification Dispatch** | Effect attempt + two-phase outbox reconciliation | `workbenches/assistant` drafts | Mail / message connectors |
| **Context & Cache Optimization** | Context compiler, prompt cache affinity (0.1x) | Domain source coverage | Provider KV cache |

---

## 3. Dual Execution Pathways

1. **ModelPort Pathway (Custos-Governed Worker):**
   - The worker loads task metadata -> selects necessary skills -> compiles context -> invokes capabilities via Two-Phase Authority mediation -> validates typed outputs against criteria verifiers.
   - Declarative capabilities are sandboxed; un-permitted actions fail closed.

2. **AgentRuntimePort Pathway (Native Autonomous Harness):**
   - Conducts capability handshake -> provisions isolated Git worktree -> provides scoped context -> intercepts tool calls or exposes mediated MCP tools -> collects artifacts and verifies outcomes via Custos verifiers.
   - External CLI tools operating outside mediation report explicit assurance tags (`custos-mediated`, `provider-governed`, `observe-only`).

---

## 4. Workflow for Adding New Skills

1. **Define Contract:** Specify task kind, input/output schemas, and deterministic acceptance criteria.
2. **Assign Ownership:** Determine whether the logic represents an instruction, domain service, shared engine feature, or I/O integration adapter.
3. **Register Manifest:** Add the skill descriptor to the appropriate workbench manifest in `workbenches/<pack>/`.
4. **Implement Verifiers:** Provide independent verifiers validating outcomes without trusting self-reported model claims.
5. **Verify Quality Gates:** Ensure compliance via `cargo xtask check-boundaries` and integration test suites.
