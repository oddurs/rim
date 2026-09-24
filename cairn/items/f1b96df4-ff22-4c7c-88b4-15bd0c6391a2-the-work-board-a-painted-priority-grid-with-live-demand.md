---
id: f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
title: 'The Work Board: a painted priority grid with live demand'
type: feature
status: backlog
milestone: colony
depends_on:
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
- fcb28d0c-35a3-4fca-977c-297d30a8e575
created: 2026-09-24
updated: 2026-09-24
priority: p0
api: additive
pillar:
- growth
effort: l
layer: core
area: ui
---

## Why

The grid is the colony's main control, and RimWorld's is a spreadsheet you type into. Ours should be fast to paint, show the rules it runs on, and show where work is piling up (DESIGN.md §4d).

## What

- A panel in core's UI mod, patchable and replaceable by mods, reading priorities through `view` and changing them through commands.
- Paint: drag across cells to set a value, scroll to nudge, number keys on hover, shift-scroll for a column.
- Cells show priority by brightness, skill as a bar, passion as a flame, and `base→effective` when a rule moves them.
- Column headers: jobs waiting, backlog trend, coverage; a warning when work waits and nobody is on it at a high priority. Drag columns to set the tie-break.
- Rows: current job, a 24-hour schedule strip, time idle.
- A ranked view of one colonist: drag work types into order, and it writes the numbers.

## Acceptance criteria

- [ ] Painting 30 colonists by 12 work types takes one drag per stroke, and every change is a `Command`
- [ ] Demand headers match the pools' counts
- [ ] The ranked view and the grid stay in sync both ways
- [ ] A mod adds a work type and it appears as a column without UI changes
- [ ] Screenshots reviewed at 4 and 9 levels
