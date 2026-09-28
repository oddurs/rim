---
id: 41843e8a-01d9-4a74-8da5-b2e95718b5f4
title: The rig on F12, and a Bodies page in the kit gallery
type: feature
status: backlog
milestone: people
depends_on:
- 809e1fc5-242f-4e61-aaf8-a3df2be6a36e
- 8469a7ff-bee8-4237-9fc6-a424aea6c362
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
pillar:
- plugin-first
- performance
effort: s
layer: client
area: ui
---

## Why

A modder changing a body, a gait or a hair style needs to see what the
renderer did and why. DESIGN.md §6h promises a rig overlay on F12 and a page
in the kit gallery.

## What

- **F12 on a pawn** (core's devtools, `mods/core/ui/devtools.luau`): its
  sockets, the facing arrow, the gait and the facts that picked it, the
  phase, and the mod that drew each part.
- **A Bodies page in the kit gallery:** every body in every gait side by side,
  animated, with a row of every hair feature across the skin palette.
- The overlay reads a new `view.pawn_pose(id)` returning gait, facts, phase,
  facing and parts with owning mod.

## Acceptance criteria

- [ ] F12 on a pawn shows the overlay, and the gait's reason names the facts that matched (screenshot in the UI shots)
- [ ] The gallery page draws every body × gait, and every feature × skin tone (UI shot)
- [ ] `view.pawn_pose` is in the generated UI API and types
