---
id: 6ce26632-93be-424e-935d-9312a4afbdc5
title: 'The UI draws a thing''s look: the item token'
type: feature
status: backlog
milestone: crafting
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Why

The UI can't draw an item: stockpile toggles and the dock show a coloured square (`toolbar.luau` swatch). DESIGN.md §4f wants one item token for every item on every screen, drawn from the def's own look so a modded item needs no art.

## What

- The UI paints a def's look (its primitives, tinted by material) into a box. Sprite layers draw from the world atlas.
- `kit.item{ thing, made_of, count, size = "s"|"m"|"l", hp, state }` with the count, full-stack notch, condition bar, and incoming, leaving and empty states.
- `kit.grid` cells can carry a look, so a contents grid stays one node.
- Counts: exact to 999, then 1.0k and 12k on a token; lists show exact numbers.

## Acceptance criteria

- [ ] `kit.item` renders every core and primitive item in the F12 kit gallery (shot test)
- [ ] A grid of 200 tokens stays under the UI budget (measured, noted here)
- [ ] ui_api bumped, types/ui.d.luau and docs regenerated
