#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo "  CUSTOS MIGRATION: 11 CANONICAL CRATES RESTRUCTURING    "
echo "=========================================================="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "[1/4] Checking git status..."
if [[ -n $(git status -s) ]]; then
  echo "Error: Working directory is not clean. Please commit or stash changes."
  exit 1
fi

echo "[2/4] Phase 1: Removing dead & empty crates..."
# Remove empty judgment mocks and unused acp macros
git rm -rf crates/adapters/judgments/jev || true
git rm -rf crates/adapters/judgments/onnx || true
git rm -rf crates/adapters/judgments/rules || true
git rm -rf crates/adapters/judgments/contracts || true
git rm -rf crates/adapters/custos-acp-macros || true
rm -rf crates/adapters/judgments || true

echo "[3/4] Phase 2: Scaffolding and moving via git mv..."

# ----------------------------------------------------
# 1. custos-domain (Tier 0)
# ----------------------------------------------------
echo " -> Moving custos-domain..."
git mv crates/core/custos-domain crates/custos-domain

# ----------------------------------------------------
# 2. custos-core (Tier 1)
# ----------------------------------------------------
echo " -> Moving custos-core (kernel, authority, evidence, capability)..."
mkdir -p crates/custos-core/src
git mv crates/core/custos-kernel/src crates/custos-core/src/kernel
git mv crates/runtime/custos-security/src/authority crates/custos-core/src/authority
git mv crates/runtime/custos-security/src/evidence crates/custos-core/src/evidence
git mv crates/runtime/custos-security/src/gateway crates/custos-core/src/capability
git mv crates/runtime/custos-security/src/sandbox crates/custos-core/src/sandbox_policy
rm -rf crates/core/custos-kernel crates/runtime/custos-security

# ----------------------------------------------------
# 3. custos-provider (Tier 1)
# ----------------------------------------------------
echo " -> Moving custos-provider..."
mkdir -p crates/custos-provider/src
git mv crates/core/custos-provider-sdk/src/* crates/custos-provider/src/
git mv crates/core/custos-provider-types/src/lib.rs crates/custos-provider/src/types.rs
rm -rf crates/core/custos-provider-sdk crates/core/custos-provider-types

# ----------------------------------------------------
# 4. custos-persistence (Tier 1)
# ----------------------------------------------------
echo " -> Moving custos-persistence..."
git mv crates/infrastructure/custos-persistence crates/custos-persistence
rmdir crates/infrastructure || true

# ----------------------------------------------------
# 5. custos-bridge (Tier 1)
# ----------------------------------------------------
echo " -> Moving custos-bridge..."
git mv crates/core/custos-bridge crates/custos-bridge

# ----------------------------------------------------
# 6. custos-runtime (Tier 2)
# ----------------------------------------------------
echo " -> Moving custos-runtime (agent, engine, session, workflow, cognitive, gateway, context)..."
mkdir -p crates/custos-runtime/src
git mv crates/runtime/custos-agent/src crates/custos-runtime/src/agent
git mv crates/runtime/custos-engine/src crates/custos-runtime/src/engine
git mv crates/runtime/custos-session/src crates/custos-runtime/src/session
git mv crates/runtime/custos-workflow/src crates/custos-runtime/src/workflow
git mv crates/runtime/custos-cognitive/src crates/custos-runtime/src/cognitive
git mv crates/runtime/custos-gateway/src crates/custos-runtime/src/gateway
git mv crates/runtime/custos-context/src crates/custos-runtime/src/context
git mv crates/runtime/custos-context-management/src crates/custos-runtime/src/context_management
git mv crates/runtime/custos-memory-service/src crates/custos-runtime/src/memory_service
rm -rf crates/runtime/custos-* || true
rmdir crates/runtime || true

# ----------------------------------------------------
# 7. custos-adapters (Tier 2)
# ----------------------------------------------------
echo " -> Moving custos-adapters (model, mcp, sandboxes, local_inference, download_manager, roaming)..."
mkdir -p crates/custos-adapters/src/model
git mv crates/adapters/custos-providers/src crates/custos-adapters/src/model/providers
git mv crates/adapters/providers/claude/src crates/custos-adapters/src/model/claude
git mv crates/adapters/providers/antigravity/src crates/custos-adapters/src/model/antigravity
git mv crates/adapters/providers/codex/src crates/custos-adapters/src/model/codex
git mv crates/adapters/providers/local-model/src crates/custos-adapters/src/model/local_model
git mv crates/adapters/providers/fake/src crates/custos-adapters/src/model/fake

mkdir -p crates/custos-adapters/src/mcp
git mv crates/adapters/custos-mcp/src crates/custos-adapters/src/mcp/core
git mv crates/adapters/custos-adapters-mcp/src crates/custos-adapters/src/mcp/adapters

mkdir -p crates/custos-adapters/src/sandbox
git mv crates/adapters/sandboxes/linux-bubblewrap/src crates/custos-adapters/src/sandbox/bubblewrap
git mv crates/adapters/sandboxes/macos-seatbelt/src crates/custos-adapters/src/sandbox/seatbelt

git mv crates/adapters/custos-local-inference/src crates/custos-adapters/src/local_inference
git mv crates/adapters/custos-download-manager/src crates/custos-adapters/src/download_manager
git mv crates/adapters/custos-roaming/src crates/custos-adapters/src/roaming
rm -rf crates/adapters/* || true
rmdir crates/adapters || true

# ----------------------------------------------------
# 8. custos-sdk (Tier 3)
# ----------------------------------------------------
echo " -> Moving custos-sdk..."
git mv crates/core/custos-sdk crates/custos-sdk
mkdir -p crates/custos-sdk/src/wire_types
git mv crates/core/custos-sdk-types/src/* crates/custos-sdk/src/wire_types/
rm -rf crates/core/custos-sdk-types || true
rmdir crates/core || true

# ----------------------------------------------------
# 9. custos-daemon (Tier 4)
# ----------------------------------------------------
echo " -> Moving custos-daemon..."
git mv crates/app/custos-daemon crates/custos-daemon
git mv crates/app/custos-local-api/src crates/custos-daemon/src/local_api
rm -rf crates/app/custos-local-api || true

# ----------------------------------------------------
# 10. custos-cli (Tier 4)
# ----------------------------------------------------
echo " -> Moving custos-cli..."
git mv crates/app/custos-cli crates/custos-cli

# Move vscode out of crates/app to packages/
mkdir -p packages
git mv crates/app/custos-vscode packages/custos-vscode || true
rmdir crates/app || true

# ----------------------------------------------------
# 11. custos-packs (Tier 5)
# ----------------------------------------------------
echo " -> Moving custos-packs..."
mkdir -p crates/custos-packs/src
git mv crates/packs/custos-packs-engineering/src crates/custos-packs/src/engineering
git mv crates/packs/custos-packs-research/src crates/custos-packs/src/research
git mv crates/packs/custos-packs-assistant/src crates/custos-packs/src/assistant
mkdir -p crates/custos-packs/declarative
git mv crates/packs/custos-packs-engineering/declarative crates/custos-packs/declarative/engineering || true
rm -rf crates/packs/* || true
rmdir crates/packs || true

echo "[4/4] Consolidation moves complete. Directory structure in crates/:"
ls -la crates/

echo "=========================================================="
echo "  PHASE 1 & 2 COMPLETE! READY FOR CARGO.TOML WIRING      "
echo "=========================================================="
