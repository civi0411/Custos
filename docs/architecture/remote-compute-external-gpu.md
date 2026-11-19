# Dedicated and On-Demand GPU Compute Fleet Architecture

> **Status:** Authoritative Architectural Specification  
> **Topic:** External dedicated hosts, on-demand cloud Pods, lifecycle management, and financial guardrails  
> **Governing Boundaries:** `custos-contracts`, `custos-storage`, `custos-integrations`, `custos-engine`, and `workbenches/research`.

---

## 1. Scope & Core Distinctions

External compute resources (on-premise workstations, university lab machines, and provider-managed cloud Pods such as RunPod or Vast.ai) represent raw computational capacity. They are neither LLM model ports nor reasoning agents.

Architectural boundaries cleanly decouple:
1. **Provider Control Plane:** Quota discovery, allocation APIs, instance provisioning, and teardown.
2. **Host Connectivity:** Secure SSH tunneling, authenticated loopback bridges, and structured command execution.
3. **Execution Engine:** Supervised process trees, Jupyter notebook kernels, and checkpoint streaming.
4. **Durable Storage:** Content-addressable manifests, object stores, and local session databases.

### Compute Profile Matrix

| Profile | Default Authority | Lifecycle Semantics |
|---|---|---|
| **Dedicated SSH Host** | Probe/read upon connection consent; file mutation requires explicit scope. | Disconnecting does not shut down the physical host; leases release workspace locks only. |
| **Adopted Cloud Instance** | Account/resource ID + explicit attachment scope. | Autonomous termination is prohibited if unmanaged workloads share the instance. |
| **Custos On-Demand Pod** | Allocation requires preview of quote, token/monetary budget, and container image digest. | Full lifecycle tracking: quote -> allocating -> active -> terminating -> released. |
| **Spot / Interruptible** | Includes spot eviction risk and automated checkpoint frequency policies. | System assumes abrupt revocation; relies on externalized checkpoint manifests. |

---

## 2. Four Unambiguous Identities

To prevent state confusion, the system tracks four orthogonal identity types:
1. **Connection / Account Identity:** Authentication credentials, secret references, and provider API versions.
2. **Compute Lease Identity:** Instance or Pod UUID, hardware region, allocated quota, volume attachments, and expiry deadlines.
3. **Host Runtime Identity:** SSH host public key fingerprint, process supervisor PID, and kernel execution epoch.
4. **Experiment Attempt Identity:** Task revision digest, recipe hash, input dataset SHA-256, and artifact manifest.

Public IP addresses and integer PIDs are never treated as durable identities.

---

## 3. Placement Across Canonical Platform Blocks

| Layer / Crate | Responsibilities |
|---|---|
| **`custos-contracts`** | Compute backend enums, hardware observation structs, authority scopes (`compute.allocate`, `compute.execute`, `compute.release`), and accounting types. |
| **`custos-storage`** | Single-writer persistence of remote fleet nodes, lease records, checkpoint receipts, and billing audit logs. |
| **`custos-integrations`** | Concrete SSH adapters, provider API clients (RunPod, Vast.ai), NVML structured telemetry probes, and file sync bridges. |
| **`custos-engine`** | Remote experiment coordination, checkpoint save/restore handoffs, and admission watchdog services. |
| **`workbenches/research`** | Recipe execution models, training loop monitoring, and evaluation gates. |

Target integration modules reside in `platform/integrations/src/compute/` (`ssh_host.rs`, `runpod.rs`, `vastai.rs`).

---

## 4. Dedicated SSH Host: Secure Protocol Flow

1. **Host Verification:** User provides connection parameters and verifies host key fingerprints against known hosts; `StrictHostKeyChecking=no` is strictly disallowed.
2. **Structured Capability Probing:** Host capabilities (OS, GPU model, VRAM capacity, CUDA driver vs. PyTorch runtime) are queried via structured probes (NVML or `nvidia-smi --query-gpu ... --format=csv,noheader,nounits`).
3. **Explicit Workspace Scope:** Operations are bounded to a designated directory; `~` is never granted blanket write authorization.
4. **Content-Hashed Input Staging:** Files and datasets are validated against SHA-256 manifests prior to remote execution; shell string interpolation of unescaped model output is forbidden.
5. **Supervisor-Managed Jobs:** Batch jobs execute under a local process supervisor with distinct job IDs, ensuring execution continues across SSH disconnects.
6. **Isolated Notebook Bridges:** Jupyter kernel bindings operate via authenticated loopback tunnels over SSH, never exposing unauthenticated public ports.
7. **Clean Teardown:** Host disconnection releases leased processes; user data outside the managed workspace is strictly preserved.

---

## 5. Storage Classifications & Lifecycle Invariants

Provider storage documentation (e.g., RunPod container vs. volume vs. network storage) enforces strict classification:
- **Container Ephemeral Disk:** Discarded immediately upon Pod termination; never used for critical checkpoints.
- **Pod-Attached Volume:** Persists across Pod restarts, but is destroyed if the Pod is permanently terminated.
- **Independent Network Volumes:** Persists across Pod terminations, but continues to incur hourly storage fees until explicitly deleted.

### Command Semantics

| Command | Expected System Effect | Prohibited Behavior |
|---|---|---|
| **Disconnect** | Closes SSH/control channels; leaves remote jobs running. | Marking remote jobs as stopped in UI. |
| **Stop Job** | Sends SIGTERM/SIGINT to target supervisor process. | Terminating the underlying shared host/Pod. |
| **Stop Instance** | Triggers provider-supported ACPI shutdown; retains storage. | Assuming every cloud instance supports non-billing stop. |
| **Terminate Pod** | Confirms deletion of the compute instance with provider. | Silently deleting independent persistent volumes. |
| **Delete Storage** | Destructive purge of retained volumes with confirmation. | Bundling volume deletion into tab-close actions. |

---

## 6. Financial Guardrails & Local Watchdogs

1. **Comprehensive Spend Model:** Total cost comprises compute hours + idle time + persistent storage + data ingress/egress + checkpoint snapshot transfers.
2. **Pre-Flight Quotes:** Allocations capture timestamped price quotes from provider APIs with currency and granularity annotations.
3. **Active Watchdog:** The local engine monitors elapsed wall-clock time and estimated spend against user-defined token/dollar ceilings, raising warnings at 80% and halting new step admission at 100%.
4. **Deadline-Driven Checkpoint Commits:** When approaching hard spend or duration limits, the system triggers a final model checkpoint and state commit before executing instance termination.
