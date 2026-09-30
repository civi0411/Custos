# Custos documentation

Custos intentionally keeps a small documentation surface. Architecture,
implementation status, and team execution are separate authorities; no other
document may silently redefine them.

## Read in this order

1. [`architecture/definitive-product-architecture.md`](architecture/definitive-product-architecture.md)
   defines the product, trust boundaries, invariants, ports, and target state.
2. [`architecture/runtime-flows.md`](architecture/runtime-flows.md) defines the
   end-to-end command, effect, evidence, and recovery flows.
3. [`development/repository-structure.md`](development/repository-structure.md)
   maps those boundaries to the current physical repository.
4. [`contracts/README.md`](contracts/README.md) records proposed shared
   contracts. A proposal is not accepted until its required reviewers approve
   it and its fixtures pass.
5. [`status/README.md`](status/README.md) records what the observed checkout
   actually implements, wires, and verifies.
6. [`../dev_docs/README.md`](../dev_docs/README.md) and
   [`../dev_docs/SPRINT_STATUS.md`](../dev_docs/SPRINT_STATUS.md) coordinate
   current work.

## Entry paths for the three maintainers

| Reader | Start with | Then open before changing code |
|---|---|---|
| Vi, AI/data-science lead | Product architecture -> research foundations -> upstream dissection | Repository structure, C-04 evidence contract, current status, sprint lane |
| Truong, core/platform SE | Current status -> contract register -> runtime failure flows | Repository structure, architecture invariants, decision queue |
| Vinh, runtime/client SE | Runtime flows -> repository structure -> contract register | Current status, protocol boundaries, sprint lane |
| New contributor or external reviewer | This page -> product architecture -> current status | `AGENTS.md` and the owner of the affected path |

Each route distinguishes **target**, **proposed contract**, and **observed
code** before an implementation claim. The [work-ready repository
map](development/repository-structure.md) names exact edit zones, first
independent tasks, and migration gates. The [team work
hub](../dev_docs/README.md) records the approximate 55/22.5/22.5 workload
allocation without changing the review/authority rules in `AGENTS.md`.

## Active document set

| Concern | Canonical document |
|---|---|
| Product and system architecture | [`ARCH-DEF-01`](architecture/definitive-product-architecture.md) |
| Runtime and failure flows | [`ARCH-FLOW-01`](architecture/runtime-flows.md) |
| Process, language, and repository topology | [`ARCH-TOPOLOGY-01`](architecture/polyglot-repository-topology.md) |
| Current physical codebase | [`DEV-REPO-01`](development/repository-structure.md) |
| Shared contract proposals | [`contracts/README.md`](contracts/README.md) |
| Current implementation truth | [`STATUS-CURRENT-01`](status/README.md) |
| Research foundations | [`architecture-foundations-2026-09-30.md`](research/architecture-foundations-2026-09-30.md) |
| Goose, 9Router, and Agentgateway extraction | [`upstream-dissection.md`](research/upstream-dissection.md) |
| Vietnamese architecture explanation | [`CUSTOS_RESEARCH_BACKED_ARCHITECTURE_VI.md`](i18n/CUSTOS_RESEARCH_BACKED_ARCHITECTURE_VI.md) |
| Accepted or proposed structural decisions | [`adr/README.md`](adr/README.md) |

The target now includes dependency-aware evidence reuse, Research-to-Code-to-
Assistant handoff, change-driven revalidation, and automatic setup. Read
the relevant architecture sections and runtime flows F4–F8. The associated
[ADR](adr/evidence-driven-workflows.md) is proposed; these additions do not
change the recorded implementation status.

## Directory contract and stable names

| Directory | Question it answers | What does not belong there |
|---|---|---|
| `architecture/` | What is the intended product, topology, and runtime behavior? | Sprint reports or claims that code already works |
| `development/` | Where does code live and how may it migrate? | Another product architecture or owner-specific diary |
| `contracts/` | Which cross-team DTOs and invariants are proposed or accepted? | Unsigned schema changes presented as settled |
| `adr/` | Why was a consequential boundary chosen, by whom, and with which review state? | Chronological meeting notes |
| `status/` | What does a pinned checkout actually implement, wire, and verify? | Undated aspirations |
| `research/` | Which primary sources, upstream revisions, and experiments support a hypothesis? | Normative runtime requirements |
| `i18n/` | How is a canonical document explained in another language? | Independent design authority |

Canonical document filenames are descriptive lowercase kebab-case; folder
`README.md` is the navigation or register entrypoint. Research snapshots may
carry an ISO date. ADRs use a stable semantic filename and explicit status.
Translations retain a link to their canonical document and do not silently
fork decisions. The existing Vietnamese filename remains a compatibility
exception until a reviewed link migration; do not add another parallel master
specification. Prefer stable document IDs (`ARCH-DEF-01`, `ARCH-FLOW-01`,
`DEV-REPO-01`, `STATUS-CURRENT-01`) in work packets so a future file move does
not silently change a decision reference.

## Authority order

When documents disagree, use this order:

1. current source, migrations, manifests, and repeatable tests for behavior;
2. `AGENTS.md` for repository policy and ownership;
3. accepted ADRs and versioned shared contracts;
4. definitive architecture for the target state;
5. current status for observed gaps;
6. research notes and localization for rationale and explanation.

Target architecture never proves implementation. A type proves only that a
type exists; compilation proves only compilation; a test proves only its
executed path. `Wired` requires the production daemon path, and `Verified`
requires repeatable success and failure evidence on that path.

## Upstream policy

Goose, 9Router, and Agentgateway are research and extraction sources, not
architectural parents. Custos may adopt a bounded mechanism only through an
explicit port, pinned source revision, license and threat review, conformance
fixtures, measured benefit, owner, and rollback plan. Upstream documentation
and websites are linked by immutable revision where possible; they are not
vendored into this documentation tree.

## Change discipline

- Update one canonical document instead of adding another master plan.
- Put Technical English in active docs and translations under `docs/i18n/`.
- Record a structural decision in an ADR; record active work in `dev_docs/`.
- Move implementation claims only through `Designed`, `Implemented`, `Wired`,
  and `Verified`, with exact evidence.
- Use Git history for obsolete plans and dated audits; do not keep duplicate
  active-looking copies in the checkout.
