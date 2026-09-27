---
id: 5e323021-7352-4d48-8453-78e98afb11e9
title: 'Store inspector: Contents and Accepts tabs'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
- 6ce26632-93be-424e-935d-9312a4afbdc5
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] Selecting a zone or a container shows its contents and accepts tabs (UI tree test)
- [x] Changing the filter in the tab changes where hauls go (test)
- [x] A mod can register a sort order without touching core

## 2026-09-26

Built in mods/core/ui/storage.luau (returns the storage.sorter registry; core registers Category, Count, Value, Name). Zones are selectable: the client keeps selected_zone (a plain click on an empty zone cell, or the Zones tray row via act.select_zone), and the inspector's sel is { kind = 'zone', ... }. view.store(ref) and view.item_categories() give the tabs everything in one read; act.store_filter(ref, edit) and act.store_level(ref, level) take a zone id, { zone = id } or { thing = id }, and resolve ids in the client like zone_allow. Keys: comma and full stop step the level, Ctrl+C and Ctrl+V copy and paste a filter. [ and ] were the plan, but they're the dock's tray keys, and C is Centre, and bindings are last-registered-wins. Paste sends the copy as edits (nothing, then each thing, materials, condition), so it replays. Condition is four presets (any, 25/50/75%+), not a slider, so a drag doesn't send a command per frame. 'Empty the store' left out: it needs an eject command the sim doesn't have yet.
