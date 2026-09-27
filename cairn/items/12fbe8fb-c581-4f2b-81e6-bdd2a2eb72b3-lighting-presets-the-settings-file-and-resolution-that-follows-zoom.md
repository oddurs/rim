---
id: 12fbe8fb-c581-4f2b-81e6-bdd2a2eb72b3
title: Lighting presets, the settings file, and resolution that follows zoom
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: render
---

## Why

Performance is the point. A player on the reference laptop must get a lit world inside budget without touching a setting, and a player with a better GPU should be able to ask for more. DESIGN.md §6e.

## What

- `low`, `medium` (default), `high`, `ultra`: texels per cell, sky steps, bodies with shadows, soft shadows, contact shadows, dynamic cap and rays, bounce blur, upsampling, sky rebuild threshold. Values as in the concept's table.
- `[lighting]` in the player's settings file: `quality` plus any per-setting override.
- A light texel is never smaller than 4 screen pixels: zooming out lowers texels per cell.

## Acceptance criteria

- [x] Presets and overrides are read from the settings file, and a bad value is an error, not a default (test)
- [x] At the minimum zoom the light buffer is 1 texel per cell (test)
- [x] Bench numbers for each preset on the dusk scene recorded here

## 2026-09-26

No longer waits on moving lights (5a69f9c9, blocked on fire). The dynamic cap and rays settings arrive with that item; this one covers the rest.

## 2026-09-27

Measured (release, 250x250 bench, Apple M4 Pro under load, CPU only): all four presets draw the dusk scene with the lighting passes doing no work on steady frames; the multiply is about 0.02 ms of CPU at every preset (one run of low read 0.96 ms, an outlier). Rebuild costs per preset, from --lighting low|medium|high|ultra: sun 0.54 / 1.22 / 0.53 / 1.06 ms, firelight 1.07 / 0.77 / 1.25 / 2.97 ms, noisy; ultra's 4-texel cost is not in them: the dusk view is at the minimum zoom, where every preset is capped at 1 or 2 texels a cell, so ultra measured what high does, plus longer sun marches. GPU per preset needs a GPU that can time a pass; CI's bench runs medium. Not in the presets: bodies casting shadows (only the sun exists until 1a17d685), dynamic caps (5a69f9c9, blocked on fire) and bounce blur (never built: room fill does its job). The palette command is filed separately (508ad373).

Review: `auto` is cut and filed as 24bad102. It watched mean frame time, so a slow sim would have turned the lights down for nothing. Bicubic upsampling is cut too: at 2 texels a cell it changed nothing you could see, and it cost four taps per light read. Nothing writes `[lighting]` yet, so the settings test reads each preset and override back from TOML text rather than round-tripping. The palette item (508ad373) owns the writing. The sun pass is keyed on its step count too, so a changed preset redoes it. The settings file is read once at launch, and a file that exists but can't be read is reported.
