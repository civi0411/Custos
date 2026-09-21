# Open Source Adoption Strategy

> **Status:** Canonical Baseline v4.0  
> **Source:** Part XIII (§35-36) & Part VII (§50-53, §57-58) Canonical Specification

Custos adopts a pragmatic open source philosophy: **Own the semantic core, avoid indiscriminate copy-pasting, and integrate high-quality libraries exclusively across strict architectural boundaries.**

---

## 1. Five Adoption Modes

| Mode | Definition | Enforcement Policy | Example Projects |
|---|---|---|---|
| **Direct Dependency** | Added directly into `Cargo.toml` | Standardized libraries with rigorous license auditing (MIT/Apache-2.0). | `tokio`, `serde`, `rusqlite`, `tree-sitter`, `cedar-policy` |
| **Clean Integration** | Integrated via dedicated Adapter | Interfaces strictly through abstract traits; swappable without modifying core. | `modelcontextprotocol/rust-sdk`, `opentelemetry` |
| **Reference / Borrow** | Architectural study, re-implemented to Custos specs | Learn structural patterns; reproduce them with Custos-native unit tests. | State machine patterns from Temporal; outbox pattern from Restate |
| **Shadow / Evaluation**| Run in parallel evaluation mode | Zero impact on main execution paths; used purely for metric comparisons. | TypeSafe Jev adapter in advisory mode |
| **Reject / No-Adopt** | Explicitly rejected | Avoid bloated dependencies, architectural misalignment, or copyleft licenses (GPL). | Bloated chat agent frameworks (LangChain, CrewAI) |

---

## 2. Curated Open Source Repository Map

Custos selectively adopts world-class libraries to accelerate development:

```text
┌─────────────────────────────────────────────────────────────┐
│                    RECOMMENDED REPO MAP                     │
├─────────────────────────┬───────────────────────────────────┤
│ Foundation & Parsing    │ tree-sitter/tree-sitter           │
│                         │ ast-grep/ast-grep                 │
│                         │ BurntSushi/ripgrep                │
├─────────────────────────┼───────────────────────────────────┤
│ Security & Policy       │ cedar-policy/cedar                │
│                         │ containers/bubblewrap             │
├─────────────────────────┼───────────────────────────────────┤
│ Protocols & Standards   │ modelcontextprotocol/rust-sdk     │
│                         │ open-telemetry/opentelemetry-rust │
├─────────────────────────┼───────────────────────────────────┤
│ Provider SDKs           │ openai/codex                      │
│                         │ anthropics/claude-agent-sdk-*     │
└─────────────────────────┴───────────────────────────────────┘
```

---

## 3. Supply Chain & Licensing Policy

- **License Compliance:** Automated verification via `cargo-deny`. Only **MIT, Apache-2.0, BSD-2-Clause, and BSD-3-Clause** licenses are permitted. GPL/AGPL licensed code is strictly prohibited in all binary releases.
- **Pinned Dependencies:** The `Cargo.lock` file is committed to Git; all dependency version bumps must be submitted via separate PRs with full regression test suites.
