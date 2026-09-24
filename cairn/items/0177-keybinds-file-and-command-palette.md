---
id: 14df87a8-1789-4cb0-ab15-f105f1b13085
title: Keybinds file and command palette
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 01e4d9fe-095a-47d9-b06b-5d2c5099f063
- 1e977052-1ce0-4280-a9ea-a6ff0c8f4cc8
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: additive
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Every action a mod adds should be reachable without that mod drawing a
button. A registry of named actions with keys makes the palette a list and
the settings' keybinds tab the registry, editable.

## What

- `ui.bind("core:pause", { key = "space" }, fn)`: a registry with defaults,
  conflicts reported like replaced components.
- A keybinds file per player overrides defaults.
- The command palette: a window with a text input over the registry.

## Acceptance criteria

- [ ] A mod's bound action fires from its key and from the palette
- [ ] Two mods binding one key is reported
- [ ] The keybinds file overrides a default and survives a restart
