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

# Rule 4: core-domain must never depend on storage or async runtime crates
for banned in "tokio" "rusqlite" "axum" "sqlx" "reqwest"; do
    if grep -q "^$banned = " "$CORE_DOMAIN_CARGO"; then
        echo "ERROR: crates/core-domain must never depend on $banned!"
        exit 1
    fi
done

echo "[OK] core-domain is free from async/storage dependencies."

echo "=== All Dependency Rules Passed! ==="

