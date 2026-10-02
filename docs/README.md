# Custos Documentation

> **Notice:** The definitive architecture and specification of this repository have been fully consolidated into a Single Source of Truth (SSOT).

## 1. Canonical Architecture
All architectural boundaries, design decisions, invariants, and component specifications are documented exclusively in **[Custos.md](../Custos.md)**. 
- Please read `Custos.md` first before reading any historical files in this directory.

## 2. Agent Behavior & Team Policy
All AI agents, human collaborators, and bots must strictly adhere to the rules outlined in **[AGENTS.md](../AGENTS.md)**.

## 3. Internationalization (i18n)
Localized documentation and repository overviews are available in **[i18n/README.md](i18n/README.md)** (English, Tiếng Việt, Deutsch, 简体中文).

## 4. Definitive Documentation Structure
The documentation is strictly organized into 5 core subdirectories to maintain cleanliness and prevent overlap with `Custos.md`:

- **`i18n/`**: Localized README translations (English, Tiếng Việt, Deutsch, 简体中文).
- **`assets/`**: Images, diagrams, and visual resources.
- **`architecture/`**: System foundations, trust zones, and 8 deep-dive architectural pillars supporting Custos.md.
- **`reference/`**: Dictionaries, API schemas, error codes, and naming conventions for quick lookup.
- **`development/`**: Developer guides, codebase topology, testing standards, supply chain policy, and delivery blueprints.

## Authority Order
When documents disagree, use this strict order:
1. Current source code and manifests.
2. `AGENTS.md` (for repository policy and boundaries).
3. `Custos.md` (for definitive architecture and target state).
4. The specific deep-dive documents in `architecture/`, `reference/`, or `development/`.
