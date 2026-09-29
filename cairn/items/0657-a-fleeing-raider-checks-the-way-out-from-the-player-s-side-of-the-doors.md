---
id: 657
uid: 991b29e5-1d03-44a4-84eb-010785220565
title: A fleeing raider checks the way out from the player's side of the doors
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: ai
---

## What

`ai::leave` (ai.rs ~330) picks a map-edge cell with `w.map.can_reach(p.pos, Goal::Cell(c))`, which reads the Player's regions (doors open). `go_to` then asks `can_reach_as(.., p.faction, ..)`, where player doors are walls to a raider. `wander`, `flee` and `reachable` (used by animals and raiders) have the same mismatch.

## How it fails

A raider inside the colony's doors (in through a breach that was since rebuilt, or walled in) wants to leave. `leave` finds an edge the player could reach, `go_to` fails, the job ends with no delay, and it picks the same edge again: stuck every few ticks, never breaching out. Animals shut in waste their thinking the same way.

## Reproduce

Not yet run; verified by reading. A raider wounded inside a closed room with a player-owned door: it never moves, and never breaches.

## Fix

Use `can_reach_as(.., p.faction, w.climbs(p))` in `leave`, `wander`, `flee` and `reachable` for non-colonists. That changes behaviour (full).

## Acceptance

- [ ] A shut-in raider that wants to leave breaches or goes, instead of retrying the same edge
- [ ] A test that fails before the fix and passes after
