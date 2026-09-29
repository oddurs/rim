---
id: 494
uid: 78393567-4993-47b3-924f-d4b58da37271
title: 'Light in the mine: torch-lit adits, sunlit pits, a hill''s long shadow'
type: feature
status: backlog
milestone: rock-face
depends_on:
- 443
- 450
- 329
- 365
- 379
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: render
---

## Problem

Lighting (§6e) and Depth each have their own checks, and none of them look at a mine. A torch in a one-wide adit, a pit at noon and a hill at dusk are the pictures Rock face is for, and they're where two milestones' work meets. DESIGN.md §6g, §6e.

## Proposal

A set of autotest scenes and checks, plus any tuning they show is needed. No new passes.

- A torch halfway down a 1-wide, 8-deep adit at night.
- A 2 × 3 pit and a closed cellar beside it at noon, seen from the surface and from −1.
- The quarry hill beside a lone wall at dusk.

## Acceptance criteria

- [ ] The adit is lit from the torch to its end and dark past the bend, with soft shadow edges at the face (shot, readback)
- [ ] The pit's floor on −1 is sunlit inside the sun's cone, and the cellar beside it is dark (readback)
- [ ] The hill's dusk shadow is longer than the wall's (readback)
- [ ] Shots pair with the concept artifact's Noon, Dusk and level −1 figures, looked at, with differences noted here
