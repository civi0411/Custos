# Vi workspace — AI systems and product intelligence

**Status:** active ownership orientation. Current work items live in
[`SPRINT_STATUS.md`](../SPRINT_STATUS.md); this page does not create a backlog or
certify implementation. Repository policy remains [`AGENTS.md`](../../AGENTS.md).

## Primary responsibility

Vi owns product semantics, model and role routing, context quality, memory
policy, provider ports/adapters, domain packs, repository intelligence, and
evaluations. Current package groups are:

- `crates/runtime/custos-cognitive`, `custos-context`,
  `custos-context-management`, and `custos-memory-service`;
- `crates/core/custos-provider-sdk` and `custos-provider-types`;
- `crates/adapters/providers/**`, `custos-providers`, and
  `custos-local-inference`;
- `crates/adapters/judgments/**`, `crates/packs/**`,
  `tools/repo_intelligent/**`, and `evals/**`.

## Boundary rules

- Routing policy proposes a role/candidate; it cannot widen Task authority,
  egress policy, budget, or tool permissions.
- Provider wire formats stay in adapters. Runtime policy uses provider ports.
- Nexus is a derived discovery index. Exact source bytes and versioned snapshots
  remain the evidence authority.
- New role, routing, context, or pack behavior requires an evaluation fixture
  and a declared abstention/failure mode.
- Coordinate with Truong on privacy, egress, evidence, and completion; coordinate
  with Vinh on runtime handoff, cancellation, and external-agent behavior.

## Historical material

Any dated report under this workspace is a historical author record. It may
contain old paths, branches, estimates, or feature claims and must not be used
as an agent instruction or current implementation status.
