# Personal Operations Domain Pack

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§20) & Part VII (§55) Canonical Specification

The Personal Operations Domain Pack facilitates daily personal workflow coordination (triaging emails, organizing calendars, managing notes, tracking tasks) with strict data privacy guarantees.

---

## 1. The Autonomy Ladder

To prevent accidental email transmissions or destructive calendar modifications, Custos enforces a 5-tier Autonomy Ladder:

| Level | Permitted Capability | Default Status | Safety Mechanism |
|---|---|---|---|
| **A0** | Search and read information | Enabled in scope | Read-only access |
| **A1** | Draft composition | Enabled | Mandatory Preview mode |
| **A2** | Reversible local mutations | Requires opt-in | Undo operation supported |
| **A3** | External side effects (Sending email, updating event) | **Mandatory Approval** | Exact-Payload Approval |
| **A4** | Destructive / High-sensitivity actions | Manual execution only | AI suggests action; human triggers execution |

> [!NOTE]
> In MVP and Alpha releases, the personal operations runtime **executes strictly at levels A0 and A1**. Zero external messages or calendar commits occur without direct human confirmation.

---

## 2. Human Attention Budget

An intelligent runtime should never inundate users with alerts. Custos introduces the **Human Attention Budget**:
- Batches non-critical approval requests into designated review windows (e.g., Daily Review).
- Triages notifications: only high-risk critical path blockers ring attention chimes; routine reviews queue in the `Approval Inbox`.
- Preserves uninterrupted human focus for deep technical work.
