#!/usr/bin/env bash
set -euo pipefail

# Enforce product/package naming while reporting the staged Goose compatibility debt.

failures=0

if rg -n '^name\s*=\s*"goose([_-]|"|$)' crates --glob 'Cargo.toml'; then
  echo "ERROR: Rust package/library targets owned by Custos must use a Custos name." >&2
  failures=1
fi

if rg -n '"name"\s*:\s*"(@[^"/]+/)?goose([_-]|"|$)' \
  services ui/desktop packages oidc-proxy --glob 'package.json'; then
  echo "ERROR: Custos-owned application packages must not use Goose product names." >&2
  failures=1
fi

if rg -n '/Users/[^/]+/Project/AgentHub' tools/repo_intelligent/src tools/repo_intelligent/README.md; then
  echo "ERROR: Nexus must not depend on a developer-specific AgentHub path." >&2
  failures=1
fi

legacy_files="$({ rg -l -i 'goose' crates ui services scripts config packages examples evals workflow_recipes oidc-proxy \
  --hidden --glob '!**/node_modules/**' --glob '!**/dist/**' 2>/dev/null || true; } | wc -l | tr -d ' ')"
echo "Goose-named compatibility/debt files: ${legacy_files}"
echo "Classify these with docs/status/goose-naming-migration.md; the count is informational during N0-N5."

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Naming checks passed."
