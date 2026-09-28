# Research and extraction register

**Status:** source-analysis workspace. A research conclusion becomes product architecture only through an accepted decision and a measured Custos integration.

| Subject | Useful material | Adoption boundary |
|---|---|---|
| Goose | [`docs/goose/`](../goose/) and [source map](../specifications/SOURCE_MAP.md) | Import a capability through a specific port, pinned source revision, license review, conformance test and rollback owner. A copied crate is not a wired flow. |
| Nexus/repo intelligence | [Nexus audit](../archive/NEXUS_FULL_AUDIT_LATEST.md) and [detailed analysis](../archive/NEXUS_REPOS_DETAILED_ANALYSIS.md) | Treat index counts as snapshots; context citations must revalidate source bytes and dirty buffers. |
| System One/Two | [Cognitive research proposal](../architecture/COGNITIVE_ARCHITECTURE_S1_S2.md) | Tier, role and concrete backend are different concepts. Validate routing with task-class evals. |
| 9Router | [Official architecture](https://github.com/decolua/9router/blob/master/docs/ARCHITECTURE.md) and [smart routing](https://github.com/decolua/9router/blob/master/gitbook/content/en/features/smart-routing.md) | Provider/account routing backend only; test actual model/account provenance, streaming, tool calls and fallback. |
| Agentgateway | [Official routes](https://agentgateway.dev/docs/standalone/latest/documentation/configuration/routes/) and [virtual models](https://agentgateway.dev/docs/standalone/latest/documentation/llm/virtual-models/) | Optional network/identity/MCP/A2A front. Do not treat network authorization or post-response budget accounting as Custos effect authorization. |

The [Intelligence Hub design](../architecture/intelligence-hub.md) describes the proposed composition of 9Router and Agentgateway and the tests required before adoption. Pin upstream releases in spike reports; web documentation may change independently of the deployed binary.

The [connectivity findings](connectivity-gateway-findings.md) compare Goose, the MCP specification, 9Router and Agentgateway using primary documentation and distinguish model APIs from agent runtimes and tool execution.

The [Goose naming migration register](../status/goose-naming-migration.md) records which upstream names are provenance or wire compatibility and which are accidental Custos product identity. Use it before renaming any copied symbol, config key, package or directory.
