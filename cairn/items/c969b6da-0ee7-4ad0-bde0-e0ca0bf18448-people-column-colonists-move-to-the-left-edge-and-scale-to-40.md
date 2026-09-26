---
id: c969b6da-0ee7-4ad0-bde0-e0ca0bf18448
title: 'People column: colonists move to the left edge and scale to 40'
type: feature
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Why

Colonists sit in the top bar, one button each, capped at 24. The top bar also holds the clock, the status and every mod's extension, so each new colonist squeezes the rest. A colony of 20 does not fit at 1280 px.

## What

- Colonists leave `core:topbar` for a column on the left edge, in a `scroll` node.
- Density follows the count: cards up to 8, compact rows from 9, grouped (drafted, working, idle, down) from 20.
- Button ids stay `core:colonists.<name>`, so tests and mods keep working.
- An extension point `core:people.row` lets a mod add a badge to each row.
- Rows are built from `view.colonists()`; nothing is built for rows scrolled out of view.

## Acceptance criteria

- [ ] 40 colonists fit at 1280×720 without pushing anything off screen
- [ ] The top bar no longer grows with the colony
- [ ] `core:colonists.<name>` ids still select their pawn
- [ ] The UI budget stays under 1 ms with 40 colonists
