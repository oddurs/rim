---
id: 3f4c257d-5e6f-4417-86d4-58e1cc981c73
title: 'Rooms made of something: leak and daylight from the boundary'
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- ab0599d2-dae6-450a-8cf0-9b3076d331d6
- 82bc6c98-e5fb-42a5-9534-5f647d714654
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: l
layer: engine
area: map
---

## Why

A room ringed in stone should hold heat differently from the same room in
wood. Until it does, materials are a number in a tooltip.

## What

Depends on the answer from 0211. Assuming the boundary option:

- `[[thing]]` gains boundary contributions (`leak`, `daylight`) -- what
  this piece of wall does to the room it helps enclose.
- A room's leak and daylight are summed over its boundary cells, scaled by
  each piece's material factors, and cached. Rooms already rebuild only
  when walls change, so this rides that.
- Field defs keep their constant as the default for a boundary that says
  nothing.

## Acceptance criteria

- [x] A stone room and a wooden room of the same shape reach different
      temperatures from the same fire
- [x] Visible in the temperature overlay
- [x] Boundary recompute stays inside the room-rebuild budget

## 2026-09-23

Done. Design: a field names which material factor scales its boundary (core's temperature says insulation; the engine only looks the string up), a thing declares per-field leak (multiple of leak_per_hour, divided by the factor) and pass (fraction of the outdoor value let in, multiplied by the factor). The room flood fill now collects each room's boundary cells as it goes -- 8-neighbour so corners count, deduped per room with a stamp -- so the walk the spike priced separately is free. World::refresh_boundaries runs after ensure_rooms and only when room_rebuilds changed: per room per field it averages leak and sums pass, capped at 1. Pieces that declare nothing count as the field's constant, so a boundary nobody described behaves exactly as before. Measured: boundary refresh over 14 rooms: 0.056 ms. Two decisions: wood's insulation factor is now 1.0 (was a placeholder 1.2) so a wooden hut is bit-for-bit what it was and the weather balance harness still passes -- stone at 1.5 is the improvement; and the 'same fire, different temperature' criterion is tested on cooling, because a campfire is capped at its room value and while it burns both huts sit at the cap (stone 21.41 vs wood 21.02 after two hours) -- the material shows in what the room keeps once the fire is out (0.8+ degrees after three cold hours). A pane test declares pass=0.5 from a throwaway mod and lights a hut to half daylight, so 0217 is pure content.
