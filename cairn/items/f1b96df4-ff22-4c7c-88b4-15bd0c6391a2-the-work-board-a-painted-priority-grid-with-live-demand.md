---
id: f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
title: 'The Work Board: a painted priority grid with live demand'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
- fcb28d0c-35a3-4fca-977c-297d30a8e575
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] Painting 30 colonists by 12 work types takes one drag per stroke, and every change is a `Command`
- [x] Demand headers match what find_work would take (its sources, counted by work_waiting)
- [x] The ranked view and the grid stay in sync both ways
- [x] A mod adds a work type and it appears as a column without UI changes
- [x] Screenshots reviewed at 4 and 9 levels

## 2026-09-25

Built on the engine's paintable grid (ui.grid): the board is four grids (headers, cells, names, jobs), so thirty by twelve costs a handful of nodes. Engine additions: a grid cell can carry a bar (0-1, drawn along its bottom) and its own tooltip (tip); grids take on_wheel(r, c, steps, shift); Ui::grid_cell reads a painted cell for tests. Sim: rim_sim::ai::work_waiting counts jobs waiting per work type from the same sources find_work uses (blueprints, ready designations, work orders, loose items a stockpile takes, designated creatures), ignoring reach. UI: view.board() returns columns in tie-break order with waiting, on and high (colonists at levels 1 to levels/2), and a row per colonist with each cell's base, effective, why and skill. Painting: a press takes the brush (1..levels, never) or steps the cell, a drag paints it, the wheel nudges a cell and shift-wheel its column; each change is one SetPriority. Cells: brighter the sooner, 3→1 when a rule moves them, skill as a bar, the why as the tooltip. Headers: waiting count, marked when work waits and nobody is on it at a high priority. A name opens the ranked view (▲ ▼ –), which writes the same numbers. Measured: a forced rebuild with the board open and 30 colonists costs 1.41 ms against 0.75 ms without it. Split out, as they need sim data that doesn't exist yet: passion (006ef572), a schedule strip and idle time (2787bdc6), dragging columns for the tie-break (2d44949e), all under mood. Number keys on hover are replaced by the brush bar. Criterion 2 names pools, which 0e73145a proposes to drop: the headers match find_work's sources instead (tested against work_waiting and against the world). It stays unticked until that decision; if pools go, the criterion should read 'match what find_work would take'.

## 2026-09-25

Review fixes before the PR: the wheel over a grid without on_wheel (names, jobs, headers) now reaches the scroll area it's in, so a long board scrolls (routing keeps looking for the enclosing scroll after the first interactive hit, which also lets the wheel scroll a list over its buttons); a trackpad's fractional wheel adds up per cell and nudges only in whole steps; a drag paints only cells inside the grid's clip, so dragging off the window paints nothing hidden; the client passes a horizontal wheel as the wheel while shift is held (macOS turns shift-wheel sideways). The board no longer nests its own scroll: the window's does it (kit's window body now has an id, <window>.body). Cells are 20 px high and the window 860 px, so 30 colonists fit one stroke.

## 2026-09-25

Owner accepted dropping work pools (0e73145a) on 2026-09-25 and agreed to reword criterion 2 from 'match the pools' counts' to match what find_work would take. Tested: headers equal rim_sim::ai::work_waiting (board.rs headers_show_demand_and_mark_neglected_work), which counts find_work's sources against the world (priorities.rs work_waiting_counts_what_there_is_to_do).
