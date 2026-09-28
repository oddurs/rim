---
id: 0927f0af-ae81-4e4c-91bc-d08a2ca25055
title: 'Tailoring: make apparel'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: core
area: building
---

## Why

Clothe the naked.

## Acceptance criteria

- [x] Leather from butchering

## 2026-09-27

Leather from butchering is in primitive/defs/hides.toml. Deer give 3 hides, wolves 2, hares 1, appended to butcher like bones. A hide is scraped into one leather at a crafting spot with a cutting tool: 400 work, recipe primitive:leather, input by tag 'hide' so other mods' hides count. Clothes are split off to d0362b0c: nothing in the engine can be worn yet (no equip slot, no wear job, no insulation), and garments that can't be worn would be wealth and nothing more. Test: tests/leather.rs, where a deer's death drops 3 hides and a founder scrapes one into leather with a flint flake.

People (5d09b04e): clothes (d0362b0c) draw as worn layers at the body's torso socket (535a1fb9); see apparel (41006a46).
