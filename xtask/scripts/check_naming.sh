#!/usr/bin/env bash
set -euo pipefail

# Enforce product/package naming while reporting the staged Goose compatibility debt.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
for root in platform apps sdk workbenches ui; do
  [[ -d "$root" ]] || { echo "ERROR: missing source root $root" >&2; exit 1; }
done
failures=0

if rg -n '^name\s*=\s*"goose([_-]|"|$)' platform apps sdk workbenches --glob 'Cargo.toml'; then
  echo "ERROR: Rust package/library targets owned by Custos must use a Custos name." >&2
  failures=1
fi

if rg -n '"name"\s*:\s*"(@[^"/]+/)?goose([_-]|"|$)' \
  ui/desktop packages --glob 'package.json'; then
  echo "ERROR: Custos-owned application packages must not use Goose product names." >&2
  failures=1
fi

legacy_files="$({ rg -l -i 'goose' platform apps sdk workbenches ui xtask config packages examples evals \
  --hidden --glob '!**/node_modules/**' --glob '!**/dist/**' 2>/dev/null || true; } | wc -l | tr -d ' ')"
echo "Goose-named compatibility/debt files: ${legacy_files}"
echo "Classify these with docs/research/upstream-dissection.md and docs/status/README.md; the count is informational until the compatibility exit gate."

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Naming checks passed."
