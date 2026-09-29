---
id: 48f96f56-f4f5-4a0f-b5be-a5234f6c40d1
title: Keybind editor in Settings
type: feature
status: backlog
milestone: rimos
depends_on:
- afc3e3b0-be25-4170-8d79-2f60b4c846b2
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: additive
effort: m
layer: core
area: ui
---

## Problem

The engine can rebind (`Ui::rebind`) and saves keybinds.toml, but only tests call it; there's no UI and no Luau API.

## Proposal

- `ui.rebind(id, key)` and `ui.bindings()` for Luau.
- The Controls page lists every binding, searchable. Rebind by pressing a key; a conflict is named, and there's a reset per key.

## Acceptance criteria

- [ ] Rebinding Work to another key survives a restart (test)
- [ ] A conflict names the other binding and asks which keeps the key
