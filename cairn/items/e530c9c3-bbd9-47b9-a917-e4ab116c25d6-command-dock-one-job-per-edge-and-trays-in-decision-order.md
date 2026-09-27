---
id: e530c9c3-bbd9-47b9-a917-e4ab116c25d6
title: 'Command dock: one job per edge, and trays in decision order'
type: feature
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: additive
effort: l
layer: core
area: ui
---

## Why

The bottom bar grew one full-width row per decision: materials, then groups, then things, then the categories that open them, with the screens on the end. Every row looks the same, they read in the reverse of the order a player decides, a thing's cost and stats have nowhere to live, and a group is one row of chips that forty mod buildings would run off (design: the Command Dock artifact).

## What

- Each edge has one job: the top bar holds status and the screens (as tabs); the bottom holds verbs only: Select, Orders, Build, Zones.
- A verb opens a tray above the dock, `kit.tray`, in decision order. Build: a group rail with counts, things as tiles (swatch, name, cost, number key), and a blueprint card (what it is, cost, work, hp, and the material picker) for the hovered or picked thing.
- Picking a thing folds the tray to a pill that says what the next click places, in what, and what it costs.
- Orders: the verbs with how many are marked, and a card for the hovered one. Zones: stockpile and clear, and the colony's stockpiles.
- Keys: 1–9 pick the nth tile in the open tray (speed keys otherwise), [ and ] step groups, / finds by name, Escape backs out one step.
- An `on_hover` node handler so the card follows the pointer, and the materials for any buildable, not just the active tool.

## Acceptance criteria

- [x] Build, Orders and Zones open a tray; the tray reads group, thing, then material and cost
- [x] Picking a thing folds the tray to the placing pill; Escape brings the tray back
- [x] The screens are tabs in the top bar, and the dock holds verbs only
- [x] Finding a thing by name works across groups
- [x] Only the open tray is built, and the UI budget holds with every mod loaded

## 2026-09-26

Built to the approved Command Dock artifact with three changes: find is on / (typing straight into a tray would steal Q/B/Z and Escape from the bindings); the pill keeps the material chips rather than a Tab key, since Tab is next-colonist; the tray has a fixed height, because the card changing under the pointer grew the tray and slid the tiles out from under the click (the autotest caught it).

## 2026-09-27

Merged as #141.
