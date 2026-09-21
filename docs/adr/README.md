# Architecture Decision Records (ADRs)

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§39.3) & Part VII (§59) Canonical Specification

The Custos decision repository adheres to the standardized [ADR (Architectural Decision Records)](https://adr.github.io/) format. Each record captures a significant architectural decision, context, evaluated trade-offs, considered alternatives, and technical consequences.

---

## ADR Index

| ADR ID | Decision Title | Status | Scope / Domain |
|---|---|---|---|
| **ADR-0001** | Product category and Task as core unit | Accepted | Product / Core |
| **ADR-0002** | Local daemon and trust boundary | Accepted | Architecture / Security |
| **ADR-0003** | SQLite event/outbox persistence | Accepted | Storage / Persistence |
| **ADR-0004** | Typed internal communication | Accepted | Communication |
| **ADR-0005** | MCP only at tool/resource boundaries | Accepted | Gateway / Tools |
| **ADR-0006** | ProviderPort and capability probes | Accepted | Provider / Interop |
| **ADR-0007** | Domain Packs and ephemeral workers | Accepted | Agent / Execution |
| **ADR-0008** | Worktree mutation isolation | Accepted | Execution / Git |
| **ADR-0009** | Exact-payload approvals | Accepted | Security / HITL |
| **ADR-0010** | ContextPack and provenance | Accepted | Context / Memory |
| **ADR-0011** | Evidence-based completion | Accepted | Verification |
| **ADR-0012** | Rust/TypeScript/Python split | Accepted | Codebase / Stack |
| **ADR-0013** | Cognitive Control Fabric | Accepted | Cognitive / Architecture |
| **ADR-0014** | RDC protocol (Request-Decision-Challenge) | Accepted | Cognitive / Protocols |
| **ADR-0015** | JudgmentPort vs DeliberationPort | Accepted | Cognitive / Interfaces |
| **ADR-0016** | Confidence is not authority | Accepted | Security / Principles |
| **ADR-0017** | Versioned Question Registry | Accepted | Cognitive / Evaluation |
| **ADR-0018** | Calibration profiles | Accepted | Cognitive / Tuning |
| **ADR-0019** | Jev egress/local fallback | Accepted | Cognitive / Adapters |
| **ADR-0020** | Reflexive challenge | Accepted | Cognitive / Safety |
| **ADR-0021** | Human Attention Packet | Accepted | UX / HITL |
| **ADR-0022** | Decision Ledger | Accepted | Audit / Persistence |
| **ADR-0023** | Memory promotion/invalidation | Accepted | Memory / Knowledge |
| **ADR-0024** | Artifact-addressed large payloads (CAS) | Accepted | Storage / Performance |
| **ADR-0025** | A2A deferred to federation horizon | Accepted | Architecture / Scope |
| **ADR-0026** | System One is a pluggable capability, not a vendor model | Accepted | Cognitive / Extensibility |
| **ADR-0027** | Custos-owned semantic core | Accepted | Architecture / Core |
| **ADR-0028** | OSS adoption levels and no-copy default | Accepted | Development / Governance |
| **ADR-0029** | Minimal SQLite workflow instead of external engine for MVP | Accepted | Storage / MVP |
| **ADR-0030** | Official MCP SDK without protocol mesh in MVP | Accepted | Gateway / MCP |
| **ADR-0031** | Provider sessions are not generic model calls | Accepted | Provider / Sessions |
| **ADR-0032** | Cedar assists authorization but does not own grants | Accepted | Security / Auth |
| **ADR-0033** | Derived vector/graph indexes are non-canonical | Accepted | Storage / Indexes |
| **ADR-0034** | Tiered sandbox backends (macOS Seatbelt / Linux bubblewrap) | Accepted | Security / Sandbox |
| **ADR-0035** | Shadow onboarding for judgment backends | Accepted | Cognitive / Testing |
| **ADR-0036** | VS Code first, desktop shell later | Accepted | UX / Clients |
| **ADR-0037** | Research/Personal packs after engineering gate | Accepted | Roadmap / Scope |

---

## Architectural Decision Record Template

Detailed ADR records created in `docs/adr/ADR-xxxx.md` follow this structure:

```markdown
# ADR-xxxx: [Decision Title]

## Context & Problem Statement
Describe the technical context, motivations, and requirements necessitating this architectural choice.

## Decision Outcome
The specific accepted decision and architectural approach adopted.

## Considered Options
- Option A: [Pros / Cons]
- Option B: [Pros / Cons]

## Consequences
- Positive: Expected benefits and architectural advantages.
- Negative / Risks: Technical trade-offs, added complexity, or operational constraints.

## Verification & References
Test suites, benchmarks, RFCs, or PRs validating the implementation of this decision.
```
