---
id: 14df87a8-1789-4cb0-ab15-f105f1b13085
title: Keybinds file and command palette
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
depends_on:
- 01e4d9fe-095a-47d9-b06b-5d2c5099f063
- 1e977052-1ce0-4280-a9ea-a6ff0c8f4cc8
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] A mod's bound action fires from its key and from the palette
- [x] Two mods binding one key is reported
- [x] The keybinds file overrides a default and survives a restart

## 2026-09-24

Bindings live in the UI VM's registry: ui.bind(id, { key, label }, fn) with keys normalised (modifiers ctrl, alt, shift in that order, cmd counts as ctrl). The client names every key it sees ('space', 'f3', 'ctrl+k') in Input.pressed; the engine runs the bound handler unless a text input has focus, and reports captured_keys so the client skips its own map. Core's own keys (pause, speeds, overlay, draft, centre, profiler, devtools) moved out of the client into mods/core/ui/keys.luau as bindings, so the client keeps only Escape and Tab; that is what makes a keybinds file real. The player's overrides are a [keys] table in keybinds.toml beside ui-layout.toml, restored before the first frame; view.binds() gives the palette every action with its current key. Two mods on one id or one key are reported like a replaced component. The palette is a core window (ctrl+k) with a text input over view.binds(); ui.focus(id) hands the input the keyboard once it is laid out, since window ops and focus both apply after the handler that asked. Also: the budget tests now scale their budget by a quick calibration of the machine's current speed, because the binary's other tests share the cores and the median rebuild frame doubled under that load.
