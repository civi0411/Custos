# Custos Local API Surface & RPC Contract Baseline

> **Classification:** Normative IPC Protocol & Dispatcher Reference  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

This document serves as the canonical inventory of all JSON-RPC methods exposed by `LocalApiDispatcher` (`apps/daemon/src/api/` and `apps/daemon/src/application/`). Total methods cataloged: **212 active endpoints**, organized strictly across the **10 Master Thematic Pillars**.

All methods adhere to the JSON-RPC 2.0 wire format over local IPC domain sockets or HTTP (`apps/daemon/src/http_server/`), with typed parameter validation, outbox transactional semantics, and pre-log credential redaction.

---

## Thematic API Distribution

| Theme | Functional Domain | Primary Handler Module | Method Count |
|---|---|---|---|
| **Theme 2** | Task Kernel & State Machine | `apps/daemon/src/application/tasks.rs` | 11 |
| **Theme 3** | Authority & Capability Gateway | `apps/daemon/src/application/system.rs`, `providers/` | 18 |
| **Theme 4** | Governed Loop & Environment | `apps/daemon/src/application/environment.rs`, `execution.rs` | 18 |
| **Theme 5** | Evidence & Completion Gates | `apps/daemon/src/application/shared_capabilities/` | 8 |
| **Theme 6** | Measured OI & Model Routing | `apps/daemon/src/application/oi.rs`, `providers/` | 27 |
| **Theme 7** | Sovereign Pipeline & Storage | `apps/daemon/src/application/pipeline.rs` | 4 |
| **Theme 8** | Heterogeneous Compute & GPU | `apps/daemon/src/application/compute.rs` | 7 |
| **Theme 9A** | Assistant Workbench & Outbox | `apps/daemon/src/application/assistant/` | 8 |
| **Theme 9B** | Coding Workbench & Workspaces | `apps/daemon/src/application/workspaces/` | 28 |
| **Theme 9C** | Research Workbench & Notebooks| `apps/daemon/src/application/research/`, `shared_capabilities/` | 28 |
| **Theme 9D** | Quantitative Trading Workbench | `apps/daemon/src/application/quant/` | 15 |
| **Theme 9E** | Domain Packs & Package Mgmt | `apps/daemon/src/application/packs/` | 9 |
| **Theme 10** | Connectivity, UI & Integrations | `apps/daemon/src/application/sessions.rs`, `integrations/` | 31 |
| **Total** | **All 10 Thematic Areas** | `apps/daemon/src/application/` | **212** |

---

## 1. Theme 2: Task Kernel & State Machine (`tasks.rs`, `workflow/`)

