#!/usr/bin/env bash
set -euo pipefail

# Architecture Guardrail: Layering & Dependency Direction Checker
# Ensures strict dependency order for the 4 canonical blocks:
# Block 1 (contracts) <- Block 2 (storage) & Block 3 (integrations) <- Block 4 (engine) <- Apps (daemon, cli, desktop)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

for source_root in platform/contracts/src platform/storage/src platform/integrations/src platform/engine/src; do
    [[ -d "$source_root" ]] || { echo "ERROR: missing source root $source_root" >&2; exit 1; }
done

# Search failures are errors, not proof that a boundary passed.
reject_source() {
    local pattern="$1" root="$2" status
    if grep -rnE --include='*.rs' "$pattern" "$root"; then
        echo "ERROR: forbidden dependency in $root" >&2
        exit 1
    else
        status=$?
        [[ "$status" -eq 1 ]] || exit "$status"
    fi
}

# 1. Contracts must have zero concrete I/O
reject_source 'rusqlite|reqwest|std::fs|std::net' platform/contracts/src

# 2. Engine must not depend on concrete persistence/integrations in production (pure domain orchestration)
reject_source 'rusqlite|reqwest|custos_daemon' platform/engine/src
reject_source '(std|tokio)::process::Command::new' platform/engine/src

# 3. Daemon application layer must not spawn unchecked processes
if [[ -d "apps/daemon/src/application" ]]; then
    reject_source '(std|tokio)::process::Command::new' apps/daemon/src/application
fi

echo "[OK] Architectural layering and dependency direction verified."
