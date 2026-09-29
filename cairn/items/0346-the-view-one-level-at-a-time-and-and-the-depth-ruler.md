---
id: 346
uid: 5689930d-2bd1-4838-b403-a72bc61c31e9
title: 'The view: one level at a time, [ and ], and the depth ruler'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- 393
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: m
layer: client
area: render
pillar:
- performance
---

## Why

Depth is only as good as moving through it. The player sees one level, knows where everyone is, and changes level without thinking (DESIGN.md §6d).

## What

- The client draws one level. Levels above are cut away. The level below shows through air cells, dimmed.
- Rock that no pawn has stood beside is drawn plain; its material and veins show once seen. The seen bit is sim state, one bit per cell, so prospecting (`db7f1e06`) can use it.
- Chunk meshes are keyed by `(z, chunk)`. The current level and the one below stay cached; the rest are evicted least-recently-used.
- Keys: `[` down, `]` up. PR #141 bound them to the dock's tray groups; those move to Tab and Shift+Tab while a tray is open.
- The depth ruler, `mods/core/ui/depth.luau`: each level, who is on it, alerts on it, and levels not yet reached hidden. Clicking a level goes there. Selecting a colonist on another level moves the view to it.
- The UI `view` gains the current level and per-level counts.
- `rim --bench-render` gets a stacked scene: a surface colony over a dug −1 with pits and a stairwell.

## Acceptance criteria

- [x] `[` and `]` change level; the ruler shows counts and alerts (autotest by node id)
- [x] Render bench on the stacked scene inside the 4 ms CPU budget, recorded here and in §6d
- [x] Changing level with both levels cached costs no chunk rebuilds (test on the mesh cache)
- [x] Tray groups work from Tab and Shift+Tab

## 2026-09-26

Lighting keys its buffers by z the same way as the chunk meshes (3124bd7b): the viewed level and the one below are cached, and the level below through air is drawn with its own light, dimmed. Evict them together.

## 2026-09-26

The user requires lighting to be smooth across z (DESIGN.md §6e, Depth). Lighting reads the neighbouring levels' light through openings, so 3124bd7b caches the level above as well as the one below. If the mesh cache keeps only z and z−1, eviction has to account for z+1. The level switch crossfades the two lighting buffers for 150 ms, so draw both levels' meshes during that window.

## 2026-09-27

The level lives on the camera (cam.z), so tile_at, drags, orders and the UI's cell lookups all carry it. Meshes and Ground keep z-1..=z+1 (lighting's crossfade needs z+1) and free the rest when the level changes. The level below is drawn from the chunks under this level's air cells only. The tray groups' move to Tab needed ui.bind's new 'when', or Tab would stop selecting the next colonist. Found and fixed on the way: a hunt or cancel rectangle reached pawns on every level (in_rect ignored z); build_rects dropped z.

## 2026-09-27

Render bench, stacked scene (M4 Pro, --sprite-mods 20, 100 frames): 'stacked' (the whole map from the surface, −1 through three pits) 0.66–0.67 ms world CPU; 'below' (the whole of −1) 0.31–0.46 ms, 70 calls, 1.46 M indices; 0 rebuilds in both. Budget 4 ms; --check passes. A third run under load from other agents' builds read 2.6 and 0.58 ms. Below ground the surface's lightmap, weather and roofs are skipped until lighting keeps a lightmap per level (3124bd7b); without that, −1 showed the colony's indoor darkness and its fires.
