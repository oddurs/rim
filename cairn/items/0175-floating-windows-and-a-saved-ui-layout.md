---
id: 1e977052-1ce0-4280-a9ea-a6ff0c8f4cc8
title: Floating windows and a saved UI layout
type: feature
status: backlog
milestone: colony
depends_on:
- 5514aac6-0300-4ff6-be19-fd44a5e50089
created: 2026-09-23
updated: 2026-09-24
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

- [ ] Drag, resize, close and z-order each have a headless test
- [ ] A layout survives a restart and a mod update
- [ ] Two mods declaring one window id is reported like a replaced component
