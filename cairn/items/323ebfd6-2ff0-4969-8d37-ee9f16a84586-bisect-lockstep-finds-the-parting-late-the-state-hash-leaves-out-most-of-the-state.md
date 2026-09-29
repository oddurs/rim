---
id: 323ebfd6-2ff0-4969-8d37-ee9f16a84586
title: 'bisect::lockstep finds the parting late: the state hash leaves out most of the state'
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: tooling
area: tests
---

## What

`bisect::lockstep` steps two games side by side and compares `World::state_hash` every tick, reading the section hashes only once they differ. `state_hash` (world.rs ~3151) covers pawns' positions, hp, priorities, plans, skills, worn and wet; things' def, position and count; spoil, growth, fields' outdoor values, stock fields, zones, stance, standing orders, roles and script data. It leaves out:
- pawns' needs, jobs, carry, hand and paths;
- things' hp (except where spoiling), work, designations, orders and stores;
- the map's layers, water, room values;
- reservations and events.

## How it fails

A divergence in anything left out (a need rounding one way, a reservation, a stack's condition) goes unseen by `lockstep` until it moves something hashed, maybe thousands of ticks later. The parting tick it reports is then far from the cause. The crosscheck prints the snapshot hash as well, so it isn't fooled, but `lockstep` and the modtest `hash` are.

## Direction

Compare the section hashes every tick in `lockstep` (slower, but it's a debugging tool), or widen `state_hash` to cover needs, jobs, thing hp and designations cheaply.

## Acceptance

- [ ] A one-point difference in a pawn's need is found at the tick it happens
- [ ] A test that fails before the fix and passes after
