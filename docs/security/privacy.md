# Privacy & Data Governance

> **Classification:** Legacy V4 design detail, non-normative. Use the [documentation hub](../README.md), [reference architecture](../architecture/reference-architecture.md), and [source-backed status rules](../status/README.md) before treating any statement below as current behavior.

> **Implementation status:** Unknown unless a current status record and source/test evidence say otherwise.
> **Source:** Part V (§25.5-25.6) Canonical Specification

Custos protects intellectual property and developer privacy in an era where cloud AI vendors continuously harvest enterprise codebases for model training.

---

## 1. Zero Egress by Default

1. **Zero Telemetry to Custos Servers:** Custos is an open-source, local-first runtime. It never transmits usage telemetry, behavioral analytics, or crash dumps to any remote server.
2. **Egress Gate for AI Providers:**
   - Before any `ContextPack` transmits across the network to OpenAI, Anthropic, or external providers, it must clear the **Data Sanitizer** filter.
   - Automatically detects and redacts sensitive patterns: email addresses, phone numbers, private key blocks, and API tokens (`sk-ant-...`, `ghp_...`, AWS Access Keys).

---

## 2. Local Data Retention & Purge Policy

The user retains complete sovereignty over all data stored within `.custos/`:
- **Cryptographic Purge:** Running `custos purge --task <task_id>` permanently removes all database rows, execution logs, and CAS artifacts associated with that task from the workstation.
- **Automated CAS Eviction:** Ephemeral artifacts (stale build logs, rejected patch attempts) are pruned after 30 days to reclaim workstation disk capacity.
