# Custos Workspace — Agent Rules

> These rules apply to all AI Assistants (Antigravity, Cursor, Claude Code, Copilot, Codex, Windsurf) working within the `Custos` workspace.  
> The authoritative project-level rules are in [`AGENTS.md`](../../AGENTS.md).

---

## R1 — Documentation Language

All files authored or modified in this workspace MUST be written in **100% Technical English**.

- No Vietnamese, mixed-language prose, or informal language in any `docs/`, `dev_docs/`, notes, reports, or commit messages.
- The sole exception: explicit i18n mirror files at `docs/i18n/README.<lang>.md`.

## R2 — Zero Decorative Emojis

Do not use decorative symbols or emojis in documentation, headings, tables, diagrams, or commit logs.

## R3 — Git Safety

AI Agents MUST obtain explicit written confirmation from the human operator before executing any of:
`git add`, `git commit`, `git push`, `git rebase`, `git merge`, `git tag`, `git reset --hard`, `git push --force`.

**`git push --force` is prohibited unless the user types the exact command themselves.**

## R4 — Do Not Cross Domain Boundaries

Respect the 3-role ownership matrix at all times:
- **Vi** owns: AI cognition, prompts, context, judgment, evaluation, domain agents, sidecars.
- **Truong** owns: Task Kernel, SQLite persistence, capability gateway, authority engine, CLI, daemon.
- **Vinh** owns: Multi-agent coordination, worker lifecycle, handoff protocols, concurrency, fault tolerance.

Never edit a file owned by one team member while assisting another.

## R5 — No Hallucinated Implementation

Do not invent new crates, schemas, or protocols. If a domain type or module does not exist and needs to be created, surface the design question to the human before generating code.

## R6 — Conservative Scope

Do not refactor, rename, or restructure code outside the explicitly requested change. Preserve all `todo!()` markers unless the human explicitly requests implementing that specific function.
