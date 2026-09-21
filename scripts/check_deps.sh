#!/usr/bin/env bash
set -euo pipefail

# Custos Dependency Rule Verification Script
# Ensures that architectural boundaries and layer directions are respected.

echo "=== Verifying Custos Architectural Boundaries ==="

CORE_DOMAIN_CARGO="crates/core-domain/Cargo.toml"

# Rule 1: core-domain must have ZERO external dependencies beyond serde/thiserror
if grep -q "path = \"\.\." "$CORE_DOMAIN_CARGO"; then
    echo "ERROR: crates/core-domain depends on other workspace crates! Core domain must be pure."
    exit 1
fi

echo "[OK] core-domain is isolated from internal crates."

# Rule 2: adapters must not depend on apps
if grep -rn 'path = "\.\./\.\./apps/' adapters/; then
    echo "ERROR: adapters must never depend on apps!"
    exit 1
fi

echo "[OK] adapters do not depend on apps."

# Rule 3: crates must not depend on apps
if grep -rn 'path = "\.\./apps/' crates/; then
    echo "ERROR: crates must never depend on apps!"
    exit 1
fi

echo "[OK] crates do not depend on apps."

echo "=== All Dependency Rules Passed! ==="
