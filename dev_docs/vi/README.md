# Vi's Workspace (Product & AI/Research Lead)

> **Role:** Core Architect & AI/Research Lead  
> **Primary Modules:** `crates/core-domain`, `crates/cognitive-runtime`, `crates/deliberation-contracts`, `crates/judgment-contracts`, `crates/context-compiler`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.  
> **Language & Invariants:** Rust (domain interfaces, Zero I/O), Python (fast heuristics in sidecar), TypeScript (agent sidecars). Zero LangChain/LlamaIndex. Native HTTP/JSON-RPC protocols.

---

## 1. Core Focus Areas

1. **Core Domain Models & SSOT:**
   - Define immutable entities, value objects, and domain events in `crates/core-domain`.
   - Ensure `core-domain` has **zero external I/O** (no filesystem, no network, no database dependencies).
   - Authoritative schemas: `Task`, `TaskId`, `TaskStatus`, `TaskContract`, `ActionIntent`, `Receipt`, `ExecutionPermit`, `ContinuationPacket`.
2. **Cognitive Control Fabric:**
   - Implement the two-tier reasoning architecture:
     - *System 1 (Judgment Fabric):* Fast, low-cost reflex evaluation, risk screening, invariant checks via Python sidecar or local rules.
     - *System 2 (Deliberation Fabric):* Deep multi-step reasoning, plan synthesis, and code generation via external LLMs.
   - Design RDC (Request-Decision-Challenge) protocol communication.
3. **Context Engineering & Token Economy:**
   - Design Context Compiler algorithms: token budgeting, AST symbol extraction, relevance scoring, and selective slice assembly.
4. **AI Quality Evaluation & Benchmarking:**
   - Build offline evaluation scenarios and test fixtures (`FakeProvider`) for deterministic verification in CI.

---

## 2. Directory Structure

- `notes/`: Technical design notes, prompt schemas, RDC protocol specs, evaluation datasets, research papers.
- `reports/`: Daily sprint reports and sync logs committed to the `report` branch for peer review.

---

## 3. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress to the `report` branch, use this format:

```markdown
# Vi Progress Report — YYYY-MM-DD

## 1. Accomplished Today
- [x] Task 1 description (Crate: `core-domain`)
- [x] Task 2 description (Crate: `provider-sdk`)

## 2. Tests & Verification
- Contract tests added: `cargo test -p custos_core_domain`
- Invariant verification: Zero I/O invariants validated.

## 3. In-Flight Work & Next Steps
- Currently implementing: `FakeProvider` mock responses for `repo_explain`.
- Tomorrow: Scaffold `sidecars/python-judgment`.

## 4. Blockers & Questions for Truong
- Schema question on SQLite foreign key constraints for `ActionIntent`.
```

---

## 4. Domain & AI Logic Deliverables (Completed)

- **Core Domain Models:** Finalized `crates/core-domain` (Zero I/O), featuring immutable value objects: `Task`, `TaskStatus`, `DomainEvent`, `ActionIntent`, `Receipt`, `ExecutionPermit`.
- **Evidence Engine (Verification):** Implemented `CitationVerifier` and `ExactMatchVerifier` to objectively validate task completion evidence.
- **Provider Subsystem:** Completed `ProviderPort` trait and deterministic `FakeProvider` for reliable end-to-end testing.
- **Sidecar Scaffolding:** Initialized isolated Sidecar structures for Python (`sidecars/python-judgment`) and TypeScript (`sidecars/ts-claude-agent`).
