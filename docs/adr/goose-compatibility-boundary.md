# ADR: Isolate Goose compatibility from Custos identity

**ADR ID:** ADR-COMPAT-01. **Status:** Lead-directed; implementation staged. **Date:** 2026-09-27. **Owners:** Vi coordinates taxonomy, Vinh owns protocol/UI migration, Truong reviews config, release and security effects.

## Context

Custos contains code and packages derived from Goose. The current tree mixes three different meanings under the same name: exact upstream identity, compatibility with existing Goose data/protocols, and accidental product/internal naming. A global text replacement would break `_goose/unstable/*`, existing `GOOSE_*` configuration, `.goose` data, deep links, published package identities and user installations. Leaving every name unchanged makes Custos ownership and release behavior ambiguous.

Nexus and source scans measured the scope in [STATUS-NAME-01](../status/goose-naming-migration.md). Caller tracing also shows that legacy configuration accessors participate in multiple runtime paths rather than existing as isolated labels.

## Decision

1. Custos-owned product, package, telemetry and new API names use Custos terminology.
2. Exact upstream names remain unchanged in vendor references, licenses, immutable fixtures and packages that still distribute upstream artifacts.
3. Legacy wire/data/config names live behind an explicit compatibility boundary. Custos introduces a canonical name first, reads the legacy form second and records a redacted deprecation event.
4. No Goose DTO may become the canonical Task, authority, permit, effect or evidence contract. Translation and validation occur at the boundary.
5. Directory renames follow responsibility and ownership. A fork must become either an independently owned Custos package or an isolated vendor dependency; hybrid identity is temporary debt.
6. Removal requires a breaking-version gate, migration fixtures and release notes. Compatibility is not removed by search-and-replace.

## Consequences

- Product identity can be corrected immediately without silently breaking existing users.
- Temporary aliases and dual-read logic increase maintenance during N1–N5.
- Compatibility code becomes measurable and removable instead of spreading through new code.
- Vendor provenance stays visible for license, SBOM and upstream-update work.

## Verification

- [`scripts/check_naming.sh`](../../scripts/check_naming.sh) blocks new Goose-named Custos application packages and developer-specific Nexus paths.
- [`STATUS-NAME-01`](../status/goose-naming-migration.md) records the measured inventory, hotspots, target paths and N0–N6 gates.
- Configuration and protocol removals require old/new fixtures and the exit evidence listed in that register.
