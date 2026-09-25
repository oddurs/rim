---
id: cc6ac1c3-2361-44ff-8a6e-4f04e23e80f5
title: 'Autotest: the wall-seam check picks a row a pawn is standing on'
type: bug
status: done
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: none
effort: s
layer: client
area: tests
---

## What happens

The walls check (0220) samples the pixel between two wood walls on a free
row near home. The row test skips fixtures and items but not pawns, so when
the founder stands there his token and name label cover the seam and the
check fails: "no seam between joined walls ([0.46, 0.58, 0.65] vs fill ...)".

## What should happen

The check picks a row with no pawn on or next to it, so it measures the
walls, whatever the colony happens to be doing.

## Reproduction

Seed: 7 (autotest)
Mods: all shipped
Tick: the walls step, after building

Any change to the sim's trajectory can walk the founder onto the row: it
failed on #63 and #64, and passes on main only because he is elsewhere.

## 2026-09-24

The row now also needs every pawn more than 4 cells from its middle wall: a token plus a name label spans about two cells either side. Not reproducible locally: the autotest advances the sim by rendered frames, so the founder's position at the check depends on the machine; the CI screenshot (05_walls.png on #64) shows him on the row.
