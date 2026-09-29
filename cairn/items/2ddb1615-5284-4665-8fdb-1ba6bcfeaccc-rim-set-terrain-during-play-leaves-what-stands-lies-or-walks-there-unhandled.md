---
id: 2ddb1615-5284-4665-8fdb-1ba6bcfeaccc
title: rim.set_terrain during play leaves what stands, lies or walks there unhandled
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: scripting
---

## What

`rim.set_terrain` (script.rs ~1230) is documented as "what a level generator uses", but any hook can call it during play. It calls `Map::set_terrain`, and nothing else:
- a stack on a cell turned to air doesn't fall (`World::fall` isn't asked);
- a pawn on a cell turned to solid rock or deep water stays inside it;
- a fixture (a wall, a plant) keeps standing on terrain it can't stand on;
- rock set under a fixture gets no rock thing.

## Reproduce

Verified by reading. A hook sets air under a stack of wood: the stack stays on the item layer over air. No shipped mod calls `set_terrain`.

## Direction

Either refuse it outside level generation (a script error, as `on_generate_level` does for its own window), or run what a terrain change does in play: `fall` for air, and pawns and things moved off solid terrain.

## Acceptance

- [ ] A terrain change from a script can't leave a thing or pawn on a cell it can't be on
- [ ] A test that fails before the fix and passes after
