---
id: 535
uid: d8a98377-5878-49a8-bd61-14deab99652e
title: 'Rock is drawn as rock: a broken outline, bevelled corners and scree'
type: feature
status: backlog
milestone: rock-face
depends_on:
- 504
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Problem

A rock mass is drawn with the walls' `mass`: rounded rectangles joined into runs. A hill reads as masonry. Each cell has its own tint (`vary = 0.12` on `thing/core:granite`), so the hill shows its grid as tiles. DESIGN.md §6g.

## Proposal

- A join style `rough`: `look.join = { group = "rock", style = "rough" }`. Joined sides stay straight and pad half a point into the neighbour, as `mass` does. An exposed side is broken into four facets, offset across the edge by −0.08 to +0.05 of a cell. An outer corner is bevelled at 0.12 to 0.34 of a cell on each side, with a two-facet bevel. Every offset is `hash2_f` of the world edge or corner, so a shared corner agrees and a cell always looks the same.
- The contour, the lit top-left line (ae5c3807) and the contact shadow follow the broken outline. The fill still covers the whole cell (§6b: the look never lies about what blocks).
- Scree: on an open floor cell beside a rough face, one to three small five-sided stones per exposed side in the rock's colour, drawn with the floor at pattern zooms.
- Tone: `vary` below 0.05 takes low-frequency value noise over world position instead of a per-cell hash. Core's rock things move to `vary = 0.03`.
- The reference is `rockRing`, `ringOutline` and `scree` in the concept artifact.

## Acceptance criteria

- [ ] Every solid terrain's thing in core and primitive uses `style = "rough"`; walls are unchanged (autotest pixel check on a wall run)
- [ ] A rock face is drawn with broken edges and bevelled corners, and two cells sharing a corner meet with no gap (autotest `rock_close`, looked at)
- [ ] The same map draws the same outline on every run: a hash of `rock_close` is stable across two autotest runs
- [ ] Render bench: whole map unchanged within noise; quarry view recorded here
- [ ] `docs/modding/looks.md` documents `style = "rough"`
