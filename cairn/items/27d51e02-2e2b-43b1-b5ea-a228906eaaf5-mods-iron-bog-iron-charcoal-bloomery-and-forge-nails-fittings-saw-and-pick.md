---
id: 27d51e02-2e2b-43b1-b5ea-a228906eaaf5
title: 'mods/iron: bog iron, charcoal, bloomery and forge, nails, fittings, saw and pick'
type: content
status: backlog
milestone: crafting
depends_on:
- 4f3e5d8d-a566-4520-802f-f03fd6d81ab5
- dce75339-9120-495d-9b3b-b4012251adac
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: l
layer: plugin
area: building
---

## Why

Iron opens the upper rungs: the saw for fast planks, nails for plank walls and crates, fittings for shelves, and the pick (DESIGN.md §4f, §6d).

## What

- Bog iron: dug from the marsh with a `digging` tool, like clay banks. Deep ore at −3 comes with the depth milestone (§6d) and uses the same smelting.
- Charcoal: wood burned at a clamp.
- Bloomery: bog iron and charcoal to iron bars. Forge: bars to nails, fittings, a saw (`sawing`) and a pick (`pounding`, `mining`).
- Patches timber's plank wall, crate, shelf and rack to add nails or fittings; skipped when timber is absent.

## Acceptance criteria

- [ ] From a marsh, a colony can make iron bars, nails and a saw (mod test)
- [ ] With iron installed, a crate needs nails; without it, it doesn't
- [ ] rim check --strict passes with every combination of timber and iron
