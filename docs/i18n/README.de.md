<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **Supervised Agent Development Environment (SADE) — Local-First, Evidenzbasiert, Menschlich Gesteuert.**

Custos ist eine Local-First-Entwicklungs- und Ausführungsumgebung für autonome Agenten in den Bereichen Programmierung (Coding), wissenschaftliche Forschung (Research) und persönliche Assistenz. Im Gegensatz zu einfachen LLM-Wrappern oder intransparenten Multi-Agenten-Systemen erzwingt Custos strikte menschliche Aufsicht, deterministische Zustandspersistenz, kryptografisch auditierte Fähigkeitsschranken und empirische Nachweise, bevor eine Aufgabe als abgeschlossen anerkannt wird.

---

## Drei Nicht Verhandelbare Säulen

| Säule | Grundprinzip | Architektonische Garantie |
|---|---|---|
| **P1 — Aufgabe als First-Class-Bürger** | Aufgaben existieren unabhängig von Sitzungen und Verbindungen | Persistente SQLite-WAL-Speicherung, deterministische Absturzwiederherstellung und idempotentes Befehlsbuch. |
| **P2 — Vorschlag ≠ Entscheidung ≠ Effekt** | Das Modell schlägt vor; der Kernel genehmigt; das Gateway führt aus | Das KI-Modell gibt lediglich einen `ActionIntent` aus; der Kernel signiert ein Einmal-`ExecutionPermit`; die Sandbox erzwingt Fähigkeitsschranken. |
| **P3 — Evidenzbasierter Abschluss** | Kein Abschluss ohne empirischen Nachweis | Aufgaben bestehen nur, wenn sie durch objektive Artefakte (Diffs, Testausführungs-Traces, Zitationsaudits) belegt sind; null Scheinerfolge. |

---

## 10 Unverrückbare Systeminvarianten

1. **INV-01 (Local-First-Souveränität):** Alle Zustände, Verträge, Aufgaben und Telemetriedaten verbleiben auf dem lokalen Datenträger.
2. **INV-02 (Keine Schein-Atomarität):** Zustandsänderungen werden in SQLite committet, bevor externe Aktionen versendet werden.
3. **INV-03 (Genehmigung vor Effekt):** Keine externe Mutation wird ohne ein kryptografisch gebundenes `ExecutionPermit` ausgeführt.
4. **INV-04 (Akteursidentitätsbindung):** Die Identität des Akteurs wird aus authentifizierten Transportschichten gebunden, niemals aus Payload-Angaben.
5. **INV-05 (Evidenz vor Behauptung):** Die Aufgabenabnahme erfordert verifizierte empirische Artefakte, nicht bloße Zusicherungen des Modells.
6. **INV-06 (Explizite UI-Wahrheit):** Die Benutzeroberfläche wechselt sofort in einen Fail-Closed-Zustand (`unavailable`/`degraded`), wenn Backend-Dienste fehlen.
7. **INV-07 (Worktree-Lease-Isolation):** Parallele Schreibzugriffe auf denselben Arbeitsbaum werden gefenced, um Datenverlust zu verhindern.
8. **INV-08 (Ressourcen-Obergrenzen):** Jede Prozessausführung, Zeilenpufferung und Token-Bereinigung respektiert strenge Byte-Limits.
9. **INV-09 (Auditierte Transportschichten):** Jede Mutation durchläuft das idempotente Befehlsbuch mit Digest-Überprüfung.
10. **INV-10 (Menschliche Vorherrschaft):** Der menschliche Operator behält das uneingeschränkte Recht, Aufgaben in jeder Phase zu pausieren, anzupassen oder abzubrechen.

---

## Funktionsmechanismus

```mermaid
flowchart LR
    Human[Menschlicher Operator] --> Session[Sitzungskanal / Workbench]
    Session --> Task[Persistente Aufgabe]
    Task --> Context[Kontextaufbereitung]
    Context --> Worker[Modell- / Harness-Worker]
    Worker --> Intent[Handlungsabsicht (Action Intent)]
    Intent --> Gate[Autorisierungsschranke & Schein]
    Gate --> Effect[Sandbox-Effekt]
    Effect --> Evidence[Objektive Evidenz]
    Evidence --> Outcome[Verifizierter Abschluss]
```

