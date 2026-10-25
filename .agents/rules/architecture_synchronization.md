# Architecture Decision & Documentation Synchronization Triad
## Mandatory Protocol for AI Coding Agents

> **Scope:** All AI Coding Assistants (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf) working on the Custos and AgentHub repositories.  
> **Absolute Invariant:** Any system architecture decision or modification MUST be synchronized across the 3-tier documentation structure before code changes are permitted.

---

### 1. The Documentation Triad Hierarchy

Every architectural fact and system boundary is strictly governed within three synchronized tiers:

1. **Tier 1 — `Custos.md` (Root Master Specification — SSOT):**
   - **Nature:** The Single Source of Truth (SSOT) defining core philosophy, 10 System Invariants, State Machines, Trust Zones, Threat Modeling, and Protocol Specifications.
   - **Rule:** When agreeing on a new architectural design, locate and update the corresponding Chapter/Topic in `Custos.md` (e.g. Part 2 for Layers, Part 3 for Kernel/Task, Part 4 for Authority, Part 15 for Repo Boundaries).

2. **Tier 2 — `docs/` Directory (Authoritative Topic-Based Engineering Specifications):**
   - **Nature:** Deep-dive technical specifications per functional subsystem:
     - `docs/architecture/`: 9 architectural pillars (Kernel, Authority, Evidence, Memory, Protocols, Security, Orchestration, Packs).
     - `docs/reference/`: Invariants, Naming Conventions, Schema Mapping, Glossary.
     - `docs/development/`: Engineering standards, Delivery Blueprint, Verification.
   - **Rule:** Update or create matching topical documents to ensure no divergence exists between implementation guides and `Custos.md`.

3. **Tier 3 — `docs/development/codebase-architecture.md` (Master Physical Codebase Catalog):**
   - **Nature:** Detailed physical code map tracking every source file, line counts, architectural responsibilities, and exported structs/traits across the 11 canonical product crates.
   - **Rule:** Every new, modified, or deleted file must be registered into the exact matching crate table in this document.

---

### 2. Mandatory 5-Step Synchronization Workflow

For any architectural change, follow this sequential execution pipeline:

```
[Architectural Alignment with User/Team]
               │
               ▼
   [Step 1: Update Custos.md (SSOT)]
               │
               ▼
   [Step 2: Update docs/ Topical Spec]
               │
               ▼
   [Step 3: Register in docs/development/codebase-architecture.md]
               │
               ▼
   [Step 4: Implement Code & Verify with cargo check / cargo test]
```

- **Step 1: Architectural Alignment:** Clarify which crate among the 11 canonical crates owns the logic, verify adherence to invariants, and confirm crate ownership.
- **Step 2: Update `Custos.md`:** Commit the design update into the exact chapter of `Custos.md`.
- **Step 3: Update `docs/`:** Detail functional requirements and contracts in the appropriate topical markdown file.
- **Step 4: Register in `docs/development/codebase-architecture.md`:**
  - Locate the exact Layer (Layer 0 to Layer 4) and Crate (`crates/custos-...`).
  - Insert a new row in the Crate's file table following the 4-column standard:
    `| Source File Link | Line Count | Architectural Responsibility | Key Structs / Traits / Functions |`
  - Update aggregate file and line counters in the crate header.
- **Step 5: Code Implementation & Verification:** Write source code and verify with `cargo check --workspace` and `cargo test`. Never write code prior to completing Steps 1 through 4.

---

### 3. Mental Model for AI Coding Agents

Before executing any file modification:
1. **Read `docs/development/codebase-architecture.md` first:**
   - Identify which crate and layer owns the target file, and verify all constraints.
   - Avoid creating loose, unstructured files or unapproved utility directories.
2. **Zero Guesswork Policy:**
   - If unsure whether a struct or contract belongs to `custos-domain` or `custos-core`, consult `Custos.md` and `docs/development/codebase-architecture.md`.
3. **Absolute Invariants:**
   - Do not create crates outside the 11 canonical product crates.
   - Never violate the Zero-I/O invariant of `custos-domain`.
   - Never allow `custos-bridge` to access `custos-persistence` directly.