Enforces INV-01 and INV-02: pure state transitions, durable journal commits, and outbox batch transactions.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.tasks.create` | `apps/daemon/src/application/tasks.rs` | Stages new task with contract, criteria, and budget limits |
| `v1.tasks.get` | `apps/daemon/src/application/tasks.rs` | Retrieves immutable task state and current status |
| `v1.tasks.list` | `apps/daemon/src/application/tasks.rs` | Queries tasks with status, domain, and pagination filters |
| `v1.tasks.advance` | `apps/daemon/src/application/tasks.rs` | Drives task through discrete steps via TrustedKernel |
| `v1.tasks.cancel` | `apps/daemon/src/application/tasks.rs` | Aborts running task, terminates child process groups |
| `v1.tasks.complete` | `apps/daemon/src/application/tasks.rs` | Requests completion gate evaluation against evidence |
| `v1.tasks.spans` | `apps/daemon/src/application/tasks.rs` | Returns OpenTelemetry step spans and timing traces |
| `v1.workflow.start_run` | `apps/daemon/src/application/tasks.rs` | Dispatches DAG execution run under TaskRuntime |
| `v1.workflow.cancel_run`| `apps/daemon/src/application/tasks.rs` | Cancels active execution run and rolls back outbox |
| `v1.workflow.live_run` | `apps/daemon/src/application/tasks.rs` | Subscribes to live streaming run status and steps |
| `v1.workflow.session_runs` | `apps/daemon/src/application/tasks.rs` | Lists all execution runs associated with a session |

---

## 2. Theme 3: Authority Engine & Capability Gateway (`system.rs`, `providers/`)

Enforces INV-03, INV-05, and INV-10: single-use capability tokens, zero ambient authority, and cryptographic credential vaults.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.capabilities.list` | `apps/daemon/src/application/capabilities.rs` | Catalogs active security capabilities and sandbox permits |
| `v1.capabilities.get` | `apps/daemon/src/application/capabilities.rs` | Inspects specific capability permissions and constraints |
| `v1.resources.capabilities` | `apps/daemon/src/application/system.rs` | Queries available host resources (GPU, RAM, VRAM) |
| `v1.settings.get` | `apps/daemon/src/application/system.rs` | Retrieves daemon security policies and configuration |
| `v1.settings.set` | `apps/daemon/src/application/system.rs` | Updates dynamic runtime settings with permit authorization |
| `v1.keys.list` | `apps/daemon/src/application/providers/crud.rs` | Lists registered API provider keys (redacted digests) |
| `v1.keys.generate` | `apps/daemon/src/application/providers/crud.rs` | Enrolls new provider secret into secure credential vault |
| `v1.keys.revoke` | `apps/daemon/src/application/providers/crud.rs` | Revokes API provider key and wipes memory instances |
| `v1.oauth.authorize` | `apps/daemon/src/application/providers/oauth.rs` | Initiates OAuth 2.1 PKCE authorization flow |
| `v1.oauth.exchange` | `apps/daemon/src/application/providers/oauth.rs` | Exchanges authorization code for encrypted tokens |
| `v1.oauth.status` | `apps/daemon/src/application/providers/oauth.rs` | Checks connection health and expiration status |
| `v1.oauth.refresh` | `apps/daemon/src/application/providers/oauth.rs` | Manually triggers background token refresh |
| `v1.oauth.get` | `apps/daemon/src/application/providers/oauth.rs` | Inspects connected identity and granted scopes |
| `v1.oauth.delete` | `apps/daemon/src/application/providers/oauth.rs` | Disconnects external account and revokes tokens |
| `v1.oauth.cancel` | `apps/daemon/src/application/providers/oauth.rs` | Aborts an in-flight OAuth authorization challenge |

---

## 3. Theme 4: Governed Agent Loop & Environment (`environment.rs`, `execution.rs`)

Enforces INV-04 and INV-12: process group containment (`setpgid(0)`), `ProcessDropGuard` zombie elimination, and PTY terminal security.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.environment.list` | `apps/daemon/src/application/environment.rs` | Lists all Sovereign Agent Execution Environments |
| `v1.environment.get` | `apps/daemon/src/application/environment.rs` | Inspects specific AEE container status and leases |
| `v1.environment.provision`| `apps/daemon/src/application/environment.rs` | Provisions isolated worktree or container sandbox |
| `v1.environment.verify` | `apps/daemon/src/application/environment.rs` | Asserts environment integrity and boundary isolation |
| `v1.environment.destroy`| `apps/daemon/src/application/environment.rs` | Teardown sandbox, terminates processes with SIGKILL |
| `v1.harness.list` | `apps/daemon/src/application/execution.rs` | Discovers registered agent harnesses (`governed`, `claude`) |
| `v1.harness.get` | `apps/daemon/src/application/execution.rs` | Inspects harness driver metadata and active sessions |
| `v1.harness.run_native` | `apps/daemon/src/application/execution.rs` | Spawns external CLI harness with ProcessDropGuard |
| `v1.harness.steer` | `apps/daemon/src/application/execution.rs` | Sends steering input or user intervention to runner |
| `v1.harness.cancel` | `apps/daemon/src/application/execution.rs` | Signals immediate cancellation to running harness |
| `v1.terminal.spawn` | `apps/daemon/src/application/execution.rs` | Spawns an isolated PTY pseudo-terminal session |
| `v1.terminal.get` | `apps/daemon/src/application/execution.rs` | Inspects terminal session dimensions and status |
| `v1.terminal.list` | `apps/daemon/src/application/execution.rs` | Enumerates active terminal sessions |
| `v1.terminal.read` | `apps/daemon/src/application/execution.rs` | Reads buffered output from terminal PTY stream |
| `v1.terminal.write` | `apps/daemon/src/application/execution.rs` | Writes authorized stdin bytes to terminal PTY |
| `v1.terminal.resize` | `apps/daemon/src/application/execution.rs` | Updates terminal row/col window dimensions |
| `v1.terminal.terminate`| `apps/daemon/src/application/execution.rs` | Closes terminal session and reaps child process group |
| `v1.system.caffeinate` | `apps/daemon/src/application/system.rs` | Prevents host OS sleep during long compute runs |

---

## 4. Theme 5: Evidence Engine & Completion Gate (`shared_capabilities/`)

Enforces INV-06 and INV-07: verifier-isolated receipts, CAS deduplication, and anti-tampering protection.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.artifacts.list` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Queries CAS artifacts by task, hash, or tags |
| `v1.artifacts.get` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Fetches immutable content and cryptographic digest |
| `v1.artifacts.record_lineage`| `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Records parent-child provenance in SQLite |
| `v1.artifacts.lineage_graph`| `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Computes directed acyclic provenance graph |
| `v1.reviews.record` | `apps/daemon/src/application/shared_capabilities/reviewer.rs` | Submits structured peer or automated code review |
| `v1.reviews.get` | `apps/daemon/src/application/shared_capabilities/reviewer.rs` | Fetches verification review details and diff ranges |
| `v1.reviews.list` | `apps/daemon/src/application/shared_capabilities/reviewer.rs` | Lists reviews associated with candidate changes |
| `v1.reviews.mark_stale` | `apps/daemon/src/application/shared_capabilities/reviewer.rs` | Invalidates cached reviews upon underlying code change |

