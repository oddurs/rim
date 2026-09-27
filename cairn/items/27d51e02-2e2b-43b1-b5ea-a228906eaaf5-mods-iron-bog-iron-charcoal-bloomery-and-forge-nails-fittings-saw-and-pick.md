---
id: 27d51e02-2e2b-43b1-b5ea-a228906eaaf5
title: 'mods/iron: bog iron, charcoal, bloomery and forge, nails, fittings, saw and pick'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 4f3e5d8d-a566-4520-802f-f03fd6d81ab5
- dce75339-9120-495d-9b3b-b4012251adac
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] From a marsh, a colony can make iron bars, nails and a saw (mod test)
- [x] With iron installed, a crate needs nails; without it, it doesn't
- [x] rim check --strict passes with every combination of timber and iron

## 2026-09-26

Iron depends on timber: its saw pit recipe outputs timber:planks, and a recipe naming a missing thing stops the game from loading, so iron can't be optional about timber. Timber still plays alone (planks hewn with an axe). Bog iron beds spawn on marsh at 2% and are dug with a digging tool like clay banks (8-day regrow). Chain: charcoal clamp (2 logs -> 2 charcoal), bloomery (3 bog iron + 2 charcoal -> 1 bar), forge (1 bar -> 20 nails or 4 fittings; 2 bars + a log -> saw or pick, needs pounding). The pick carries depth's 'mining' tag already. Patches add nails to timber's plank wall, crate and tool rack and fittings to its shelf; ids in patches are qualified, since a bare id resolves in the target mod. Test: from bog iron (spawned, as the bed's dig is covered by the harvest's requires) to nails and a saw, and the patched costs; rim check --strict covers timber alone and timber with iron.

## 2026-09-27

Merged in #219; closed in the Proving ground planning PR's cairn cleanup (2026-09-27), since the item was left at doing/review after its merge.
