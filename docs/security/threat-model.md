# Threat Model & Security Defenses

> **Classification:** Legacy V4 design detail, non-normative. Use the [documentation hub](../README.md), [reference architecture](../architecture/reference-architecture.md), and [source-backed status rules](../status/README.md) before treating any statement below as current behavior.

> **Implementation status:** Unknown unless a current status record and source/test evidence say otherwise.
> **Source:** Part V (§25) Canonical Specification  
> **Framework:** STRIDE Threat Model

This document outlines the security threat model for an autonomous local-first AI runtime operating on developer workstations, and the concrete defenses implemented by Custos.

---

## 1. STRIDE Threat Analysis

| STRIDE Category | Concrete Threat to Agentic Runtime | Custos Defense Mechanism |
|---|---|---|
| **Spoofing** | Malicious local process impersonates user instructions | Authenticates local IPC requests via Unix domain socket peer credentials (UID matching); Ed25519 digital signatures on all Events and Permits. |
| **Tampering** | Rogue worker mutates files outside task scope or modifies event history | Git worktree isolation; SQLite WAL with append-only access; cryptographic hash-chained event sequences. |
| **Repudiation** | Model or tool denies generating faulty code | Append-only `Decision Ledger` logs all choices; signed `Receipts` record exit codes, file diff hashes, and timestamps. |
| **Information Disclosure** | Prompt injection exfiltrates sensitive files (`~/.ssh/id_rsa`, `.env`) | Sandboxing blocks read access to user home directories; Privacy Membrane regex scans and blocks secret egress. |
| **Denial of Service** | Infinite reasoning loops exhaust token budget or freeze workstation CPU | Hard budget invariant ceilings; strict per-step timeouts; automatic worker lease cancellation upon expiry. |
| **Elevation of Privilege** | Model uses prompt injection to self-grant root execution capabilities | **Confidence never creates Capability**; only the authenticated human principal can grant scope expansion. |

---

## 2. Prompt Injection Defense

All external inputs (GitHub issue descriptions, unfamiliar source code, PDFs, web search results, shell stdout/stderr) are classified as **Potentially Malicious Untrusted Data**.

1. **Data vs. Instruction Segregation:** Untrusted file contents are encapsulated inside explicit XML-like delimiters (e.g., `<untrusted_content>`), preventing LLM tokenizers from confusing user data with system instructions.
2. **Zero Direct Execution from LLM Outputs:** Raw model text is never piped directly into `eval()` or `sh -c`. It must deserialize into a typed, structured `ActionProposal` evaluated and gated by the Capability Gateway.