---

## 5. Theme 6: Measured OI & Model Routing (`oi.rs`, `providers/`)

Implements Pareto Multi-Objective Optimization ($U = w_q \cdot Q - w_c \cdot C - w_t \cdot T$), Prompt Cache Affinity, and FastPathFilter.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.oi.explain` | `apps/daemon/src/application/oi.rs` | Explains routing decisions, model choices, and topologies |
| `v1.models.catalog` | `apps/daemon/src/application/providers/models.rs` | Lists dynamic model capabilities, context windows, tiers |
| `v1.models.pricing` | `apps/daemon/src/application/providers/models.rs` | Returns real-time pricing and prompt cache multipliers |
| `v1.models.probe` | `apps/daemon/src/application/providers/models.rs` | Measures live round-trip latency to inference providers |
| `v1.providers.list` | `apps/daemon/src/application/providers/crud.rs` | Enumerates configured cloud and local model providers |
| `v1.providers.get` | `apps/daemon/src/application/providers/crud.rs` | Inspects provider status and endpoint configuration |
| `v1.providers.save` | `apps/daemon/src/application/providers/crud.rs` | Registers or updates provider credentials and priority |
| `v1.providers.delete` | `apps/daemon/src/application/providers/crud.rs` | Unregisters provider and clears cached client pools |
| `v1.providers.route` | `apps/daemon/src/application/providers/crud.rs` | Evaluates optimal model route for a given task intent |
| `v1.providers.routing.get`| `apps/daemon/src/application/providers/crud.rs` | Retrieves active Pareto routing weight configuration |
| `v1.providers.routing.save`| `apps/daemon/src/application/providers/crud.rs` | Updates Pareto weights ($w_q, w_c, w_t$) |
| `v1.providers.usage` | `apps/daemon/src/application/providers/crud.rs` | Aggregates token consumption and financial costs |
| `v1.llm.status` | `apps/daemon/src/application/providers/crud.rs` | Health check for LLM inference gateway and fallbacks |
| `v1.local_models.list` | `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Catalogs discovered GGUF/Ollama/vLLM models |
| `v1.local_models.load` | `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Loads local weights into VRAM with admission permit |
| `v1.local_models.unload` | `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Unloads model to free GPU memory for training |
| `v1.local_models.verify` | `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Verifies local model checksum and tensor integrity |
| `v1.local_models.hardware`| `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Detects Metal / CUDA / ROCm accelerator topology |
| `v1.local_models.preflight`| `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Evaluates VRAM feasibility before model load |
| `v1.local_models.instances`| `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Lists active running inference worker instances |
| `v1.local_models.scan` | `apps/daemon/src/application/providers/local_models/lifecycle.rs` | Scans local filesystem directories for model weights |
| `v1.local_models.folders.list`| `apps/daemon/src/application/providers/local_models/folders.rs` | Lists registered local model discovery directories |
| `v1.local_models.folders.add` | `apps/daemon/src/application/providers/local_models/folders.rs` | Enrolls new folder path for automatic model scanning |
| `v1.local_models.folders.remove`| `apps/daemon/src/application/providers/local_models/folders.rs`| Removes folder path from automatic model scanning |
| `v1.local_models.ensure`| `apps/daemon/src/application/providers/local_models/lifecycle.rs`| Ensures required local model is downloaded and ready |
| `v1.local_models.profile.get`| `apps/daemon/src/application/providers/local_models/lifecycle.rs`| Gets local model execution profile (quantization, context) |
| `v1.local_models.profile.save`| `apps/daemon/src/application/providers/local_models/lifecycle.rs`| Updates local model execution profile |

