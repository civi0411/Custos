# Vinh workspace — agent systems and coordination

**Status:** active ownership orientation. Current work items live in
[`SPRINT_STATUS.md`](../SPRINT_STATUS.md); this page does not create a backlog or
certify implementation. Repository policy remains [`AGENTS.md`](../../AGENTS.md).

## Primary responsibility

Vinh owns Session continuity, bridge semantics, Run/Step workflow, worker and
external-agent lifecycle, MCP protocol lifecycle, thin clients, UI, and
distribution. Current package groups are:

- `crates/runtime/custos-session`, `custos-workflow`, and `custos-agent`;
- `crates/core/custos-bridge`;
- `crates/adapters/custos-mcp`;
- `crates/app/custos-cli`, `custos-vscode`, `ui/**`, and `packages/**`.

`crates/runtime/custos-engine` is an out-of-workspace imported source pool. It
is not a production runtime owner or a source of active instructions; extract a
bounded capability only after dependency, license, contract, and test review.

## Boundary rules

- Session, Task, Run, and external-agent session are different identities.
- Workflow owns bounded Run/Step state; only the Kernel mutates canonical Task
  state.
- MCP, ACP, A2A, and CLI adapters are distinct contracts. Do not label an
  external agent `Custos-mediated` unless native effects are actually visible
  and intercepted.
- Cancellation, leases, checkpoints, handoffs, and uncertain effects require
  explicit durable semantics and failure tests.
- Coordinate with Truong on persistence/effects and with Vi on role/context
  handoffs and product outcome semantics.

## Historical reports

- [`tuan-01.md`](reports/tuan-01.md)
- [`tuan-02.md`](reports/tuan-02.md)

These reports are immutable historical author records. They contain old paths,
research hypotheses, and unpinned claims. Agents must not treat them as active
work or implementation evidence.
