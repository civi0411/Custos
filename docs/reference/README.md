# Custos Reference Documentation

> **Classification:** Normative Technical Reference Suite  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Repository Documentation Hub:** See [Custos Documentation Overview](../README.md).

The `docs/reference/` directory serves as the definitive technical dictionary and lookup center for the Custos platform. Unlike architectural blueprints (`docs/architecture/`) or development guides (`docs/development/`), reference documents contain concise, information-dense, normative specifications designed for rapid developer lookup.

---

## Reference Suite Index

| Reference Document | Scope & Focus | Primary Audience |
|---|---|---|
| **[System Invariants](system-invariants.md)** | The 10 canonical system invariants (INV-01 to INV-10), 5 Golden Rules, and the automated compile/runtime enforcement matrix. | All contributors, security auditors, core engine developers |
| **[Naming Conventions](naming-conventions.md)** | Standardized casing, prefixes, and identifier rules across crates, modules, entities, CQRS commands/events, REST/IPC endpoints, database tables, and metrics. | Rust developers, API designers, pack authors |
| **[Schema & Type Mapping](schema-and-type-mapping.md)** | Wire contracts, JSON Schema to Rust struct mappings, protocol error taxonomy, and SQLite relational persistence table schemas. | Protocol implementers, bridge developers, frontend/IDE integrators |
| **[Canonical Glossary](glossary.md)** | Standardized definitions of core architectural domain concepts (ActionIntent, IBCT, Completion Gate, Taint Tracking, ContinuationPacket, etc.). | System architects, documentation writers, new team members |

---

## Reference Document Guidelines

To maintain documentation stability and prevent bloat, all documents in this directory adhere to the following principles:

1. **Lookup-Oriented:** Content is structured primarily into tables, matrices, and clear alphabetical definitions.
2. **No Narrative Sprawl:** Historical explanations, speculative designs, and background reasoning belong in `docs/architecture/`, not in `docs/reference/`.
3. **Strict SSOT Alignment:** In the event of any semantic ambiguity, `Custos.md` takes precedence.
4. **Stable Versioning:** Reference documents do not include ephemeral timestamps or temporary tags in their prose or file names.