---

## 6. Theme 7: Sovereign Pipeline & Storage (`pipeline.rs`)

Enforces relational consistency, Medallion storage progression (Bronze $\rightarrow$ Silver $\rightarrow$ Gold), and immutable provenance.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.pipeline.ingest` | `apps/daemon/src/application/pipeline.rs` | Ingests raw untrusted payloads into Bronze storage |
| `v1.pipeline.transform`| `apps/daemon/src/application/pipeline.rs` | Executes verified transformation into Silver schema |
| `v1.pipeline.aggregate`| `apps/daemon/src/application/pipeline.rs` | Produces Gold-tier analytical aggregations and proofs |
| `v1.pipeline.status` | `apps/daemon/src/application/pipeline.rs` | Queries data pipeline execution status and lineage |

---

## 7. Theme 8: Heterogeneous Compute & GPU Admission (`compute.rs`)

Enforces co-existence of GPU training and inference workloads, remote SSH/Cloud failover, and checkpoint continuity.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.compute.targets.list` | `apps/daemon/src/application/compute.rs` | Catalogs compute backends (Local Metal/CUDA, Colab, Cloud) |
| `v1.experiment.preflight` | `apps/daemon/src/application/compute.rs` | Checks target readiness, Python environment, GPU memory |
| `v1.experiment.submit` | `apps/daemon/src/application/compute.rs` | Dispatches training/eval workload with VRAM admission |
| `v1.experiment.observe` | `apps/daemon/src/application/compute.rs` | Streams GPU loss curves, telemetry metrics, and logs |
| `v1.experiment.stop` | `apps/daemon/src/application/compute.rs` | Safely checkpoints and interrupts remote compute job |
| `v1.experiment.collect` | `apps/daemon/src/application/compute.rs` | Synchronizes checkpoint weights and artifacts to local CAS |
| `v1.experiment.release` | `apps/daemon/src/application/compute.rs` | Frees compute lease and returns GPU memory to pool |

---

## 8. Theme 9: Vertical Specialist Workbenches

