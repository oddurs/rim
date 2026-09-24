---
id: 11b23c33-ff3a-4164-a7f7-594c17e79805
title: Luau types for the UI globals (ui, act, view)
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-24
updated: 2026-09-24
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

- [ ] `rim_ui` declares its globals once, and a `types/ui.d.luau` plus a reference page are generated from that, like `rim`
- [ ] A test fails if the checked-in UI types drift from the VM
- [ ] `scripts/check-luau.sh` checks `mods/*/ui` too, and the shipped UI scripts pass
