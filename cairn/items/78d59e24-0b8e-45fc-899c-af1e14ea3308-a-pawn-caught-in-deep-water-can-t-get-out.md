---
id: 78d59e24-0b8e-45fc-899c-af1e14ea3308
title: A pawn caught in deep water can't get out
type: bug
status: backlog
milestone: depth
created: 2026-09-28
updated: 2026-09-28
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

- [ ] A miner who breaches a lake at the end of a tunnel gets out alive (scene test)
