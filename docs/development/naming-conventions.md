# Custos naming and compatibility policy

**Document ID:** DEV-NAME-01. **Status:** lead-directed policy. **Date:** 2026-09-27.

## Canonical product names

Use `Custos` in product prose and `custos` in machine identifiers. Rust packages use `custos-<capability>`, Rust modules use `custos_<capability>`, JavaScript packages use `custos-*` or the future organization scope, and environment variables use `CUSTOS_*`. Name packages by responsibility, not by the upstream repository that inspired their implementation.

Preferred responsibility terms are `model-router`, `agent-runtime`, `protocol-adapter`, `capability-gateway`, `session`, `workflow`, `context`, `evidence`, `desktop`, and `repo-intelligence`. Do not use `hub`, `engine`, `manager`, or `core` without a narrower owned responsibility in the package README.

## When `goose` is valid

`goose` remains valid only when it identifies an external or compatibility contract:

- an upstream repository, license, commit, package or documentation citation;
- an exact Goose protocol namespace such as `_goose/unstable/*`;
- a legacy configuration key, filesystem location, URI scheme or serialized field that Custos intentionally reads during migration;
- a compatibility adapter/type whose public name cannot change without a versioned transition;
- an immutable fixture demonstrating Goose input/output behavior.

Every production compatibility occurrence must have a removal condition or be explicitly permanent. Internal implementation, telemetry fields, UI copy, package names and new APIs use Custos names.

## Migration pattern

1. Introduce the `CUSTOS_*` key, Custos protocol name or canonical type.
2. Read the Custos form first and the Goose form second. Emit a structured deprecation event when the fallback is used; never log a secret value.
3. Continue writing only the Custos form unless a versioned interoperability mode explicitly requires Goose output.
4. Provide import/migration tooling for `.goose`, `goose+roam`, stored settings and deep links before removing compatibility reads.
5. Remove the legacy form only at a documented breaking-version gate with fixtures for old and new data.

Do not alias security-sensitive permits, authority decisions, effect receipts or canonical Task state through an unversioned Goose DTO. Translate them at the boundary and validate the Custos contract.

## File and directory names

Custos-owned folders and files use responsibility-based names. Upstream material is isolated under `docs/vendor/goose/`, a compatibility adapter, or a clearly labelled fixture area. A directory that still builds or distributes an upstream Goose binary keeps its upstream name until the binary is replaced; merely changing the folder label would misrepresent provenance.

Generated output under `target/`, build caches and lock-derived artifacts are not source naming debt. Rebuild them after source/package migrations; do not use them to decide source ownership.
