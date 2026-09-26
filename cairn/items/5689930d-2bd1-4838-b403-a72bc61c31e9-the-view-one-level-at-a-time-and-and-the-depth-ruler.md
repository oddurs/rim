---
id: 5689930d-2bd1-4838-b403-a72bc61c31e9
title: 'The view: one level at a time, [ and ], and the depth ruler'
type: feature
status: backlog
milestone: depth
depends_on:
- acd85584-7f3d-4348-8d56-5242a0bdb620
created: 2026-09-26
updated: 2026-09-26
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

- [ ] `[` and `]` change level; the ruler shows counts and alerts (autotest by node id)
- [ ] Render bench on the stacked scene inside the 4 ms CPU budget, recorded here and in §6d
- [ ] Changing level with both levels cached costs no chunk rebuilds (test on the mesh cache)
- [ ] Tray groups work from Tab and Shift+Tab
