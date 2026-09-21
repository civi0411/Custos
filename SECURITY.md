# Security Policy & Vulnerability Disclosure

Security and human sovereignty are foundational pillars of **Custos**. Because Custos orchestrates code generation, local command execution, and AI model interactions, we treat security vulnerabilities with the highest priority.

---

## 🛡️ Supported Versions

Only the current active development baseline receives security patches:

| Version | Supported |
|---|---|
| 4.0-draft (main) | :white_check_mark: |
| < 4.0 | :x: |

---

## 🚨 Reporting a Vulnerability

If you discover a security vulnerability in Custos, **please do not disclose it publicly** via GitHub issues, discussions, or social media.

Instead, please send a detailed report to our security team:
- **Email:** `cuuvi985@gmail.com`
- **Subject:** `[SECURITY VULNERABILITY] Custos - <brief description>`

### What to Include
Please provide:
1. A clear description of the vulnerability and its potential impact.
2. Steps to reproduce the issue (proof-of-concept code, scripts, or environment details).
3. The affected component, crate, or version.
4. Any proposed remediations or patches if you have one.

### Response Timeline
- **Initial Acknowledgment:** Within 48 hours of receipt.
- **Assessment & Status Update:** Within 5 business days.
- **Fix & Public Disclosure:** Coordinated disclosure following patch verification.

---

## 🔒 Core Security Invariants

Custos implements defense-in-depth through architectural constraints:
1. **Zero Direct Execution:** Workers cannot execute commands or alter files directly; everything passes through the `Capability Gateway`.
2. **Exact-Payload Approval:** High-risk actions require explicit human sign-off bound cryptographically to the exact payload hash.
3. **Worktree Isolation:** Code mutations occur inside isolated Git worktrees, never on user working branches directly.
4. **Zero Secrets in Persistence:** API keys and credentials are saved in the OS Keychain and zeroized in memory upon drop.
5. **Zero Egress by Default:** No telemetry or raw user code leaves the local workstation unless explicitly configured.

For full architectural details, see [Threat Model](docs/security/threat-model.md) and [Capability Model](docs/security/capability-model.md).
