---
id: 04fa9b7d-50eb-4f2b-9267-4f53f4f5218c
title: Pits and stairs drawn as a plan draws openings
type: feature
status: backlog
milestone: rock-face
depends_on:
- 5689930d-2bd1-4838-b403-a72bc61c31e9
- acd85584-7f3d-4348-8d56-5242a0bdb620
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: render
---

## Problem

Digging down makes air cells (acd85584), and the view (5689930d) draws the level below through them. Nothing says a hole is a hole: from above, a pit reads as a darker patch of floor. DESIGN.md §6g, "Holes".

## Proposal

- An air region draws the level below through it, dimmed by §6e's depth tint, then an X across each rectangular opening (per cell if it isn't rectangular) at 35% chalk, and a heavy ink outline.
- The mass's light turned inside out: a soft shadow inside the near rim (top and left) and a lit hairline inside the far rim (bottom and right).
- Stairs keep §6c's symbol: treads, a break line and an arrow marked DN from above and UP from below.

## Acceptance criteria

- [ ] A 2 × 3 pit shows the level below, an X and the rim light (autotest shot)
- [ ] A stair pair shows DN on the upper level and UP on the lower (autotest by node or pixel)
- [ ] Render bench stacked scene (5689930d) within budget with the openings drawn
