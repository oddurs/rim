---
id: 212
title: 'Stuff: one buildable, many materials'
type: feature
status: backlog
milestone: building
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: building
---

## Why

A "wooden wall" and a "stone wall" are two unrelated defs that happen to
look alike. Six materials times ten buildables is sixty defs, and a mod
adding one material has to write ten of them.

## What

- `build.stuff = { category = "structural", count = 25 }` replaces a fixed
  `cost` for buildables that can be made of anything. `cost` stays for
  things with a fixed recipe.
- `[[thing]]` gains `stuff = { categories = [...] }` on items, so wood
  declares itself structural rather than walls listing every wood.
- The material is chosen when the blueprint is placed and travels on the
  `Command`, so replays and lockstep still work.
- `MadeOf(DefId)` on the blueprint and the finished thing.
- Delivery and refund resolve against the chosen material, not a fixed
  cost list.

## Acceptance criteria

- [ ] One `wall` def buildable in wood or stone
- [ ] A mod adding an item with a `stuff` category makes it buildable with
      no other change
- [ ] Cancel refunds the material actually delivered
- [ ] Determinism holds: the choice is in the command log
