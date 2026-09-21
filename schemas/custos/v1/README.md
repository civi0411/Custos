# Custos Canonical Schemas (v1)

This directory contains the canonical JSON Schemas for Custos v4.0.
All inter-service and boundary communication must validate against these contracts.

| Schema | File | Description |
|---|---|---|
| Task | `task.v1.schema.json` | Core task state machine and lifecycle |
| Continuation Packet | `continuation.v1.schema.json` | Resumable state carrier across spans & models |
| Action | `action.v1.schema.json` | Side-effect declaration subject to policy gate |
| Capability | `capability.v1.schema.json` | Tool & runtime capability manifest |
| Context Pack | `context_pack.v1.schema.json` | Compiled context slice with evidence anchors |
| Claim | `claim.v1.schema.json` | Verifiable claim object |
| Evidence | `evidence.v1.schema.json` | Cryptographic/deterministic proof of correctness |
| Provider Capability | `provider_capability.v1.schema.json` | AI model provider features and limits |
| Workflow | `workflow.v1.schema.json` | Declarative multi-step task definition |