### 8A. Assistant Workbench (`assistant/`)
| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.assistant.drafts.list` | `apps/daemon/src/application/assistant/drafts.rs` | Lists pending communication and action drafts |
| `v1.assistant.drafts.get` | `apps/daemon/src/application/assistant/drafts.rs` | Fetches draft contents, recipients, and context |
| `v1.assistant.drafts.save` | `apps/daemon/src/application/assistant/drafts.rs` | Saves or edits an assistant message draft |
| `v1.assistant.drafts.delete`| `apps/daemon/src/application/assistant/drafts.rs`| Discards an unapproved message draft |
| `v1.assistant.outbox.list` | `apps/daemon/src/application/assistant/outbox.rs` | Lists staged external side effects awaiting approval |
| `v1.assistant.outbox.create`| `apps/daemon/src/application/assistant/outbox.rs`| Stages an external side effect with execution permit |
| `v1.assistant.outbox.approve`| `apps/daemon/src/application/assistant/outbox.rs`| Human explicit approval granting dispatch permit |
| `v1.assistant.outbox.dispatch`| `apps/daemon/src/application/assistant/outbox.rs`| Atomically dispatches approved outbox message |

### 8B. Coding Workbench & Workspaces (`workspaces/`)
| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.workspaces.list` | `apps/daemon/src/application/workspaces/lifecycle.rs` | Lists active project workspaces and worktrees |
| `v1.workspaces.get` | `apps/daemon/src/application/workspaces/lifecycle.rs` | Retrieves workspace metadata, root path, and Git HEAD |
| `v1.workspaces.create` | `apps/daemon/src/application/workspaces/lifecycle.rs` | Creates a new managed workspace or clones repository |
| `v1.workspaces.archive` | `apps/daemon/src/application/workspaces/lifecycle.rs` | Archives workspace and tears down child worktrees |
| `v1.workspaces.detect` | `apps/daemon/src/application/workspaces/inspection/detect.rs` | Automatically detects toolchains, tests, package managers |
| `v1.workspaces.recover` | `apps/daemon/src/application/workspaces/lifecycle.rs` | Reconciles orphaned worktrees after daemon restart |
| `v1.workspaces.pick_folder`| `apps/daemon/src/application/workspaces/lifecycle.rs` | Invokes native OS file dialog to select repository folder |
| `v1.workspaces.inspect_dirty`| `apps/daemon/src/application/workspaces/inspection/changes.rs` | Audits uncommitted changes and unstaged diffs |
| `v1.workspaces.candidates.spawn`| `apps/daemon/src/application/workspaces/changes/spawn.rs` | Spawns isolated Git worktree for candidate patch |
| `v1.workspaces.candidates.compare`| `apps/daemon/src/application/workspaces/changes/compare.rs` | Generates 3-way diff between base, head, and candidate |
| `v1.workspaces.integrate` | `apps/daemon/src/application/workspaces/changes/integrate.rs` | Merges verified candidate patch into main branch |
| `v1.workspaces.agent_steer`| `apps/daemon/src/application/workspaces/lifecycle.rs` | Injects guidance or constraints into coding agent |
| `v1.workspace.files.tree` | `apps/daemon/src/application/workspaces/inspection/files.rs` | Returns directory tree respecting `.gitignore` |
| `v1.workspace.files.read` | `apps/daemon/src/application/workspaces/inspection/files.rs` | Reads file content with path normalization security |
| `v1.workspace.files.write`| `apps/daemon/src/application/workspaces/inspection/files.rs` | Writes file inside authorized worktree boundary |
| `v1.workspace.diff` | `apps/daemon/src/application/workspaces/inspection/changes.rs` | Computes Git diff across the entire worktree |
| `v1.workspace.diff.file` | `apps/daemon/src/application/workspaces/inspection/changes.rs` | Computes diff for a single file |
| `v1.workspace.git.repository`| `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Inspects Git repository state, branches, remotes |
| `v1.workspace.git.fetch` | `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Fetches upstream Git commits |
| `v1.workspace.git.switch` | `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Switches active branch or checkout |
| `v1.workspace.git.stage` | `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Stages modified files for commit |
| `v1.workspace.git.unstage`| `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Unstages files from index |
| `v1.workspace.git.commit` | `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Commits staged changes with signed message |
| `v1.workspace.git.discard`| `apps/daemon/src/application/workspaces/inspection/repository_git.rs`| Reverts uncommitted changes in worktree |
| `v1.workspace.git.account`| `apps/daemon/src/application/workspaces/github.rs` | Retrieves active Git committer identity |
| `v1.workspace.git.github` | `apps/daemon/src/application/workspaces/github.rs` | Queries GitHub repository issues and pull requests |
| `v1.github.auth_status` | `apps/daemon/src/application/workspaces/github.rs` | Checks GitHub OAuth token status |
| `v1.github.login` | `apps/daemon/src/application/workspaces/github.rs` | Initiates GitHub OAuth authentication flow |

