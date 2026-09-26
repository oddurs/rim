---
id: 4675019b-c017-47e7-b148-b12e4be58bda
title: 'Stone tools: knapping, hafting, and the gates they open'
type: content
status: doing
milestone: stone-age
assignee: Oddur Sigurdsson
claimed: 2026-09-25
depends_on:
- 049e2f73-0d64-48ff-bf2e-21264f765d35
- 3ccab46f-32d5-4248-a23d-88cdc8b3ec70
created: 2026-09-25
updated: 2026-09-25
priority: p0
api: none
effort: l
layer: plugin
area: building
---

## Why

Split from e3846c47. The first tier gave you hands and a campfire. This one gives you edges: a hammerstone, flint flakes and hand axes knapped at a crafting spot, cordage, and the hafted tools that make felling and quarrying worth doing. Only then does core's work start to need them.

## What

- **Materials:**
  - flint becomes lithic stuff (`tool_speed` 1.0, hp 1.0), so what's knapped from it is made of it;
  - stones carry a tool block: a hammerstone, `pounding` at 0.4, which never wears out;
  - cordage is an item, crafted from 3 fibre.
- **Recipes** at `crafting:hand` (the free crafting spot), from `mods/crafting`:

  | Tool | Inputs | Needs | Tool tags | Speed | Wear |
  |---|---|---|---|---|---|
  | flint flake | 1 knappable | `pounding` | `cutting` | 0.6 | 2 |
  | hand axe | 2 knappable | `pounding` | `chopping`, `cutting` | 0.6 | 3 |
  | cordage | 3 fibre | none | | | |
  | hafted axe | 2 knappable, 1 branch, 1 cordage | `cutting` | `chopping` | 1.0 | 2 |
  | stone maul | 3 stones, 1 branch, 1 cordage | `cutting` | `pounding` | 1.0 | 2 |
  | digging stick | 1 branch | `cutting` | `digging` | 1.0 | 1 |

- **Gates**, as patches on core: oak chop requires `chopping`, granite mine requires `pounding`.
- **Nudges** from a script: the first flint gathered, and the first tool made.

## Acceptance criteria

- [x] A flint flake and a hand axe are made at a free crafting spot
- [x] Tool quality comes from material, not duplicate defs
- [x] Chopping and mining are gated behind the tools, and say why when blocked
- [x] A `rim test` scene goes from bare hands to a felled tree
- [x] Core alone still plays with the plugin removed

## 2026-09-25

Started while 3ccab46f (#110) is in CI and 049e2f73's close rides with it. This is content on top of both, and lands after them.

## 2026-09-25

Built as specified, except for four things:
- The hammerstone is its own item, shaped from one stone at the spot (60 work), not the stones item with a tool block. The loader forces tools to stack_limit 1, which would split every stack of stones.
- The digging stick takes no material (stuff = false). Made of branches it would be a 20 hp 'branches digging stick'.
- The flake's label is 'flake', so the material names it: 'flint flake', later 'bone flake'.
- Cordage needs no tool, per the table; the tool.toml header said otherwise, now fixed.

## 2026-09-25

Review fixes that reach beyond primitive:
- crafting: a bill whose tool no one has now says 'needs a pounding tool' and lets the bills below run. Before, a hand-axe bill above the hammerstone bill held the spot forever. New rim.has_tool(tags).
- crafting: emits crafting:made for each output, which the nudges hear; a new tool is usually in hand before any poll could see it.
- Tests: the all-mods ones that chop (snapshot, entity_ids, determinism, savetext, the bench example) arm their colonists with common::arm. savefile's replay tests don't, since items placed outside a Command can't replay.
