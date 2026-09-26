---
id: 65588ce1-0625-4cd2-bb31-e2425f781cb2
title: Names and speech bubbles lag a frame behind their pawn
type: bug
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: client
area: ui
pillar:
- plugin-first
---

## What happens

Names, the sleep marker and speech bubbles drift off their pawn, worst while zooming or panning and visible on any walking pawn. `frame()` places the anchored layer from the camera and pawn positions as they are, then applies pan and zoom and steps the sim; `render()` draws the world with the new camera and pawns and the UI with the old placement.

## Fix

Remember each anchored label's anchor and the screen point it was placed at. Before the UI draws, shift each label's draw commands by how far its anchor moved. No relayout, no Luau: a translation of the anchored layer's commands.

## Acceptance criteria

- [x] A label placed, then the camera zoomed and the pawn stepped, draws at the pawn's new screen position
- [x] The cost is a translation of the anchored layer's draw commands, nothing more
