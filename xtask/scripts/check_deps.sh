#!/usr/bin/env bash
set -euo pipefail

# Validate hard dependency boundaries against Cargo's resolved workspace graph.
# Known transitional edges are reported by default and fail only with --strict.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

strict=false
if [[ "${1:-}" == "--strict" ]]; then
  strict=true
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--strict]" >&2
  exit 2
fi

command -v cargo >/dev/null || { echo "ERROR: cargo is required" >&2; exit 2; }
command -v jq >/dev/null || { echo "ERROR: jq is required" >&2; exit 2; }

metadata="$(cargo metadata --offline --no-deps --format-version 1)"

domain_id="$(jq -r '.packages[] | select(.name == "custos-domain") | .id' <<<"$metadata")"
if [[ -z "$domain_id" ]]; then
  echo "ERROR: custos-domain is not a Cargo workspace member" >&2
  exit 1
fi

domain_workspace_deps="$(jq -r --arg id "$domain_id" \
  '.packages[] | select(.id == $id) | .dependencies[]? | select(.path != null) | .name' \
  <<<"$metadata")"
if [[ -n "$domain_workspace_deps" ]]; then
  echo "ERROR: custos-domain must not depend on workspace crates:" >&2
  sed 's/^/  - /' <<<"$domain_workspace_deps" >&2
  exit 1
fi
echo "[OK] custos-domain has no workspace-crate dependencies."

forbidden_edges="$(jq -r '
  .packages[] as $pkg
  | $pkg.dependencies[]? | select(.kind == null or .kind == "normal") as $dep
  | select(
      ($pkg.name == "custos-domain" and (["tokio", "rusqlite", "reqwest"] | index($dep.name))) or
      ($pkg.name == "custos-protocol" and ($dep.path != null) and ($dep.name != "custos-domain")) or
      ($pkg.name == "custos-sdk" and (["custos-core", "custos-runtime", "custos-persistence", "custos-adapters", "custos-daemon", "custos-workbench-research", "custos-workbench-coding", "custos-workbench-assistant", "custos-workbench-quant"] | index($dep.name))) or
      ($pkg.name == "custos-daemon" and ($dep.name == "custos-sdk")) or
      ($pkg.name == "custos-core" and (["custos-runtime", "custos-persistence", "custos-adapters", "custos-daemon", "custos-protocol"] | index($dep.name))) or
      ($pkg.name == "custos-packs" and ($dep.name == "reqwest")) or
      ($pkg.name == "custos-provider" and ($dep.name == "keyring")) or
      ($pkg.name == "custos-runtime" and (["custos-persistence", "rusqlite", "custos-daemon"] | index($dep.name))) or
      ($pkg.name == "custos-app-cli" and (["tauri", "tauri-build", "tauri-plugin-opener", "custos-persistence", "custos-core", "custos-adapters", "custos-daemon"] | index($dep.name))) or
      ($pkg.name == "custos-app-desktop" and (["custos-persistence", "custos-core", "custos-adapters", "custos-daemon"] | index($dep.name)))
    )
  | "\($pkg.name) -> \($dep.name)"' <<<"$metadata")"
if [[ -n "$forbidden_edges" ]]; then
  echo "ERROR: forbidden production dependency edges:" >&2
  printf '%s\n' "$forbidden_edges" >&2
  exit 1
fi

app_names="$(jq -c '[.packages[] | select(.manifest_path | contains("/apps/")) | .name]' <<<"$metadata")"
adapter_to_app="$(jq -r --argjson apps "$app_names" \
  '.packages[] | select(.manifest_path | contains("/platform/adapters/")) as $pkg
   | $pkg.dependencies[]? | select(.path != null) | select(.kind == null or .kind == "normal") as $dep
   | select($apps | index($dep.name))
   | "\($pkg.name) -> \($dep.name)"' <<<"$metadata")"
if [[ -n "$adapter_to_app" ]]; then
  echo "ERROR: adapter packages must not depend on app binaries:" >&2
  sed 's/^/  - /' <<<"$adapter_to_app" >&2
  exit 1
fi
echo "[OK] adapter packages do not depend on app binaries."

transitional=""
if [[ -n "$transitional" ]]; then
  echo "TRANSITIONAL dependency edges (remove through ports/composition):" >&2
  sed 's/^/  - /' <<<"$transitional" >&2
  if [[ "$strict" == true ]]; then
    exit 1
  fi
fi

if [[ "$strict" == true ]]; then
  echo "Dependency boundary checks passed (strict mode)."
else
  echo "Dependency boundary checks passed."
fi
