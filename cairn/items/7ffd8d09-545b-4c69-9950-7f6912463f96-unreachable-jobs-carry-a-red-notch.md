---
id: 7ffd8d09-545b-4c69-9950-7f6912463f96
title: Unreachable jobs carry a red notch
type: feature
status: planned
milestone: chalkline
depends_on:
- a207eded-13e8-468d-9b4a-1255cbb38d02
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: s
layer: client
area: ui
---

## Why

A marked tree across a river waits forever, and nothing on the map says so. DESIGN.md §6f puts the unreachable mark at a footprint's bottom-left.

## What

- A `threat` triangle, about 0.3 of a cell, at the footprint's bottom-left on a keyline, on every job the spike's answer names as unreachable.
- Hovering it shows the chip "No one can reach this".

## Acceptance criteria

- [ ] Autotest: a tree marked to chop on an island with no path shows a notch mark within one planner pass; after a path is built, the notch goes away
- [ ] Screenshot `chalk-unreachable`
