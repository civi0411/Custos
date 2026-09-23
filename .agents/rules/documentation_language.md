# Rule: Mandatory 100% Technical English Documentation Standard

## Purpose & Scope
This rule enforces strict language discipline across all documentation, architectural specifications, design notes, daily sprint reports, commit messages, and code comments within Custos.

## Invariants

1. **100% Technical English Only:**
   - Every file authored or modified under `docs/` and `dev_docs/` MUST be written in **100% Technical English**.
   - Absolutely no Vietnamese or mixed-language prose may be inserted into core documentation, task notes, architectural blueprints, or progress reports.
   - The **only** permitted location for non-English documentation is within explicitly designated i18n mirror files located in `docs/i18n/` (e.g., `docs/i18n/README.vi.md`).

2. **Zero Decorative Emojis:**
   - Technical documentation must remain pristine, professional, and publication-ready.
   - Do NOT include decorative emojis (e.g., 🚀, 💡, 🔥, ✨, 📌) in markdown titles, headers, bullet points, table cells, or diagrams.

3. **Authoritative & Formal Systems Engineering Tone:**
   - Write with high technical precision, academic rigor, and clear architectural boundaries.
   - Define exact contracts, typed interfaces, and deterministic state transitions.

4. **Consistency Across Team Workspaces:**
   - Applies equally to all subdirectories:
     - `docs/` (Canonical specifications)
     - `dev_docs/` (Engineering collaboration protocol & master docs)
     - `dev_docs/vi/` (AI Systems & Product Intelligence Lead workspace)
     - `dev_docs/truong/` (Core Platform & Security Lead workspace)
     - `dev_docs/vinh/` (Agent Systems & Coordination Research Engineer workspace)
