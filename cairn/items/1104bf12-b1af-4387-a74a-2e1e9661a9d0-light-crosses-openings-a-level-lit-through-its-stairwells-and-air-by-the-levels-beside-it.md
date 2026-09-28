---
id: 1104bf12-b1af-4387-a74a-2e1e9661a9d0
title: 'Light crosses openings: a level lit through its stairwells and air by the levels beside it'
type: feature
status: doing
milestone: lighting
assignee: Oddur Sigurdsson
claimed: 2026-09-28
depends_on:
- 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: render
---

## Why

A torch at the top of a stairwell should light the steps below, and a fire in a pit should glow on its rim. With each level lit on its own (3124bd7b), light stops dead at every opening. DESIGN.md §6e, Depth.

## What

- Each level's light adds the level above's through its openings (stairwells, ladders) and the level below's through its air cells, read from a blurred half-resolution copy of that level's light and attenuated 0.5 per level.
- Skipped on a level with no openings.

## Acceptance criteria

- [x] A torch at the top of a stairwell lights the steps on the level below, fading with distance from the opening, with no visible seam at the opening's edge (screenshot pair, both levels)

## 2026-09-28

Done as: each light on the next level up or down that reaches an opening into this level gives a light there on this level. The openings are air above or below, and stair and ladder portals. The new light is as bright as the old one still is at the opening, halved (ACROSS), and reaches as far as it has left to go. This level's walls shade it in the ordinary bake. The bake is also keyed on the level revisions of the levels beside, so a stair or pit dug there rebakes this one. Done on the CPU, in place of the blurred half-resolution copy of the neighbour's light the design first had: it needs no neighbour level cached, and it costs nothing without a light near an opening. Autotest on the stacked scene: a campfire at the head of the stairs lights their foot on the level below at 0.35, fading to 0.29 and 0.06 two and four cells off, the way that keeps clear of the pits. Without it the foot reads 0.00. A fire below now lights the level above through the stairs and pits, at most half (0.31 to 0.79). Also here: the level-change fade lengthens to 200 ms (twelve frames at least), since at 150 ms the brightness check sat at its 10% edge (11% in one run); it now reads 7%.

## 2026-09-28

Review fixes:
- A wide opening shares each light between its cells. The lights a source gives through openings add up to no more than the brightest of them, halved, so a pit isn't brighter below than its light above. A unit test on a real map with a 3x3 pit fails without the sharing.
- A light passes on only in sight of the opening on its own level: a wall or door between them stops it (the same test).
- The bake's key covers this level's own revision too, since its own air is an opening.
- Room fill counts only the level's own lights.
- The autotest's stair probe keeps six cells clear of the pits.
Declined: light down more than one level, and carried lights crossing levels; both have no caller yet. Also the per-frame cost of lamps_across without a spatial cull, which is fine at today's sizes.

## 2026-09-28

Before the PR, on main 0d8cc8c1: the level-change fade check sat at its edge, 13% on main's CI and 10-12% on some of mine. Two causes, both fixed here. The eye's clock-driven easing added to the fade, so changing level now snaps the eye to the new level and the fade alone carries the change. And the check measured the landing grab (three frames: the press's own and the grab's two) as two. It now reads the world's middle, not the panels that change with the level at once. Three runs read 4%, 4%, 5%. Full gate: 814 of 814 tests. The only reds are main's own: the crosscheck seed (lucky-harbor) and the dock-cancel family and room labels in the autotest (green-forest), which fail the same on plain main.
