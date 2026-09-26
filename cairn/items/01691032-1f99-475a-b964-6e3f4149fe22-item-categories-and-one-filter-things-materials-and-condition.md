---
id: 01691032-1f99-475a-b964-6e3f4149fe22
title: 'Item categories and one filter: things, materials and condition'
type: feature
status: backlog
milestone: crafting
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: sim
---

## Why

Stores, stockpile zones and bills all ask one question: may this stack go here? Zones answer it with a sorted list of def ids (`zone.rs`, `Zone.allows`), which can't say "no bone" or "50% condition or better", and bills can't answer it at all (ca22f222). DESIGN.md §4f rules one filter for all three.

## What

- `[[item_category]]` defs: `id`, `label`, `parent`, `order`, and membership by `tags` or `things`. Core ships the shared tree (food, materials and its children, tools, other). An item no category claims falls under Other, so nothing is unfilterable.
- The `bulky` tag in core's vocabulary: wood, stone and raw meat carry it; containers will refuse it.
- A compiled `Filter { things, made_of, hp_pct }`: bitsets over item and material def ids plus a condition range, interned so stores that share one share a `FilterId`. `takes(&Lot)` is two bit reads and a compare.
- Zones use it: `Zone.allows` becomes a filter, the zone commands keep working, and the save stores it by qualified id (remapped on load as today).
- Scripts can read the category tree (`rim.defs("item_category")`), and the UI can read it for the Accepts tab.

## Acceptance criteria

- [ ] Core declares categories and `bulky`; every item def lands in exactly the categories it should (test)
- [ ] A zone can refuse a material (bone) while taking the thing's other materials (test)
- [ ] A zone can refuse stacks below a condition (test)
- [ ] Zone filters survive save and load, and a missing def drops out with a note
- [ ] Determinism test passes
