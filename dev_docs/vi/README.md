# Vi's Workspace (AI Engineer)

> **Role:** Core Architect & AI Engineer  
> **Modules Owned:** `crates/core-domain`, `crates/cognitive-runtime`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.

---

## Focus Areas
1. **Core Domain:** Design data models, traits, state transitions, and JSON schemas. Ensure `crates/core-domain` remains the Single Source of Truth, strictly Zero I/O, with zero internal crate dependencies.
2. **Cognitive Control Fabric:** Build the two-tier reasoning architecture:
   - *System 1:* Fast heuristic reflection and risk triage via Python sidecar.
   - *System 2:* Deep reasoning via LLM APIs and TypeScript sidecar.
3. **Prompt & Context Engineering:** Context compiler, token budgeting, prompt versioning.

---

## Directory Organization
- `notes/`: Technical notes, draft schemas, prompt designs, and research notes.
- `reports/`: Personal progress reports by day/sprint for peer review on the `report` branch.

---

## Active Checklist
- [ ] Add required fields to `Task` and `TaskStatus` in `crates/core-domain`.
- [ ] Define request/response action payload interfaces.
