---
id: 11b23c33-ff3a-4164-a7f7-594c17e79805
title: Luau types for the UI globals (ui, act, view)
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: additive
layer: tooling
area: modding
effort: s
---

## Why

Sim scripts get completion and type errors from `types/rim.d.luau`, but UI
scripts see `ui`, `act` and `view` as unknown globals, so `scripts/check-luau.sh`
skips `mods/*/ui`.

## Acceptance criteria

- [x] `rim_ui` declares its globals once, and a `types/ui.d.luau` plus a reference page are generated from that, like `rim`
- [x] A test fails if the checked-in UI types drift from the VM
- [x] `scripts/check-luau.sh` checks `mods/*/ui` too, and the shipped UI scripts pass

## 2026-09-24

Declared in one sorted table (crates/rim_ui/src/api.rs) rather than inline at each registration like the sim: view functions change with most features and vm.rs is the busiest shared file, so a flat list is easier to review and conflicts less. tests/api_types.rs compares it with what the VM registers (UiVm::api_names), so a new function without a declaration fails with the names to add. luau-lsp then found the UI guide's anchored sample passing children as a second argument to ui.col, which silently dropped them; fixed. check-luau.sh checks sim and UI scripts against their own definitions only.
