# Capability Model & Approvals

> **Status:** Canonical Baseline v4.0  
> **Source:** Part V (§25.3-25.4) Canonical Specification

Custos security is built on **Capability-Based Security**: a component cannot perform any external action unless it presents a cryptographically signed, unforgeable `ExecutionPermit`.

---

## 1. Exact-Payload Approval Principle

One of the most dangerous vulnerabilities in existing AI agent tools is asking for broad, generic approvals (e.g., *"Do you allow the agent to run terminal commands?"*).

Custos enforces the strict principle of **Exact-Payload Approval**:
- The human principal **approves only a specific action bound to its exact payload hash**:
  ```text
  PermitHash = SHA256(ToolName + NormalizedParameters + TargetFileDiff + Timestamp)
  ```
- If an LLM mutates even a single character or flag in the command, the SHA-256 digest changes, and the issued `ExecutionPermit` is immediately invalidated.
- Custos never supports an option to *"Allow all commands from now on"* for high-risk operations.

---

## 2. Zero Secrets in Persistence & Telemetry

- **No Plain-text Credentials:** API keys for AI providers (OpenAI, Anthropic) and third-party services are never persisted to SQLite tables or flat configuration files.
- **Native OS Keychain Integration:** Custos interacts directly with the operating system's secure credential store:
  - macOS Keychain Services via the Security Framework.
  - Linux Secret Service via FreeDesktop DBus secret store.
- **Memory Scrubbing:** Variables holding secrets are wrapped in self-zeroizing memory wrappers (`zeroize` crate in Rust) ensuring heap and stack buffers are overwritten with zeros immediately upon drop.