### 8C. Research Workbench & Notebooks (`research/`, `shared_capabilities/`)
| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.research.sources.list` | `apps/daemon/src/application/research/sources.rs` | Lists cited academic literature and dataset sources |
| `v1.research.sources.save` | `apps/daemon/src/application/research/sources.rs` | Records new bibliographic reference or DOI citation |
| `v1.research.claims.list` | `apps/daemon/src/application/research/mod.rs` | Catalogs scientific claims extracted from literature |
| `v1.research.claims.save` | `apps/daemon/src/application/research/mod.rs` | Records claim with supporting evidence anchors |
| `v1.research.anchors.list` | `apps/daemon/src/application/research/mod.rs` | Lists textual and empirical citation anchors |
| `v1.research.anchors.save` | `apps/daemon/src/application/research/mod.rs` | Saves pinpointed evidence snippet with page/line |
| `v1.research.recipes.list` | `apps/daemon/src/application/research/reviews/recipes.rs` | Lists reproducible experiment recipes |
| `v1.research.recipes.get` | `apps/daemon/src/application/research/reviews/recipes.rs` | Fetches experiment recipe steps and environment |
| `v1.research.recipes.save` | `apps/daemon/src/application/research/reviews/recipes.rs` | Saves executable experiment recipe |
| `v1.research.runs.list` | `apps/daemon/src/application/research/runs.rs` | Lists historical research experiment runs |
| `v1.research.runs.save` | `apps/daemon/src/application/research/runs.rs` | Records completed run with metric logs and CAS hashes |
| `v1.research.executions.list`| `apps/daemon/src/application/research/runs.rs` | Lists live research executions |
| `v1.research.executions.get` | `apps/daemon/src/application/research/runs.rs` | Inspects execution status, memory, and telemetry |
| `v1.research.executions.resume.assess`| `apps/daemon/src/application/research/runs.rs`| Evaluates whether interrupted run can resume safely |
| `v1.research.lineage.list` | `apps/daemon/src/application/research/mod.rs` | Returns full data lineage for research findings |
| `v1.research.handoff_coding`| `apps/daemon/src/application/research/reviews/handoff.rs`| Hands off verified algorithm to Coding Workbench |
| `v1.synthesis.proposals.list`| `apps/daemon/src/application/research/reviews/proposals.rs`| Lists research synthesis proposals |
| `v1.synthesis.proposals.get` | `apps/daemon/src/application/research/reviews/proposals.rs`| Fetches synthesis proposal details |
| `v1.synthesis.proposals.save`| `apps/daemon/src/application/research/reviews/proposals.rs`| Saves generated research synthesis proposal |
| `v1.synthesis.handoff.execute`| `apps/daemon/src/application/research/reviews/handoff.rs`| Dispatches synthesized research into execution DAG |
| `v1.notebook.cells.list` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Lists cells in interactive notebook |
| `v1.notebook.cells.save` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Saves cell code, markdown, or metadata |
| `v1.notebook.execute` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Executes cell in isolated Jupyter/Python kernel |
| `v1.notebook.interrupt` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Sends SIGINT to interrupt running kernel cell |
| `v1.notebook.reset` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Resets notebook kernel state and clears variables |
| `v1.notebook.status` | `apps/daemon/src/application/shared_capabilities/notebooks.rs` | Queries Jupyter kernel status (Idle, Busy, Dead) |
| `v1.notes.list` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Lists research lab notebook notes |
| `v1.notes.get` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Fetches note markdown content and attachments |
| `v1.notes.save` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Persists note with revision history |
| `v1.notes.history` | `apps/daemon/src/application/shared_capabilities/artifacts.rs` | Retrieves complete revision audit log of note |

### 8D. Quantitative Trading Workbench (`quant/`)
| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.quant.backtest.submit` | `apps/daemon/src/application/quant/backtest.rs` | Submits quantitative trading strategy backtest |
| `v1.quant.tearsheet.get` | `apps/daemon/src/application/quant/backtest.rs` | Retrieves Sharpe, Sortino, Drawdown tearsheet |
| `v1.quant.pbo.compute` | `apps/daemon/src/application/quant/backtest.rs` | Computes Probability of Backtest Overfitting (PBO) |
| `v1.quant.grid.evaluate` | `apps/daemon/src/application/quant/backtest.rs` | Evaluates parameter grid search across historical data |
| `v1.quant.indicators.compute`| `apps/daemon/src/application/quant/backtest.rs` | Computes technical/statistical indicators on bars |
| `v1.quant.orders.create` | `apps/daemon/src/application/quant/orders.rs` | Creates staged order ticket requiring human approval |
| `v1.quant.orders.list` | `apps/daemon/src/application/quant/orders.rs` | Lists pending and filled order tickets |
| `v1.quant.orders.approve` | `apps/daemon/src/application/quant/orders.rs` | Authorizes staged order ticket with execution permit |
| `v1.quant.kill_switch` | `apps/daemon/src/application/quant/orders.rs` | Emergency stop: cancels all pending orders, closes positions |
| `v1.quant.orderbook.get` | `apps/daemon/src/application/quant/orders.rs` | Queries live L2 orderbook market depth |
| `v1.quant.factors.discover` | `apps/daemon/src/application/quant/market.rs` | Discovers statistical alpha factors from market data |
| `v1.quant.market.bars.ingest`| `apps/daemon/src/application/quant/market.rs` | Ingests OHLCV bar data into partitioned storage |
| `v1.quant.market.bars.get` | `apps/daemon/src/application/quant/market.rs` | Queries historical candlestick market bars |
| `v1.quant.gate.evaluate` | `apps/daemon/src/application/quant/market.rs` | Pre-trade risk gate evaluation (VaR, exposure limits) |
| `v1.quant.review.compute` | `apps/daemon/src/application/quant/market.rs` | Post-trade execution quality and slippage review |

