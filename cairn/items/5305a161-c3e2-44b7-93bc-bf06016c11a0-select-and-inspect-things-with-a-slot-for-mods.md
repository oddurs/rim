---
id: 5305a161-c3e2-44b7-93bc-bf06016c11a0
title: Select and inspect things, with a slot for mods
type: feature
status: backlog
milestone: stone-age
created: 2026-09-25
updated: 2026-09-25
priority: p0
api: additive
effort: m
layer: client
area: ui
---

## Why

Only pawns can be selected: `app.selected` holds whatever `pawn_under` finds (`crates/rim_client/src/main.rs:1317`), and `core:inspector` shows a pawn or nothing (`mods/core/ui/hud.luau:167`). A workbench's bills, a tool's wear and a tree's "needs a chopping tool" all need somewhere to live on screen. That place is the selected thing.

## What

- **Clicking a thing selects it.** A click picks a pawn first, then the fixture or floor in the cell, then the item stack there. `view.selected()` returns the entity either way.
- **`view.thing(id)`** returns `{ id, def, label, count, hp, max_hp, made_of, held_by, designated, why }`. `why` is the reason its designated work isn't happening ("needs a chopping tool"), or nil.
- **`core:inspector` shows things too**, with a slot `core:inspector.thing` that mods fill with `ui.extend`. That slot is where the crafting plugin puts a station's bills.
- **The UI API gains `view.thing`.** UI API 0.3.

## Acceptance criteria

- [ ] Clicking a building, plant, rock or item stack selects it
- [ ] The inspector shows its label, count, hp and material
- [ ] A mod extends the thing inspector through `core:inspector.thing`
- [ ] A designated thing whose work is blocked says why
