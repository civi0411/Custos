# Protocol & Connectivity Hub Architecture

> **Classification:** Core Architectural Pillar  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md) (Part 7).  
> **Architecture Hub:** See [Custos Architecture Overview](README.md).

Custos interacts with client user interfaces (CLI, VS Code, Web UI), external tool providers (MCP Servers), remote agents (A2A networks), and AI reasoning backends through a unified **Protocol & Connectivity Hub Layer** hosted within `custos-daemon`.

---

## 1. The Six Specialized Daemon Hubs

The `custos-daemon` acts as a central composition root housing six dedicated hubs:

```mermaid
graph TD
    Client["Client UI (CLI / IDE / Web)"] -->|Local API (Unix Domain Socket)| SessionHub["1. Session Hub"]
    SessionHub --> Kernel["Trusted Task Kernel"]
    
    Kernel --> ModelHub["2. Model Hub"]
    Kernel --> CapHub["3. Capability Hub"]
    Kernel --> McpHub["4. MCP Hub"]
    Kernel --> A2aHub["5. A2A Hub"]
    
    ModelHub --> ExtLLM["Local & Cloud LLMs"]
    CapHub --> LocalTools["Host Tools & Sandboxes"]
    McpHub --> ExtMCP["External MCP Servers"]
    A2aHub --> ExtAgents["Remote Agent Networks"]

    Kernel -.-> EventBus["6. Event Bus Hub"]
    EventBus -.-> SessionHub
```

| Hub Name | Core Architectural Responsibility | Protocol Boundary |
|---|---|---|
| **Session Hub** | Manages client connections, heartbeat liveness, and session lifecycle. | Local API v1 (Domain Sockets, Named Pipes) |
| **Capability Hub** | Dispatches vetted commands to local OS sandboxes and Git worktrees. | Internal Rust Capability Port |
| **MCP Hub** | Discovers, connects to, and governs external Model Context Protocol servers. | MCP Protocol (STDIO, HTTP, SSE) |
| **A2A Hub** | Manages cryptographic agent-to-agent delegation networks. | A2A Protocol (HTTPS REST, SSE) |
| **Model Hub** | Orchestrates provider-neutral model dispatch, token streaming, and fallback. | HTTPS REST / Streaming |
| **Event Bus Hub** | Broadcasts immutable domain events asynchronously to clients. | Tokio broadcast channels |

---

## 2. Inbound & Outbound Protocol Map

| Direction | Protocol Standard | Transport Layer | Authentication Mechanism | Managing Crate |
|---|---|---|---|---|
| **Inbound** (Client $\rightarrow$ Daemon) | Local API v1 | Unix Domain Socket / Named Pipe | OS IPC Peer Credential (UID Check) | `custos-bridge` |
| **Inbound** (Editor $\rightarrow$ Daemon) | ACP (Agent Comm Protocol) | JSON-RPC 2.0 over IPC | Ephemeral session token | `custos-bridge` |
| **Outbound** (Daemon $\rightarrow$ Tools) | MCP Specification | STDIO (Local) / HTTP & SSE | OAuth 2.1 AS Metadata + Scoped Perms | `custos-adapters` |
| **Outbound** (Daemon $\rightarrow$ Agents)| A2A Protocol | HTTPS REST + SSE Streaming | Agent Cards + IBCT Token Chain | `custos-adapters` |
| **Outbound** (Daemon $\rightarrow$ Models)| Provider Native / OpenAI Wire| HTTPS REST / Streaming | API Keys in OS Secure Keychain | `custos-provider` |

---

## 3. Model Context Protocol (MCP) Integration

Custos implements the Model Context Protocol strictly adhering to the specification:

### 3.1 Transport Security
- **STDIO Transport:** Launches external MCP servers as child processes restricted by working directory sandboxes.
- **HTTP Transport:** Requires **OAuth 2.1** with Authorization Server Metadata (RFC 8414) and Dynamic Client Registration (RFC 7591).
- **SSE Transport:** Ingests unidirectional streaming tool updates verified with short-lived Bearer tokens.

### 3.2 Callback Interception
- **Sampling Callback (Model Calling Reverse LLM):** If an MCP server requests an LLM sampling call, it **must route through Custos's Model Hub**. Direct model querying by MCP servers is blocked to enforce budget limits and taint tracking.
- **Elicitation Callback:** Prompts the user via the Session Hub for clarifying information, parking the task in `Blocked`.
- **Roots Callback:** Only exposes paths explicitly granted in `TaskScope`. The host machine's root filesystem is never exposed.

---

## 4. Agent-to-Agent (A2A) Delegation Protocol

Custos discovers and coordinates with independent external agent systems using the A2A standard:

```json
// Agent Card Representation (/.well-known/agent-card.json)
{
  "agent_id": "urn:custos:agent:rust-specialist-node",
  "display_name": "Custos Rust Specialist Worker",
  "version": "2.0.0",
  "capabilities": [
    { "name": "rust_code_repair", "assurance": "custos-mediated" },
    { "name": "cargo_clippy_audit", "assurance": "custos-mediated" }
  ],
  "endpoint": "https://node.custos.local/a2a/v1",
  "security": {
    "auth_type": "OAuth2.1-PoP",
    "supported_tokens": ["urn:ietf:params:oauth:token-type:ibct"]
  }
}
```

### Delegation Invariant:
Any artifact or message ingested from a remote agent via A2A is unconditionally stamped with `Taint::Untrusted`. It cannot modify local configuration or bypass the Completion Gate without verified evidence.

---

## 5. Five Harness Adapters

Custos integrates established development tools through five dedicated harness adapters:
1. **Claude Code Adapter:** Intercepts tool calls and converts them into governed `ActionIntent` records.
2. **OpenAI Codex Adapter:** Translates tool-use payloads into typed capability requests.
3. **Cursor Adapter:** Connects to editor workspace contexts and projects diffs into isolated worktrees.
4. **Antigravity Adapter:** Interfaces with DeepMind-style autonomous agent execution harnesses.
5. **Goose Adapter:** Integrates headless command-line execution and environment automation.
