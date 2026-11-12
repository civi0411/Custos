# Canonical Model System

Provides a unified view of model metadata (pricing, capabilities, context limits) across different LLM providers. 
Normalizes provider-specific model names (e.g., `claude-sonnet-4-5-20250929`)
to canonical IDs (e.g., `anthropic/claude-sonnet-4.5`).

## Canonical Model Data
This directory holds normalized model metadata (pricing, capabilities, context limits) across LLM providers in `platform/ai/src/catalog/canonical/data/`.
Normalizes provider-specific model names (e.g., `claude-sonnet-4-5-20250929`)
to canonical IDs (e.g., `anthropic/claude-sonnet-4.5`).

- `canonical_models.json`: Generated canonical model registry loaded by `platform/ai` (`custos-provider`).
- `canonical_mapping_report.json`: Mapping report tracking unmapped and normalized upstream model IDs.
