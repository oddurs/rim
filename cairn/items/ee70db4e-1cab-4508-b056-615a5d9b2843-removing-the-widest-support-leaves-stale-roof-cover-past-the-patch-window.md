---
id: ee70db4e-1cab-4508-b056-615a5d9b2843
title: Removing the widest support leaves stale roof cover past the patch window
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: building
---

## What

`Map::spread_cover` (crates/rim_sim/src/map.rs, ~line 944) patches cover around each changed support with a window of radius `top`, where `top` is the widest span standing *after* the change. When the change takes away (or narrows) the widest support on the roofed levels, its old reach is wider than `top`: cells between `top + 1` and the old span keep the cover the removed support gave them, and the ring just outside the window reseeds the window from those stale values.

## How it fails

- A roof is reported over cells no support reaches: a room counts as enclosed (sheltered) when it isn't.
- Save/load divergence: a load works cover out from scratch (`support_changed` is `None`), so the loaded game has different cover, different `Room::uncovered`, different shelter, and a different state hash from the game that kept running.

Reachable in core: on a map with no surface rock, stone walls (span 4 × 1.25 = 5) and log walls (span 4); deconstruct the last stone wall and the cells at distance 5 from it stay covered.

## Reproduce

Map test: two supports, spans 5 and 3, far apart; `ensure_rooms`; set the span-5 one to 0; `ensure_rooms`; cover differs from the brute-force answer (`brute` in map.rs tests). The existing property test `patched_cover_matches_the_whole_map` misses it because with 50 random spans of 1..6 the widest is never the only one.

## Acceptance

- [ ] Patched cover equals cover from scratch after the widest support is removed or narrowed
- [ ] A test that fails before the fix and passes after

## 2026-09-28

Each pending change now carries the span its cell had before; the patch window is the wider of that and the widest span left. Found independently by the save/load review too (it listed it as p3 since surface granite and stone walls both span 5 in core).

## 2026-09-28

Lowered to p2: in core, surface granite and stone walls both span 5, so the widest support is rarely the only one; it needs a map without surface rock, or a mod with longer spans.
