# Workbench Lenses Specification

> **Scope**: `crates/custos-domain/src/session/session.rs`, `crates/custos-packs/`  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 3 / F07)

---

## 1. 4-Workbench Taxonomy

Custos organizes all agent interactions into four specialized lenses:

```
                          WorkbenchLens
                               │
         ┌─────────────┬───────┴───────┬─────────────┐
         ▼             ▼               ▼             ▼
      Coding       Research        Assistant       Quant
   (Engineering)  (Deep Science)   (Copilot)     (Finance)
```

| Lens | Identity / ID | Domain Pack | Primary Roles | Primary Output |
|---|---|---|---|---|
| **Coding** | `engineering` | `EngineeringPack` | repo_scout, code_implementer, test_runner, ui_auditor | Working code, git commits, passing tests |
| **Research** | `research` | `ResearchPack` | web_explorer, claim_extractor, fact_checker, synthesizer | Research synthesis, evidence graph, models |
| **Assistant** | `copilot` | `AssistantPack` | triage_agent, drafter, outbox_sender, scheduler | Task management, drafts, scheduled actions |
| **Quant** | `quant` | `QuantPack` | alpha_scout, backtest_runner, risk_supervisor, exec_broker | Alpha signals, tearsheets, risk audits, order tickets |

---

## 2. Invariants & Match Completeness
1. **Exhaustive Pattern Matching**: Every pattern match on `WorkbenchLens` across all 11 crates must explicitly handle `Quant`.
2. **Session Lens Binding**: A Session is created with an immutable lens (`opened_lenses`). Opening a session in a different lens tracks cross-workbench lens promotion without mutating historical journals.
3. **Packs Mapping Parity**: Each lens corresponds to exactly one default `DomainPack` registered in `DomainPackRegistry` at daemon composition root.
