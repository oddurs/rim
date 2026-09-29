---
id: 323
uid: 28a49214-69b5-43e3-b236-9619c9477694
title: Moving the pointer over the map rebuilds the whole UI at every cell
type: perf
status: done
milestone: interface
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Budget

## Measurement (before)

## Approach

## Acceptance criteria

- [x] Benchmark shows the budget is met

The hovered cell and pawn were part of the client hash every tree is
built against, so the pointer crossing a cell forced every mount and
window to rebuild in Luau and the whole UI to lay out again: 3.8 ms of
CPU a frame against 0.7 ms holding still, on the core UI with 30
colonists. The pointer crosses a cell at nearly every step over the map.

## Acceptance criteria

- [x] Only trees that read the hover rebuild when it changes
- [x] A tree that reads only who is hovered doesn't rebuild when the cell changes
- [x] A test fails if a hover change rebuilds a tree that didn't read it

## 2026-09-26

UI CPU per frame on the core UI, 30 colonists, pointer crossing a cell every frame (getrusage, release): 3.8 ms before, 1.9 ms after; holding still 0.7 ms. The rest is relaying out the whole docked shell, filed as its own item.
