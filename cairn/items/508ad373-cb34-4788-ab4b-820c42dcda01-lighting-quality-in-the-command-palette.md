---
id: 508ad373-cb34-4788-ab4b-820c42dcda01
title: Lighting quality in the command palette
type: feature
status: backlog
milestone: lighting
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: additive
effort: s
layer: client
area: ui
---

## Why

The lighting presets (12fbe8fb) live in `[lighting]` in the player's settings file, and nothing in the game changes them. Render scale already has keys and an `act.render_scale`; lighting should be as easy to reach.

## What

- `act.lighting(name)` in the UI API, and core's keys: Lighting low, medium, high and ultra, saved to the settings file as render scale is.

## Acceptance criteria

- [ ] A key changes the preset at once and it survives a restart (autotest)
