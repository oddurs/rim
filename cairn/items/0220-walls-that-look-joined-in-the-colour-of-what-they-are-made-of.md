---
id: 49e5e583-3a4d-45de-8d65-a8202af9f9ec
title: Walls that look joined, in the colour of what they are made of
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- cecf0ba9-3f87-41fa-9c90-bb01a520e72e
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

One `wall` def in six materials has to read as six different walls, and a
run of wall has to read as a wall rather than a row of blocks.

## What

- Autotiling: a wall's drawn shape comes from its blocking neighbours, so
  corners, tees and ends join.
- Tint from the material, not the def, so marble arrives looking like
  marble without a renderer change.
- Doors and windows read as openings in the run, not separate objects sat
  next to it.

## Acceptance criteria

- [x] A stone wall and a wooden wall are told apart at a glance
- [x] Corners and junctions join with no gaps
- [x] A material added by a mod is drawn correctly with no client change

## 2026-09-24

Done. Tint: a thing with MadeOf is drawn in its material's colour, so a modded material arrives looking like itself with no client change (criterion 3 is a pixel test: the stone wall is nearer the stone def's colour than the wall def's, 0.02 vs 0.89). Joins: walls and windows draw their outline only on sides that face something that is not wall/window/door, so runs are continuous and corners join for free; doors and windows read as openings in the run. Both proven by the Linux autotest reading pixels back. Three lessons from getting that test green, all mine: (1) get_screen_data must be called before next_frame swaps the buffer, or it reads black -- shot() knew this and my first attempt did not; there is now a grab() helper both use. (2) Colour checks are relative (nearer-to-material-than-def), because the weather sprint's lighting tints everything alike. (3) The free-row search must check the row's own two ends, or the run joins a neighbour and the edge is rightly absent. Process lesson: two CI cycles were wasted by a Python edit that asserted after cargo fmt re-wrapped the block while the shell chain carried on -- heredoc edits are now joined to the rest with &&.