### 8E. Domain Packs & Package Management (`packs/`)
| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.packs.manifests.list` | `apps/daemon/src/application/packs/workbench_catalog.rs` | Lists installed domain pack descriptors |
| `v1.packs.roles.list` | `apps/daemon/src/application/packs/workbench_catalog.rs` | Enumerates specialized agent roles in each pack |
| `v1.packs.jobs.list` | `apps/daemon/src/application/packs/package_management.rs` | Lists scheduled automation jobs in domain packs |
| `v1.packs.jobs.get` | `apps/daemon/src/application/packs/package_management.rs` | Inspects domain pack automation job details |
| `v1.packs.jobs.run` | `apps/daemon/src/application/packs/package_management.rs` | Triggers execution of pack-defined automation job |
| `v1.packages.inventory` | `apps/daemon/src/application/packs/package_management.rs` | Lists discovered and installed package components |
| `v1.packages.install` | `apps/daemon/src/application/packs/package_management.rs` | Installs downloadable vertical workbench or component |
| `v1.packages.enable` | `apps/daemon/src/application/packs/package_management.rs` | Enables an installed workbench component |
| `v1.packages.disable` | `apps/daemon/src/application/packs/package_management.rs` | Disables an active workbench component |

---

## 9. Theme 10: Connectivity Hubs, Desktop UI & Integrations

Manages interactive session journals, browser automation, fleet orchestration, and system diagnostics.

| Method | Physical Handler Location | Architectural Function |
|---|---|---|
| `v1.sessions.create` | `apps/daemon/src/application/sessions.rs` | Creates an interactive user session |
| `v1.sessions.get` | `apps/daemon/src/application/sessions.rs` | Retrieves session metadata and active mode |
| `v1.sessions.list` | `apps/daemon/src/application/sessions.rs` | Lists user sessions with timestamp ordering |
| `v1.sessions.delete` | `apps/daemon/src/application/sessions.rs` | Deletes session and archives interaction journal |
| `v1.sessions.message` | `apps/daemon/src/application/sessions.rs` | Appends user message to session journal |
| `v1.sessions.journal` | `apps/daemon/src/application/sessions.rs` | Reads full event-sourced conversation journal |
| `v1.sessions.attach` | `apps/daemon/src/application/sessions.rs` | Attaches UI client to live session stream |
| `v1.sessions.steer` | `apps/daemon/src/application/sessions.rs` | Injects mid-stream guidance into agent session |
| `v1.sessions.promote` | `apps/daemon/src/application/sessions.rs` | Promotes ephemeral chat into governed persistent task |
| `v1.sessions.metrics` | `apps/daemon/src/application/sessions.rs` | Queries token consumption and latency metrics |
| `v1.sessions.open_in_lens`| `apps/daemon/src/application/sessions.rs` | Opens session in target vertical workbench lens |
| `v1.bridge.attach` | `apps/daemon/src/application/sessions.rs` | Bridges external harness (Claude Code, Aider) into session |
| `v1.bridge.steer` | `apps/daemon/src/application/sessions.rs` | Relays user steering inputs to bridged external harness |
| `v1.browser.sessions.create`| `apps/daemon/src/application/integrations/browser/sessions.rs`| Spawns headless Chromium browser session |
| `v1.browser.sessions.list` | `apps/daemon/src/application/integrations/browser/sessions.rs`| Lists active browser sessions |
| `v1.browser.tabs.create` | `apps/daemon/src/application/integrations/browser/tabs.rs` | Opens new tab in browser session |
| `v1.browser.tabs.list` | `apps/daemon/src/application/integrations/browser/tabs.rs` | Enumerates open browser tabs |
| `v1.browser.tabs.navigate` | `apps/daemon/src/application/integrations/browser/navigation.rs` | Navigates tab to URL with SSRF whitelist check |
| `v1.browser.tabs.click` | `apps/daemon/src/application/integrations/browser/interaction.rs` | Clicks DOM element by selector or coordinate |
| `v1.browser.tabs.fill` | `apps/daemon/src/application/integrations/browser/interaction.rs` | Inputs text into verified form input field |
| `v1.browser.tabs.snapshot` | `apps/daemon/src/application/integrations/browser/inspection.rs` | Captures DOM accessibility tree snapshot |
| `v1.browser.tabs.screenshot`| `apps/daemon/src/application/integrations/browser/inspection.rs` | Captures visual PNG screenshot of viewport |
| `v1.browser.tabs.close` | `apps/daemon/src/application/integrations/browser/tabs.rs` | Closes browser tab |
| `v1.emulator.devices.list` | `apps/daemon/src/application/integrations/emulator.rs` | Lists connected mobile emulator instances |
| `v1.emulator.devices.launch`| `apps/daemon/src/application/integrations/emulator.rs` | Boots mobile emulator instance |
| `v1.emulator.devices.tap` | `apps/daemon/src/application/integrations/emulator.rs` | Sends tap event to mobile UI coordinate |
| `v1.emulator.devices.navigate`| `apps/daemon/src/application/integrations/emulator.rs`| Navigates mobile app view |
| `v1.emulator.devices.snapshot`| `apps/daemon/src/application/integrations/emulator.rs`| Captures mobile UI hierarchy XML |
| `v1.emulator.devices.screenshot`| `apps/daemon/src/application/integrations/emulator.rs`| Captures mobile screen image |
| `v1.fleet.hosts.list` | `apps/daemon/src/application/integrations/fleet.rs` | Lists enrolled remote SSH host nodes |
| `v1.fleet.hosts.register` | `apps/daemon/src/application/integrations/fleet.rs` | Enrolls remote host with SSH key credential |
| `v1.fleet.hosts.ping` | `apps/daemon/src/application/integrations/fleet.rs` | Asserts SSH connectivity and host status |
| `v1.fleet.exec` | `apps/daemon/src/application/integrations/fleet.rs` | Executes command on remote host node |
| `v1.automation.jobs.list` | `apps/daemon/src/application/integrations/automation.rs` | Lists headless cron automation jobs |
| `v1.automation.jobs.create`| `apps/daemon/src/application/integrations/automation.rs` | Schedules new recurring automation job |
| `v1.automation.jobs.run` | `apps/daemon/src/application/integrations/automation.rs` | Triggers immediate execution of automation job |
| `v1.ping` | `apps/daemon/src/application/system.rs` | Fast round-trip daemon liveness probe (< 1ms) |
| `v1.health` | `apps/daemon/src/application/system.rs` | Comprehensive system health check and readiness |
| `v1.system.metrics` | `apps/daemon/src/application/system.rs` | Gathers system resource utilization and memory |
| `v1.system.telemetry` | `apps/daemon/src/application/system.rs` | Dumps OpenTelemetry span traces and events |
