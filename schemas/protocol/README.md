# Custos Protocol Schemas (v1)

This directory contains the canonical wire protocol JSON Schemas for Custos.
All inter-process, network, and language boundary communications validate against these contracts.

| Schema | File | Description |
|---|---|---|
| Protocol Envelope | `envelope.v1.schema.json` | Universal inter-component message envelope |
| Protocol Error | `error.v1.schema.json` | Standardized cross-boundary error format |
| Provider Request | `provider-request.v1.schema.json` | AI model provider invocation request |
| Provider Event | `provider-event.v1.schema.json` | Streaming event emitted by model providers |
| Judgment Request | `judgment-request.v1.schema.json` | Input to fast judgment / rule verification |
| Judgment Result | `judgment-result.v1.schema.json` | Decision, score, and explanation from judgment |
| Capability Request | `capability-request.v1.schema.json` | Tool/side-effect execution request through gateway |
| Execution Receipt | `execution-receipt.v1.schema.json` | Cryptographic/audit proof of tool execution |
| Continuation Packet | `continuation-packet.v1.schema.json` | Canonical resumable execution state |
