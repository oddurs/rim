---
id: affb8d41-045b-4949-ae5f-9f70b2ecc6b9
title: deaths grows faster than linear with the pawn count (exponent 1.28)
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

`deaths` grows faster than linear with pawns (50, 200, 800, 3,200 on a 500² map): exponent 1.28. Its mean ms per tick at each rung: 0.0011, 0.0033, 0.0337, 0.1865. The user's bar is that rim runs "beautifully and scalably", and a system that grows faster than the thing it works over is what stops that at a big colony or map.

Measured with the sim bench's `--scale` ladder (981e6792) on an M4 Pro, 0.1 in-game days a rung, at load 91-115, so the times are indicative; the growth exponent is the log-log slope of the system's mean ms per tick against size, where 1 is linear.

The deaths system, per tick, against the number of pawns (a 500² map, 50 to 3,200 pawns): something in it looks at every pawn for every pawn, or at every pawn and every thing.

## Proposal

- First, measure on a quiet machine: its cost per call against the size, and how many calls a tick makes. The exponent is their product, so it says which one grows.
- Then make the cost per call linear (or incremental) and the calls per tick flat.

## Acceptance criteria

- [ ] Measured quietly, cost per call and calls per tick at each rung, noted here
- [ ] `bench --scale pawns` shows `deaths` at exponent 1.15 or below
