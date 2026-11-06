# Dynamic Pack Registration & Lightweight On-Demand Architecture

> **Status:** Target architecture; partial working-tree implementation, not verified release support.
>
> **Crates:** `custos-domain`, `custos-core`, `custos-persistence`, `custos-adapters`, `custos-packs`, `custos-runtime`, `custos-daemon`
>
> **Frontend:** `ui/desktop/src/components/workbenches`

**Composition, packaging and lifecycle decisions:** [Modular workbench plan §§19–25](../development/modular-workbench-refactor-plan.md#19-cập-nhật-working-tree-dùng-phần-đã-xây-đóng-các-khoảng-trống) consolidates Base presets, ownership, trusted installation, worker lifetimes and implementation gates. The manifest and RPC table below are illustrative until checked against schema/SDK/handler fixtures. Package installation, profile enablement, engine readiness and action authorization are separate states. Paths below are illustrative; production uses the platform/profile path resolver.

---

## 1. Paradigm & Anti-Bloat Architecture ("Tránh Nặng")

Custos operates as a lightweight Software Agent Development Environment (SADE). Rather than bundling heavy domain monoliths (e.g., Python Jupyter runtimes, Backtrader backtest engines, or paper-vector databases) as permanent background daemons, Custos enforces a **Strict Anti-Bloat Architecture** across five critical dimensions:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CUSTOS LIGHTWEIGHT BASE                         │
│  • Memory RSS: measure renderer + daemon + workers separately         │
│  • Single-writer SQLite; large artifacts outside canonical DB         │
│  • Core Safe Tools: [read_file, list_dir] (Minimal token prompt)       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
       ┌────────────────────────────┼────────────────────────────┐
       ▼                            ▼                            ▼
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│   Coding Pack     │      │   Research Pack   │      │    Quant Pack     │
│ (Built-in / Dev)  │      │ (Add-on Package)  │      │ (Add-on Package)  │
└───────────────────┘      └───────────────────┘      └───────────────────┘
       │                            │                            │
       ▼                            ▼                            ▼
  Git Worktrees             PDFs & Vector Chunks        Tick Data & Parquets
  ~/.custos/worktrees/      ~/.custos/data/research/    ~/.custos/data/quant/
```

### 1.1. Compute & Process Footprint: On-Demand Execution
- **The Problem:** Eager optional processes consume resources even when unused; the actual footprint must be measured per supported build and workload.
- **Custos Solution:** **JIT Ephemeral Workers via `WorkerLease`**.
  - Installing a workbench does not automatically start a background service.
  - When an operation requires execution (e.g. running a backtest or parsing an academic paper), the runtime acquires a `WorkerLease`.
  - Worker wire contracts declare deadlines and bounded outputs. Memory limits need explicit OS enforcement and capability reporting; contract fields alone do not enforce them.
  - One-shot workers are reaped after completion/cancellation according to bounded reuse policy. Notebook kernels, terminals and remote training have explicit stateful leases; a universal 60-second idle kill would lose work. No immediate RSS reduction is promised without measurements.

### 1.2. Data Infrastructure: Shared Core vs Isolated Domain Storage (Chung vs Lẽ)
- **Shared Infrastructure (Hạ tầng chung - SQLite):**
  - Stored strictly in the single-writer SQLite database:
    - Canonical Task, Session, and Run lifecycles.
    - Outbox transactional effects ledger.
    - Package inventory and activation audit log.
    - **Universal Resource Registry (`custos_universal_resource_registry`):** Pointers to domain resources (`quant://...`, `research://...`).
  - Keep large artifact payloads outside SQLite; bounded binary fields are a schema decision, not a blanket ban. Resource references do not replace domain records, ACLs or provenance.
- **Isolated Domain Storage (Hạ tầng lẽ - Out-of-band File Storage):**
  - High-volume domain artifacts are saved to dedicated local directories outside SQLite:
    - Quant: Candlestick/tick datasets in Parquet format (`~/.custos/data/quant/datasets/`).
    - Research: Downloaded PDF papers and LanceDB/vector embeddings (`~/.custos/data/research/papers/`).
    - Coding: Git checkouts, branch diffs, and worktrees (`~/.custos/worktrees/`).
  - Uninstall removes verified managed package code only when dependencies and live leases permit. User datasets, PDFs, notebooks, checkpoints and canonical history remain. Purge is a separate scoped operation with a preview.

### 1.3. Agent Token Context: Dynamic Tool Disclosure (`compile_governed_tools`)
- **The Problem:** Exposing 100+ domain tools across all workbenches into the LLM system prompt wastes 3,000–5,000 tokens per interaction, creates latency, and leads to cross-domain tool hallucination.
- **Custos Solution:** **Governed Admission Control**.
  - Baseline: Compile the smallest tool set needed by the Task; even file reads require source/privacy checks.
  - Dynamic Disclosure: Filter by installed/enabled capabilities, worker support, Task scope, privacy and authority. The active lens is a presentation/default signal, not a permission boundary.
  - Assistant can coordinate enabled Coding/Research/Quant capabilities without requiring a lens switch; tool disclosure itself does not grant execution permission.
  - Measure token overhead and accepted-task quality against the same baseline; no percentage saving is established yet.

### 1.4. Local Discovery Auto-Probe (Máy có sẵn tự nhận diện)
- **The Problem:** If packages are pre-packaged, built from source, or copied offline to `~/.custos/packages`, a fresh database would not show them unless the user re-downloads them.
- **Custos Solution:** **Startup Reconciliation Auto-Probe (`scan_local_packages`)**.
  - On daemon startup, `custos-adapters::packages::scan_local_packages` scans `~/.custos/packages/<component_id>/<version>/`.
  - A discovered bundle/manifest/worker is a candidate. Validate the complete manifest, file inventory, publisher signature/trust, paths and compatibility before recording it as installed. A computed digest alone is not authentication.
  - Verified offline imports may avoid downloads; discovery does not automatically enable or execute a package. UI availability follows canonical installation/profile/readiness projections.

### 1.5. Frontend Bundle Footprint: Lazy Code-Splitting
- **The Problem:** Packaging heavy visualization engines (Plotly, TradingView lightweight-charts, Monaco code editor, PDF.js) into the core desktop bundle results in a 20+ MB initial download and sluggish startup.
- **Custos Solution:** **Contribution-Driven Micro-Frontend Registry (`WorkbenchContribution`)**.
  - All heavy pane visualizers (`AlphaStudioPane`, `LiteraturePane`, `TearsheetPane`, etc.) are wrapped with `React.lazy()` dynamic imports.
  - Missing packages show an installation affordance without loading their viewers. React.lazy defers execution/loading but bundled Tauri chunks may still ship in the installer. Independently downloadable UI requires the separate M2 trust/compatibility gates in the modular plan.

---

## 2. Package Directory Layout & Manifest

A valid installable workbench package adheres to the following directory structure:

```
~/.custos/packages/<component_id>/<version>/
├── manifest.json       # Metadata, permissions, min_custos_version
├── package.bundle      # Packaged assets or worker executable
└── ui/                 # (Optional) Static UI contribution assets
```

### `manifest.json` Schema
```json
{
  "id": "quant",
  "version": "0.1.0",
  "name": "Quantitative Finance & Strategy Studio",
  "min_custos_version": "0.1.0",
  "lens": "quant",
  "tools": ["quant_backtest_run", "quant_portfolio_optimize"],
  "resources": ["quant://datasets", "quant://models"],
  "worker_binary": "custos-worker"
}
```

---

## 3. Package Lifecycle RPC Endpoints

The local daemon exposes standard JSON-RPC endpoints for package management:

| Method | Description |
|---|---|
| `packages.inventory.list` | Returns all known components, versions, states, and paths. |
| `packages.install` | Initiates SHA-256 verified download to staging and atomic promotion. |
| `packages.cancel_install` | Cancels an active in-flight installation and purges staging artifacts. |
| `packages.enable` | Enables an installed package for the active profile. |
| `packages.disable` | Disables an active package from the profile. |

---

## 4. Verification & Quality Gates

Every component addition or modification must satisfy the inviolable quality gates:
1. **Zero-Mock Policy:** All IPC calls must connect to real daemon handlers and SQLite tables.
2. **Crate Invariants:** Adheres strictly to the 11 canonical Rust crates rule.
3. **Rust Quality Gate:** `cargo check --workspace --all-targets --offline` and `cargo test --workspace --lib --offline`.
4. **UI Quality Gate:** `npm --prefix ui/desktop run build` (zero TypeScript errors) and `node tools/canvas-smoke.mjs`.
