---
id: 478
uid: 508ad373-cb34-4788-ab4b-820c42dcda01
title: Lighting quality in the command palette
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A key changes the preset at once and it survives a restart (autotest)

## 2026-09-27

Done as: act.lighting(preset) in the UI API, and core's palette bindings Lighting: low, medium, high and ultra, with no keys (like render scale). The preset is saved as [lighting] quality, keeping whatever else the player set by hand there, and the game re-reads that file so it runs as it will after a restart. Test: a file with quality low and sun_steps 40 saves ultra, keeps the 40 steps and the other settings; no [lighting] is made. Autotest: a key lent to Lighting: low switches to it at once (16 sun steps, hard shadows), and back to medium. The autotest has no settings file, so the restart half is the unit test's.
