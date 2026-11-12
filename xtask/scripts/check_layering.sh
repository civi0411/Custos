#!/usr/bin/env bash
set -euo pipefail

# Architecture Guardrail: Layering & Dependency Direction Checker
# Ensures strict dependency order:
# Layer 0 (custos-domain) <- Layer 1 (custos-core) <- Layer 2 (runtime/adapters/persistence) <- Layer 4 (daemon)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

for source_root in platform/domain/src platform/core/src platform/runtime/src/oi; do
    [[ -d "$source_root" ]] || { echo "ERROR: missing source root $source_root" >&2; exit 1; }
done

# Search failures are errors, not proof that a boundary passed.
reject_source() {
    local pattern="$1" root="$2" status
    if rg -n --glob '*.rs' "$pattern" "$root"; then
        echo "ERROR: forbidden dependency in $root" >&2
        exit 1
    else
        status=$?
        [[ "$status" -eq 1 ]] || exit "$status"
    fi
}
reject_source 'tokio|rusqlite|reqwest|std::fs|std::net' platform/domain/src
reject_source 'custos_runtime|custos_persistence|custos_adapters|custos_daemon|custos_protocol' platform/core/src
reject_source 'rusqlite' platform/runtime/src/oi
if [[ -d "platform/components/src" ]]; then
    reject_source 'rusqlite|reqwest' platform/components/src
fi
if [[ -d "platform/ai/src" ]]; then
    reject_source 'keyring' platform/ai/src
fi
if [[ -d "platform/runtime/src" ]]; then
    reject_source '(std|tokio)::process::Command::new' platform/runtime/src
fi
if [[ -d "apps/daemon/src/application" ]]; then
    reject_source '(std|tokio)::process::Command::new' apps/daemon/src/application
fi
if [[ -d "platform/ai/src/contracts" ]]; then
    reject_source 'crate::(codecs|formats|cloud)\b' platform/ai/src/contracts
fi
echo "Source boundary checks passed; dependency graph is checked separately by check_deps.sh."
