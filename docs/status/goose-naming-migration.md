# Goose-derived naming migration

**Document ID:** STATUS-NAME-01. **Status:** measured migration register. **Snapshot:** dirty checkout indexed by Custos Nexus on 2026-09-27; no commit SHA pinned.

Nexus re-indexed 2,267 Custos files, 10,207 Rust/Python symbols and 70,354 syntactic call references on 2026-09-28 after the documentation restructuring. The naming guard reported `goose` compatibility/debt in 574 files outside excluded generated/vendor/history paths. Counts describe the dirty snapshot and must be regenerated after migration; they do not establish runtime reachability.

## Disposition

| Class | Examples | Decision |
|---|---|---|
| Upstream/vendor identity | `docs/goose`, upstream URLs, `@aaif/goose-*`, upstream binary packages | Preserve exact identity and provenance. Move/isolate later only with license and build-reference checks. |
| Protocol compatibility | `_goose/unstable/*`, Goose ACP notifications, serialized `goose_provider` | Preserve wire spelling in a named compatibility module. Add canonical Custos DTOs and explicit translators before removal. |
| User-data compatibility | `GOOSE_*`, `.goose`, `goose+roam://`, Goose deep links | Add Custos-first dual-read migration. Do not mechanically rename because existing installations would lose configuration or links. |
| Product-owned identity | `goose-app`, `goose-ask-ai-bot`, UI text and release defaults claiming the product is Goose | Rename to Custos now; this snapshot changed package/product/release defaults to Custos while retaining legacy protocol/config fallbacks. |
| Internal implementation | `get_goose_*`, `GooseMode`, `gooseServe`, `goose.hook.*` | Migrate behind compatibility aliases in bounded PRs; canonical call sites and telemetry become Custos-named. |
| Tests and fixtures | temporary paths, captured build text and compatibility tests | Rename generic tests; retain exact Goose fixtures when their purpose is interoperability and label them accordingly. |
| Generated build output | `target/**/goose_*` | Ignore as source debt; stale artifacts disappear on a clean rebuild. |

## Hotspots from Nexus and source scan

- `crates/runtime/custos-engine`: about 125 source files contain Goose naming. It is an out-of-workspace fork-derived package and must not be bulk-enabled or bulk-renamed.
- Nexus caller tracing found `get_goose_provider` on at least eleven production/test call paths spanning extension context, configuration, agent creation, gateway pairing, model resolution, events and scheduled jobs. Migrate it through a canonical accessor plus legacy-key fallback, not a local symbol substitution.
- `get_goose_context_limit` has a smaller observed production surface (`get_context_limit` and `get_local_context_limit`, plus tests), making it a suitable first N1 compatibility slice.
- `crates/core/custos-sdk-types`: owns hundreds of `_goose/unstable/*` request/notification occurrences. Treat this as a protocol compatibility surface, not the canonical Custos Local API.
- `ui/goose-acp`, `ui/goose-acp-client`, `ui/goose-binary`: currently retain upstream package/binary identity and provenance. Decide whether to vendor, replace or publish a Custos adapter before directory renames.
- `ui/desktop`: a Custos product surface with many Goose component/service names. Rename by feature slice after a UI build and ACP fixture baseline exists.
- `custos-local-inference`, `custos-providers`, `custos-roaming` and engine configuration: contain legacy keys, paths, user agents and schemes requiring dual-read migration.

## Target path decisions

| Current path/name | Target disposition |
|---|---|
| `docs/goose/` | Move to `docs/vendor/goose/` only after inbound-link and provenance checks; content remains upstream-named. |
| `ui/desktop/` | Keep the responsibility-based path; migrate internal components/services from Goose to Custos during N4. |
| `ui/goose-acp-client/` | Choose either `ui/acp-client/` plus a Custos package name, or isolate unchanged under a vendor tree. Do not leave a hybrid fork. |
| `ui/goose-acp/`, `ui/goose-binary/` | Keep temporarily as exact upstream distribution identities; target `vendor/goose/npm/` if retained rather than independently developed. |
| `scripts/goose_scripts/` | Move supported import/compatibility utilities to `scripts/compat/goose/`; archive one-off extraction scripts with their source revision. |
| `goose_doc_guide` builtin | Replace product documentation behavior with `custos-doc-guide`; retain a clearly named upstream Goose guide only for interoperability help. |
| Kotlin `io.github.aaif_goose` namespace | Treat as published ABI. Introduce a Custos namespace and bridge/release plan before removal. |
| `_goose/unstable/*` | Keep only in a Goose compatibility module; canonical Custos API uses a versioned Custos namespace. |

## Migration sequence

| Gate | Scope | Exit evidence |
|---|---|---|
| N0 | Freeze taxonomy and regenerate Nexus/source inventory | This register, naming policy, indexed database and no new product package named Goose. |
| N1 | Configuration compatibility | `CUSTOS_*` precedence tests, Goose fallback tests, redacted deprecation event and migration guide. |
| N2 | Canonical protocol boundary | Custos versioned requests/notifications plus round-trip translators for supported Goose messages. |
| N3 | Runtime internals | Canonical `custos_*` functions/types/telemetry; temporary deprecated aliases; engine tests. |
| N4 | Desktop and service surfaces | Custos package/component/service names, UI build, ACP reconnection and packaging tests. |
| N5 | Distribution decision | Upstream Goose binaries remain isolated vendor dependencies or are replaced by Custos binaries; license/SBOM/release evidence. |
| N6 | Removal | Breaking-version notice, old-config import fixture, zero unclassified Goose occurrence outside vendor/compatibility/history. |

The immediate rule is: preserve interoperability spellings, eliminate accidental product identity, and never infer ownership from a copied name. Follow [DEV-NAME-01](../development/naming-conventions.md) for every rename.
