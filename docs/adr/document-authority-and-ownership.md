# ADR proposal: document authority and shared-module ownership

**ADR ID:** ADR-DOC-01. **Status:** Lead-directed; team review pending. **Date:** 2026-09-27. **Sponsor:** Custos product lead. **Required reviewers:** Vi, Truong, Vinh. **Implementation:** documentation navigation and the current ownership map have been updated; shared code contracts still require cross-owner review.

## Context

The repository contains historical plans and duplicated work-hub material that mix target design, current status and roadmap. Ownership assignments must follow the current package map, not stale paths or reports.

## Proposed decision

1. Use [`docs/README.md`](../README.md) as the documentation entrypoint and authority hierarchy.
2. Use source plus repeatable tests at a pinned SHA for implemented/wired/verified claims. Use accepted, versioned contracts and ADRs for normative shared boundaries. Use `dev_docs/` for active work, not architectural authority.
3. Use `AGENTS.md` as the current ownership policy and `dev_docs/MODULE_OWNERSHIP.md` as its operational summary. `custos-domain` is assigned to Truong; Vi and Vinh review shared contract changes. The `docs/archive/dev_docs-2026-09-27/` copy is historical only.
4. Shared domain, persistence, security, public DTO and effect contract changes require all three maintainers' recorded review. A documented safety/data objection blocks that contract until resolved or scope is narrowed.
5. Preserve old plans and snapshots with status warnings and provenance. Do not silently promote research or old “canonical” headings to current implementation facts.

## Team review still required

The maintainers should review this governance text and the three-person review rule. The lead-directed assignment is operational now; this review is not a reason to block scoped work that does not change shared contracts.

## Acceptance evidence

Record review outcome, exact governance revision, current Cargo member list, affected source paths, producer/consumer tests, and a migration path for references to older ownership labels. Until team review is recorded, status remains **Lead-directed; team review pending**.
