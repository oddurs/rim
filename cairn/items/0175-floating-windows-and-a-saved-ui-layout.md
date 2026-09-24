---
id: 1e977052-1ce0-4280-a9ea-a6ff0c8f4cc8
title: Floating windows and a saved UI layout
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 5514aac6-0300-4ff6-be19-fd44a5e50089
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: additive
effort: m
layer: engine
area: ui
---

## Why

The `windows` layer exists and composes; nothing manages it. The mod
manager (platform, p0), settings, the load menu and trade all need a
window a mod declares and the engine moves.

## What

- A window record the engine keeps: position, size, z, open. Drag, resize
  and close handled in routing; chrome drawn by the kit.
- `ui.window(id, { title, w, h, resizable }, component)`. A mod never moves
  a window; two mods declaring one id is a conflict, reported.
- Layout is per player and per machine: saved beside client settings, not
  in the save game. Keyed by id, it survives a mod update and a restart.

## Acceptance criteria

- [x] Drag, resize, close and z-order each have a headless test
- [x] A layout survives a restart and a mod update
- [x] Two mods declaring one window id is reported like a replaced component

## 2026-09-24

Windows are engine records (x, y, w, h, open) in logical pixels, kept in stacking order; core draws the chrome through ui.window_chrome(kit.window) and marks the title bar, close button and grip with handle = move/close/resize, which is all the engine routes on. A press anywhere on a window raises it; a drag moves or resizes the record, so placement changes without a relayout (the chrome's layout is cached at the origin and offset). Opening from a handler goes through ui.open/close/toggle as queued ops the engine applies after the call, so Luau never holds window state. The layout is TOML keyed by id, restored before the first frame and saved by the client to the per-user data dir when the engine reports it dirty (after a release or an open/close, never mid-drag). The kit gallery is now the first real window.
