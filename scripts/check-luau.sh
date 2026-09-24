#!/usr/bin/env bash
# Type-check every mod's scripts against the declared APIs: sim scripts
# against types/rim.d.luau (generated from crates/rim_sim/src/script.rs), UI
# scripts against types/ui.d.luau (from crates/rim_ui/src/api.rs).
#
#   LUAU_LSP=path/to/luau-lsp ./scripts/check-luau.sh
#
# Get luau-lsp from https://github.com/JohnnyMorganz/luau-lsp/releases.
set -euo pipefail
cd "$(dirname "$0")/.."
lsp="${LUAU_LSP:-luau-lsp}"
check() {
  local defs="$1" dir="$2"
  local files
  files=$(find mods/*/"$dir" -name '*.luau' | sort)
  # shellcheck disable=SC2086 # one path per word
  "$lsp" analyze --platform=standard "--definitions:$defs" $files
}
check "@rim=types/rim.d.luau" scripts
check "@ui=types/ui.d.luau" ui
echo "mod scripts match the rim and ui APIs"
