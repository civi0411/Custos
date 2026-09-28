# Custos — Module Ownership

This is the working ownership map for the current repository layout. The [repository structure map](../docs/development/repository-structure.md) is authoritative for package membership and architecture; this page assigns day-to-day responsibility. Ownership does not imply that a package is complete or production-ready.

| Area | Primary | Required collaboration | Scope |
|---|---|---|---|
| Domain, kernel, persistence, security, daemon/API composition, sandboxes | Truong | Vinh for lifecycle/recovery; Vi for evidence/model policy | Stable contracts, state transitions, durability, authority and controlled effects |
| Sessions, bridge, workflow, agent lifecycle, MCP protocol | Vinh | Truong for persistence/effects; Vi for role and product semantics | Resumable execution, handoff, worker lifecycle and protocol boundaries |
| Cognitive/context/memory, provider contracts and implementations, packs, repo intelligence, evals | Vi | Truong for privacy/evidence/egress; Vinh for runtime integration | Model routing, context quality, coding/research workflows and evaluation |
| Experimental `custos-gateway` | Vi is temporary caretaker | Truong + Vinh required | Frozen from production wiring until an ADR narrows it to one responsibility or removes it |
| CLI, VS Code/UI and distribution packages | Vinh | Truong for API/release; Vi for UX and outcome semantics | Thin user-facing clients and packaging |
| Shared schemas, tests, recipes and cross-cutting docs | Subject owner coordinates | Producer and consumer review | Compatibility, conformance and shared understanding |

The `custos-domain` package is assigned to Truong as platform owner. Changes to shared domain types, state transitions, public API DTOs, provider/tool ports, or effect semantics require review from Vi and Vinh before merge. This is a repository-lead direction for team organization, not evidence that those contracts are already implemented or validated.

`custos-engine` remains an out-of-workspace imported source pool. `buzz/` is
unassigned experimental material. Agents must not activate, relocate, or broadly
refactor either surface without a recorded owner, bounded extraction/adoption
decision, and relevant cross-boundary review.

## Collaboration rules

1. Keep pull requests scoped to one ownership area where practical. Cross-area changes should identify the contract being changed and include the affected owners as reviewers.
2. Do not interpret package ownership as permission to bypass security, compatibility, or evidence requirements.
3. Use the current branch policy in [`AGENTS.md`](../AGENTS.md); do not use stale personal-branch conventions from historical reports.
4. Record structural decisions in an ADR and update the architecture map when a dependency boundary changes.
5. Distinguish verified behavior from proposed target architecture in code and documentation.
