---
id: f22f1a62-f8f7-48ef-bfe5-75f986b081c4
title: Stores sheet, storage overlay and hover card
type: feature
status: backlog
milestone: crafting
depends_on:
- 5e323021-7352-4d48-8453-78e98afb11e9
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: m
layer: core
area: ui
---

## Why

The inspector shows one store; the player also needs the whole colony's stock and a map view of where things go. DESIGN.md §4f.

## What

- `view.stock()` at 4 Hz, from the ledger.
- A Stores sheet: things grouped by category with stored, loose and store counts and a 7-day trend.
- A hover card on stores: name, level, capacity, the first tokens in the current order.
- A storage overlay in the `O` cycle: stores washed by level with their fill; a selected loose stack shows where it will go, or why it waits.

## Acceptance criteria

- [ ] The sheet matches `rim.stock` for every thing (test)
- [ ] The hover card and overlay stay within the UI budget with 200 stores
