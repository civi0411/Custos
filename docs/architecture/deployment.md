# Local-First Deployment Architecture

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§28, §34) & Part VII (§47) Canonical Specification

Custos is designed entirely around the **Local-First** paradigm: guaranteeing that user data, code repositories, and execution artifacts remain exclusively on the local workstation, with zero dependencies on centralized cloud services.

---

## 1. Local Process Architecture

The system consists of a single authoritative background daemon and thin client interfaces communicating over Unix Domain Sockets:

```text
+-------------------------------------------------------------+
|                      LOCAL WORKSTATION                      |
|                                                             |
|   [ custos CLI ]               [ VS Code Extension ]        |
|          |                              |                   |
|          +--------------+---------------+                   |
|                         | JSON-RPC (Unix Socket)            |
|                         v                                   |
|            +-------------------------+                      |
|            |  custosd (Rust Daemon)  |                      |
|            +------------+------------+                      |
|                         |                                   |
|       +-----------------+-----------------+                 |
|       v                 v                 v                 |
|  [ SQLite DB ]    [ Git Worktrees ]  [ Tiered Sandbox ]     |
+-------------------------------------------------------------+
```

---

## 2. Tiered Sandboxing Backends

Custos applies native operating system isolation mechanisms depending on the host OS:

### Tier 1: macOS Native Sandbox (`sandbox-exec`)
On macOS, all shell operations and tool invocations execute under strict **Seatbelt** profile configurations:
- Read/write access is restricted exclusively to the designated Git worktree directory.
- Sensitive user directories (`~/.ssh`, `~/.aws`, `~/Library/Keychains`) are blocked from read/write.
- All outbound network access is blocked except for domains explicitly authorized in the Task Contract.

### Tier 2: Linux Native Sandbox (`bubblewrap` / Namespaces)
On Linux, Custos utilizes `bwrap` (the underlying sandboxing engine behind Flatpak):
- Establishes isolated mount, network, and PID namespaces.
- Constructs a minimal virtualized root filesystem exposing only the compiler/interpreter toolchains required for building and testing.

### Tier 3: OCI / Docker Container (Fallback & Integration)
Reserved for tasks requiring isolated external services (e.g., spinning up a local PostgreSQL or Redis instance for integration test suites).

---

## 3. Resource Governance & Local Footprint

The `custosd` daemon enforces strict local resource bounds:
- **Resident Memory:** `< 60MB` idle, `< 250MB` under intensive multi-worker orchestration.
- **CPU Scheduling:** Automatically adjusts process scheduling priority (`nice` level) to prevent workstation UI degradation during long-running builds.
