---
id: 23eb3f04-6e1f-4b88-8e93-3309831e31bc
title: The spoil pass and wealth walk every thing every 250 ticks
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: sim
---

## What

- `systems::spoil` (systems.rs ~19) queries every `Thing` (with optional `Spoiling`/`Contained`) to work out the quarter of spoiling stacks it handles this pass. Walls, rocks, plants and furniture are all walked and skipped.
- `systems::wealth` (~308) walks and sorts every non-blueprint thing every plant pass, to sum a number that changes only when things come, go or change count.

## Why it matters

Each is O(things) (wealth O(things log things)) every 250 ticks, whatever changed. It's a periodic spike on the tick that grows with the world.

## Direction

- Spoil: keep the spoiling stacks in an index (by def with a `spoil`, kept by the stock's choke points), and walk only this pass's quarter.
- Wealth: keep it as a ledger updated where the stock and built things change (as `Stock` is), checked against a recount in debug builds.

## Acceptance

- [ ] Neither walks every thing on its pass
- [ ] The same values as today (debug check against a recount)
