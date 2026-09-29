---
id: 447
uid: 11080b20-768a-4930-badb-0474c94c86b9
title: Zones in violet; a selected zone turns chalk; paint and erase preview
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 533
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

`crate::ZONE` (main.rs) and the blueprint colour in `draw.rs` are nearly the same sky blue, so a stockpile under a plan reads as more plan. A selected stockpile doesn't look selected, and painting or erasing a zone previews the whole rectangle. DESIGN.md §6f.

## What

- `zones()` takes `zone` and `zone_fill` from the palette, and `crate::ZONE` goes.
- The selected zone (`app.selected_zone`) gets a chalk perimeter, 1.5 px on a keyline, and its wash rises to 20%. A hovered zone's perimeter goes to 1.5 px in `zone`.
- Stockpile drag: cells to be added take a 24% wash and the perimeter redraws around the union, with the chip "Stockpile · +N". Clear-zone drag: cells to be removed are hatched in chalk at 30% with a dashed edge, and the chip reads "Remove · N cells".
- Coordinate with the stores overlay (f22f1a62, PR #196), which also draws over zones.

## Acceptance criteria

- [x] `grep -rn "ZONE" crates/rim_client/src` finds no colour constant
- [x] Autotest: selecting a stockpile gives a chalk perimeter mark; screenshot `chalk-zone`
- [x] Autotest: a stockpile drag of 2×4 next to a zone reports "+8" and after release the zone has 8 more cells
- [x] Autotest: a clear-zone drag's hatched cells are the cells removed

## 2026-09-27

This item adds the theme token zone and zone_fill (and their doc rows) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

## 2026-09-27

Stacked on hover (bd7a158e), which lifts a hovered zone's edge; rebases as the stack merges.

## 2026-09-27

The sim answers the preview: Zones::painted (read-only) lists the cells a paint or clear changes, and paint itself uses it, with a test that the two agree. crate::ZONE is gone; the stockpile tool's dock colour follows the theme's zone token. The stores overlay (#196) washes with the zone token too.
