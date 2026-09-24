#!/usr/bin/env bash
# Type-check every mod's sim scripts against the declared rim API
# (types/rim.d.luau, generated from crates/rim_sim/src/script.rs).
#
#   LUAU_LSP=path/to/luau-lsp ./scripts/check-luau.sh
#
# Get luau-lsp from https://github.com/JohnnyMorganz/luau-lsp/releases. UI
# scripts aren't checked yet: their globals (ui, act, view) aren't declared.
set -euo pipefail
cd "$(dirname "$0")/.."
lsp="${LUAU_LSP:-luau-lsp}"
"$lsp" analyze --platform=standard --definitions:@rim=types/rim.d.luau mods/*/scripts/*.luau
echo "mod scripts match the rim API"