---

## Systemarchitektur: Die 11 Kanonischen Rust-Crates

Custos trennt reine Domänenlogik und Seiteneffekte strikt in 11 geschichteten Rust-Crates:

| Schicht | Crate | Kernverantwortung |
|---|---|---|
| **Schicht 0 (Domain)** | `custos-domain` | Reine Domänenentitäten, Zustandsautomaten und Zero-I/O-Systeminvarianten. |
| **Schicht 1 (Core)** | `custos-core` | Aufgabenorchestrierung, Autorisierungs-Engine und Abschlussverifizierer. |
| | `custos-persistence` | SQLite-WAL-Persistenz, Outbox-Muster, CAS-Zustandsübergänge und Absturzwiederherstellung. |
| | `custos-provider` | Anbieterneutrale Modellverträge, Streaming-SSE-Parser und Tokenizer. |
| **Schicht 2 (Runtime & Bridge)** | `custos-bridge` | Heraufstufung von Sitzungen zu Aufgaben, RPC-Routing und lokaler API-Dispatch. |
| | `custos-runtime` | Kognitiver Agentenzyklus, Kontextkomprimierung (RTK), Speicherstufen und Planer. |
| | `custos-adapters` | Betriebssystem-Sandboxen, MCP-Client-Adapter, lokale Inferenz-Sidecars und Protokollübersetzer. |
| **Schicht 3 (Domain Packs)** | `custos-packs` | Spezialisierte Workflows: Software-Engineering, wissenschaftliche Forschung, Assistenz. |
| **Schicht 4 (Entry & Distribution)** | `custos-daemon` | Einzige Composition Root, Hintergrund-Daemon-Dienst und lokale HTTP/JSON-RPC-Schnittstelle. |
| | `custos-sdk` | Client-DTOs, UniFFI-Bindungen und Transport-Clients. |
| | `custos-cli` | Terminal-TUI-Bedienoberfläche (`ratatui`). |

---

## Desktop-Benutzeroberfläche

Die Desktop-Oberfläche von Custos basiert auf **Vite, React 19 und Tauri 2**:

- **3-Ebenen-Hierarchie:** Nahtlose Navigation über `Projekt -> Arbeitsbereich -> Arbeitsbaum (Project -> Workspace -> Worktree)`.
- **Graphite JetBlack Token-System:** Modernes, kontrastreiches Dark-Design für ermüdungsfreie Arbeit.
- **Dedizierte Workbenches:**
  - **Engineering:** Dateien & Änderungen, Evidenz-Prüfer und Worktree-Manager.
  - **Forschung:** Synthesestudio, wissenschaftliche Entdeckungen und Durchlauf-Journal.
  - **Assistenz & Geteilt:** Integrierter Browser, Terminal, Remote-Flotte und Notizen.
- **Null-Mock-Richtlinie (Zero-Mock Policy):** Alle UI-Metriken spiegeln reale Daemon-Telemetrie wider und warnen explizit bei Ausfällen.

---

## Qualitätsschranken und Tests

Alle Codebeiträge müssen zwingend folgende Schranken passieren:

```bash
# 1. Blockierendes Clippy-Gate ohne Warnungen (Rust)
cargo clippy --workspace --all-targets -- -D warnings

# 2. Umfassende Testsuite (>1.670 Tests, 100% bestanden)
cargo test --workspace --all-targets

# 3. Desktop-UI Typüberprüfung und Bundle-Build
cd ui/desktop && npm run build
```

---

## Dokumentation und Referenzen

- **Architekturspezifikation:** [Custos.md](../../Custos.md)
- **Agenten- und Beitragsregeln:** [AGENTS.md](../../AGENTS.md)
- **Desktop-UI-Standards:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git-PR-Workflow:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **Dokumentationsverzeichnis:** [docs/README.md](../README.md)

---

## Lizenz

Custos wird unter der **GNU AGPL-3.0-or-later** vertrieben. Sie enthält eine strikte Markenklausel zum Schutz der Marke Custos und untersagt proprietäre kommerzielle SaaS-Verpackungen. Einzelheiten siehe [LICENSE](../../LICENSE).
