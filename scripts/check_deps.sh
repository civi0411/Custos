#!/usr/bin/env bash
set -euo pipefail

# Validate hard dependency boundaries against Cargo's resolved workspace graph.
# Known transitional edges are reported by default and fail only with --strict.

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
workspace_ids="$(jq -r '.workspace_members[]' <<<"$metadata")"

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

app_names="$(jq -c '[.packages[] | select(.manifest_path | contains("/crates/app/")) | .name]' <<<"$metadata")"
adapter_to_app="$(jq -r --argjson apps "$app_names" \
  '.packages[] | select(.manifest_path | contains("/crates/adapters/")) as $pkg
   | $pkg.dependencies[]? | select(.path != null) as $dep
   | select($apps | index($dep.name))
   | "\($pkg.name) -> \($dep.name)"' <<<"$metadata")"
if [[ -n "$adapter_to_app" ]]; then
  echo "ERROR: adapter packages must not depend on app binaries:" >&2
  sed 's/^/  - /' <<<"$adapter_to_app" >&2
  exit 1
fi
echo "[OK] adapter packages do not depend on app binaries."

transitional="$(jq -r '
  .packages[] as $pkg
  | $pkg.dependencies[]? | select(.path != null) as $dep
  | select(
      ($pkg.name == "custos-bridge" and (["custos-persistence", "custos-session"] | index($dep.name))) or
      ($pkg.name == "custos-cli" and (["custos-persistence", "custos-kernel", "custos-providers", "custos-mcp", "custos-adapters-mcp"] | index($dep.name)))
    )
  | "\($pkg.name) -> \($dep.name)"' <<<"$metadata")"
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
