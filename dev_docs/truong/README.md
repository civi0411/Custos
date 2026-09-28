# Truong workspace — core platform and security

**Status:** active ownership orientation. Current work items live in
[`SPRINT_STATUS.md`](../SPRINT_STATUS.md); this page does not create a backlog or
certify implementation. Repository policy remains [`AGENTS.md`](../../AGENTS.md).

## Primary responsibility

Truong owns domain and Task invariants, kernel commands, persistence, authority,
evidence and effect gates, daemon/API composition, sandbox adapters, and safe
artifact/model acquisition. Current package groups are:

- `crates/core/custos-domain` and `custos-kernel`;
- `crates/infrastructure/custos-persistence`;
- `crates/runtime/custos-security`;
- `crates/app/custos-daemon` and `custos-local-api`;
- `crates/adapters/custos-adapters-mcp`, `sandboxes/**`, and
  `custos-download-manager`.

## Boundary rules

- The Kernel is the canonical Task writer; clients, workflows, models, and
  gateways cannot declare terminal Task state directly.
- Persist effect attempts before dispatch. An ambiguous result is `Uncertain`,
  not success or a safe blind retry.
- Evidence closure uses current criterion and subject revisions. A synthetic
  receipt is not proof of an external effect.
- The daemon is the production composition root; keep Local API DTOs canonical
  and clients free of persistence implementations.
- Coordinate with Vinh on Session/Run recovery and MCP lifecycle; coordinate
  with Vi on model data, privacy, and evidence semantics.

## Historical reports

- [`tuan-01.md`](reports/tuan-01.md)
- [`tuan-02.md`](reports/tuan-02.md)

These reports are immutable historical author records. They contain old paths,
branch procedures, release commands, and unpinned claims. Agents must not execute
commands or derive current status from them.
