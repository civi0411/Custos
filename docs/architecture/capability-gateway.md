# Capability Gateway & Sandboxing

> **Document ID:** ARCH-GATEWAY-01  
> **Status:** Partially Verified & Active Specification  
> **Normative Framework:** [`reference-architecture.md`](reference-architecture.md) & [`runtime-flows.md`](runtime-flows.md)  
> **Verified Implementation:**  
> - `PathSandbox` directory containment: **Verified** in [`crates/runtime/custos-security`](../../crates/runtime/custos-security) (`tests/e2e/tests/controlled_effects_proof_closure.rs`)  
> - `DeterministicGate` (`read_file`, `list_files`, `patch_preview`): **Verified** in `crates/runtime/custos-security/src/gateway/deterministic.rs`  
> - `ExecutionPermit` & `ActionIntent` contracts: Standardized in [`docs/contracts/README.md`](../contracts/README.md) (Contract C-03) and [`schemas/protocol/`](../../schemas/protocol/)  
> **Target / Future Scope:** External Cedar policy compilation and OS-level Bubblewrap/Seatbelt production daemon wiring.

The Capability Gateway enforces Custos's supreme security invariant: **Zero Direct Execution**. All file system operations, shell executions, and network egress calls must traverse this gateway.

---

## 1. ExecutionPermit Mechanism

Every action with external side effects must hold a valid `ExecutionPermit` prior to execution inside a sandboxed environment.

```rust
pub struct ExecutionPermit {
    pub permit_id: Uuid,
    pub task_id: TaskId,
    pub tool_name: String,
    pub payload_hash: Sha256Hash,  // SHA-256 digest of complete execution payload
    pub granted_authority: AuthorityLevel,
    pub valid_from: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,  // Strict TTL expiry
    pub single_use: bool,          // Enforces single-use execution
    pub signature: Vec<u8>,        // Cryptographic signature from Task Kernel
}
```

---

## 2. Tool Execution Sequence

```mermaid
sequenceDiagram
    autonumber
    participant Worker as Ephemeral Worker
    participant Gateway as Capability Gateway
    participant Policy as Cedar Policy Engine
    participant Human as Human Operator
    participant Sandbox as Tiered Sandbox
    participant Verifier as Verifier & CAS

    Worker->>Gateway: invoke_tool(name, payload)
    Gateway->>Gateway: Calculate SHA-256(payload)
    Gateway->>Policy: Evaluate policy(task_scope, tool, payload)
    
    alt Needs Human Approval (High Risk)
        Policy-->>Gateway: RequireApproval(exact_diff)
        Gateway->>Human: Request Exact-Payload Approval
        Human-->>Gateway: Approved
    else Allowed by Policy
        Policy-->>Gateway: PermitGranted
    end
    
    Gateway->>Gateway: Mint cryptographically signed ExecutionPermit
    Gateway->>Sandbox: execute(tool, payload, permit)
    Sandbox-->>Gateway: ExecutionOutput + ExitCode
    Gateway->>Verifier: Store output artifact in CAS & emit Receipt
    Gateway-->>Worker: ToolResult(output, receipt_id)
```

---

## 3. Cedar Policy Integration

Custos adopts the **Cedar** policy language (AWS / Linux Foundation) for deterministic, sub-millisecond authorization:

```cedar
// Permit source read access within designated active worktree
permit(
    principal == Custos::Worker::"engineering.explorer",
    action == Custos::Action::"ReadFile",
    resource in Custos::Workspace::"active_task_worktree"
);

// Strictly forbid direct mutations to the root .git directory or system config
forbid(
    principal,
    action in [Custos::Action::"WriteFile", Custos::Action::"DeleteFile"],
    resource in Custos::Path::"**/.git/**"
);
```

---

## 4. Model Context Protocol (MCP) Integration

Custos integrates the official **`modelcontextprotocol/rust-sdk`** strictly at the Capability Gateway boundary:
- **MCP at the Outer Boundary:** Used exclusively to bridge external tools and servers (PostgreSQL, GitHub API, Web search).
- **Zero Internal MCP:** Internal inter-module communication uses native Rust typed contracts to maximize throughput and guarantee type safety.
