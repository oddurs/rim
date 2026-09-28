---
id: 78d59e24-0b8e-45fc-899c-af1e14ea3308
title: A pawn caught in deep water can't get out
type: bug
status: done
milestone: depth
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
effort: s
layer: engine
area: ai
---

## Why

Deep water is no footing (695ef115), and escaping rising water searches from the pawn's region. A pawn whose own cell is already past swimming depth, such as a miner who breaks into a lake from below in a small tunnel, has no region, finds no way out, and drowns where they stand. Found benching 202b16c4: the render bench's stacked room on seed 1 lies under a lake, and the colonists it puts there die.

## What

- A pawn standing in deep water may still step out of it: the escape search starts from the pawn's cell whatever its depth, and a step out of deep water onto footing is allowed, as a scramble.
- Whether that's everyone or only the next few cells is the call to make here.

## Acceptance criteria

- [x] A miner who breaches a lake at the end of a tunnel gets out alive (scene test)

## 2026-09-28

Water's check on pawns runs every tick now (the map still gets its cost every 60): a pawn in rising water past wading gets a route out found by a breadth-first walk through water of any depth and up stairs, to the nearest cell with footing under wading depth, and walks it as Flee, which may step through deep water. Drowning still costs health while they're under. The scene test's miner at the end of a 12-cell tunnel, with the lake over them and the tunnel full in a few ticks, gets out; without the scramble step they drown.
