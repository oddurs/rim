---
id: 613
uid: 489acd07-db04-4ea0-9fcc-6d55d02ddbf3
title: rooms grows faster than linear with the map (exponent 1.83)
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: engine
area: perf
---

## Problem

`rooms` grows faster than linear with map cells (128², 250², 500², 1000², with 200 pawns): exponent 1.83. Its mean ms per tick at each rung: 0.0108, 0.0000, 1.0179, 1.5478. The user's bar is that rim runs "beautifully and scalably", and a system that grows faster than the thing it works over is what stops that at a big colony or map.

Measured with the sim bench's `--scale` ladder (981e6792) on an M4 Pro, 0.1 in-game days a rung, at load 91-115, so the times are indicative; the growth exponent is the log-log slope of the system's mean ms per tick against size, where 1 is linear.

Rooms rebuild whole (ensure_rooms) whenever anything changed, so a bigger map pays for every cell on each rebuild and, with more going on, rebuilds more often.

## Proposal

- First, measure on a quiet machine: its cost per call against the size, and how many calls a tick makes. The exponent is their product, so it says which one grows.
- Then make the cost per call linear (or incremental) and the calls per tick flat.

## Acceptance criteria

- [ ] Measured quietly, cost per call and calls per tick at each rung, noted here
- [ ] `bench --scale map` shows `rooms` at exponent 1.15 or below
