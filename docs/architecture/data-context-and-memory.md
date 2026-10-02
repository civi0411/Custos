# Data, Context & Memory Architecture

> **Classification:** Core Architectural Pillar  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md) (Parts 6 & 9).  
> **Architecture Hub:** See [Custos Architecture Overview](README.md).

Custos manages data across a strictly partitioned architecture to ensure local sovereignty, high-throughput SQLite concurrency, sub-second context compilation, and temporal consistency across long-term agent memory.

---

## 1. The Four Core Data Zones

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        FOUR CORE DATA ZONES                            │
├───────────────────┬──────────────────────┬─────────────┬───────────────┤
│ DATA ZONE         │ STORAGE ENGINE       │ MUTABILITY  │ CANONICALITY  │
├───────────────────┼──────────────────────┼─────────────┼───────────────┤
│ **1. Relational** │ SQLite (WAL Mode)    │ Mutable     │ Canonical     │
│    (State & FSM)  │                      │ (ACID)      │ Primary SSOT  │
├───────────────────┼──────────────────────┼─────────────┼───────────────┤
│ **2. CAS**        │ Content-Addressed    │ Strictly    │ Canonical     │
│    (Artifacts)    │ Flat Files (SHA-256) │ Immutable   │ Verification  │
├───────────────────┼──────────────────────┼─────────────┼───────────────┤
│ **3. Derived**    │ SQLite FTS5 / Vector │ Mutable     │ Rebuildable   │
│    (Indexes)      │ & Tree-sitter Cache  │ (Cache)     │ (Non-Primary) │
├───────────────────┼──────────────────────┼─────────────┼───────────────┤
│ **4. Worktrees**  │ Git Worktrees        │ Ephemeral   │ Scratch       │
│    (Execution)    │ (`isolated_worktree`)| Filesystem  │ Staging Only  │
└───────────────────┴──────────────────────┴─────────────┴───────────────┘
```

---

## 2. SQLite WAL Architecture & P0 Starvation Mitigation

Custos relies on SQLite for all structured persistence. On multi-threaded async architectures (Tokio), unmanaged WAL (Write-Ahead Logging) files can grow unboundedly if long-running readers block checkpoints, leading to database degradation.

### P0 Mitigation Strategy:
1. **Dedicated Single Writer Connection:** All state mutations and event commits route through a single, dedicated write thread managed via an MPSC channel.
2. **Read-Only Pool:** Read queries execute across an isolated pool of read-only connections using `PRAGMA query_only = ON`.
3. **Aggressive Checkpoint Threshold:** The write connection configures:
   ```sql
   PRAGMA journal_mode = WAL;
   PRAGMA synchronous = NORMAL;
   PRAGMA wal_autocheckpoint = 1000;
   PRAGMA busy_timeout = 5000;
   ```
4. **Passive Maintenance Pass:** The daemon executes `PRAGMA wal_checkpoint(PASSIVE)` during idle worker loops, truncating WAL files without interrupting active transactions.

---

## 3. Context Compiler: The 8-Step Pipeline

Rather than dumping entire repositories into an LLM's context window, Custos processes project files through an 8-step Context Compiler:

```mermaid
flowchart LR
    Step1["1. Scope Filter"] --> Step2["2. Sensitivity Classify"]
    Step2 --> Step3["3. Egress Filter"]
    Step3 --> Step4["4. Secret Scanner"]
    Step4 --> Step5["5. Version Select"]
    Step5 --> Step6["6. Hybrid Retrieval"]
    Step6 --> Step7["7. Structural Rank"]
    Step7 --> Step8["8. Token Budget Fit"]
    Step8 --> Pack["Immutable ContextPack"]
```

1. **Scope Filter:** Restricts scanned paths to those authorized by `TaskScope`.
2. **Sensitivity Classify:** Tags documents with security sensitivity labels.
3. **Egress Filter:** Prevents private project files from reaching external cloud models if `LocalOnly = true`.
4. **Secret Scanner:** Automatically scrubs API keys, private certificates, and passwords before prompt packaging.
5. **Version Select:** Pins retrieved files to the exact Git commit SHA of the current run.
6. **Hybrid Retrieval:** Blends AST symbol extraction (Tree-sitter), lexical match (Ripgrep), and semantic search.
7. **Structural Rank:** Prioritizes definitions, call-graph neighbors, and active test files.
8. **Token Budget Fit:** Dynamically truncates lower-ranked context blocks to fit the model's assigned token ceiling.

The output is an immutable `ContextPack` stamped with a verifiable SHA-256 provenance hash.

---

## 4. The Four Long-Term Memory Tiers

Custos structures long-term agent memory across four functional tiers:

```text
┌─────────────────────────────────────────────────────────────┐
│                    FOUR MEMORY TIERS                        │
├─────────────────┬───────────────────────────────────────────┤
│ **1. Working**  │ Active step context, transient reasoning. │
│                 │ Lifetime: Current step execution.         │
├─────────────────┼───────────────────────────────────────────┤
│ **2. Core**     │ Persona, project rules, standing grants.  │
│                 │ Lifetime: Pinned across entire task.      │
├─────────────────┼───────────────────────────────────────────┤
│ **3. Recall**   │ Session conversation, recent action logs. │
│                 │ Lifetime: Indexed history via FTS5.       │
├─────────────────┼───────────────────────────────────────────┤
│ **4. Archival** │ LongMemEval temporal facts & invariants.  │
│                 │ Lifetime: Cross-task persistent database. │
└─────────────────┴───────────────────────────────────────────┘
```

### Temporal Consistency in Archival Memory (LongMemEval)
To prevent outdated facts from corrupting future decisions, every stored user preference or project fact in `personal_facts` includes a temporal validity window:

```rust
pub struct PersonalFact {
    pub fact_id: FactId,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
    pub provenance_task_id: TaskId,
}
```
When conflicting facts are discovered, newer observations supersede expired facts, preventing hallucination cascades.

---

## 5. ContinuationPacket & Safe Resumption

When a task pauses, transitions across models, or recovers from a daemon restart, the kernel generates an immutable `ContinuationPacket`:

```rust
pub struct ContinuationPacket {
    pub task_id: TaskId,
    pub from_span: u64,
    pub to_span: u64,
    pub provider: String,
    pub model: String,
    pub task_summary: String,
    pub current_state: serde_json::Value,
    pub integrity_hash: Sha256Hash,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

The SHA-256 hash seals the state. Any tampering with memory anchors or task summaries invalidates the hash, preventing prompt injection attacks during task resumption.
