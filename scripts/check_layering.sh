#!/usr/bin/env bash
set -euo pipefail

# Architecture Guardrail: Layering & Dependency Direction Checker
# Ensures strict dependency order:
# Layer 0 (custos-domain) <- Layer 1 (custos-core) <- Layer 2 (runtime/adapters/persistence) <- Layer 4 (daemon)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== [1/3] Checking custos-domain Zero-I/O Invariant ==="
DOMAIN_FORBIDDEN=("tokio" "rusqlite" "reqwest" "std::fs" "std::net")
for term in "${DOMAIN_FORBIDDEN[@]}"; do
    if grep -rn --include="*.rs" "$term" crates/custos-domain/src/; then
        echo "ERROR: custos-domain violates Zero-I/O by containing '$term'" >&2
        exit 1
    fi
done
echo "OK: custos-domain is pure Zero-I/O."

echo "=== [2/3] Checking custos-core layer purity ==="
CORE_FORBIDDEN=("custos_runtime" "custos_persistence" "custos_adapters" "custos_daemon")
for term in "${CORE_FORBIDDEN[@]}"; do
    if grep -rn --include="*.rs" "$term" crates/custos-core/src/; then
        echo "ERROR: custos-core violates layer boundary by importing '$term'" >&2
        exit 1
    fi
done
echo "OK: custos-core does not import outer layers."

echo "=== [3/3] Checking OI engine I/O boundaries ==="
if [ -d "crates/custos-runtime/src/oi" ]; then
    if grep -rn --include="*.rs" "rusqlite" crates/custos-runtime/src/oi/; then
        echo "ERROR: custos-runtime/oi must not import rusqlite directly; use StoragePort" >&2
        exit 1
    fi
fi
echo "OK: OI engine respects boundary constraints."

echo "All layering boundary checks passed successfully!"
