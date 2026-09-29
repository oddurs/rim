---
id: 450
uid: 1bb64fd0-dfaf-47d8-b4cc-f51a573e1e48
title: Rock height in the occluder texture
type: feature
status: backlog
milestone: rock-face
depends_on:
- 536
- 421
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: render
---

## Problem

The occluder texture (e5d8b445, PR #201) gives every blocking thing one storey. A hill then casts the same shadow as a wall, and the sun reaches the back of a deep adit. DESIGN.md §6g, "Height".

## Proposal

- In `occluders.rs`, a solid terrain cell's height is `1 + 0.4 · min(depth − 1, 3)` storeys, from the relief item's depth, in the 6-bit height of R. No format change.
- Walls, doors and windows stay at 1, so a wall on a hill never stands above it.
- The repack already keys on `terrain_rev`; mining a cell repacks its chunk and the chunks its depth reaches.

## Acceptance criteria

- [ ] At dusk in the quarry view, a hill's shadow is at least twice as long as a lone wall's beside it (readback on the sky shadow buffer)
- [ ] At noon, the floor 4 cells into a 1-wide adit gets no direct sun (readback)
- [ ] Mining one cell repacks no chunk further than 4 cells away (test)
