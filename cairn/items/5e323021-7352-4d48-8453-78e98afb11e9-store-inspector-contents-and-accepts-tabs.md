---
id: 5e323021-7352-4d48-8453-78e98afb11e9
title: 'Store inspector: Contents and Accepts tabs'
type: feature
status: backlog
milestone: crafting
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
- 6ce26632-93be-424e-935d-9312a4afbdc5
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: core
area: ui
---

## Why

Nothing shows what a zone or a container holds; the zones sheet is a wall of toggles (`zones.luau`). DESIGN.md §4f, Showing contents.

## What

- `view.store(id)`: everything the tab paints in one read (contents, level, room, incoming, filter summary).
- A Contents tab: slots for containers (empty ones dashed), totals by thing and material for zones; grouped by category; sort orders from a `storage.sorter` registry (Category, Count, Value, Name).
- An Accepts tab: the category tree with tri-state boxes, material chips, a condition range, presets, and copy and paste.
- Actions with keys: level up and down, copy, paste, empty. All Commands.
- Zones become selectable stores and use the same tabs.

## Acceptance criteria

- [ ] Selecting a zone or a container shows its contents and accepts tabs (UI tree test)
- [ ] Changing the filter in the tab changes where hauls go (test)
- [ ] A mod can register a sort order without touching core
