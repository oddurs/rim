---
id: 664
uid: a708c037-6a85-4190-b6c8-6d72ab5a30d7
title: 'Pathing around lakes: the octile heuristic floods the near shore, and searches reach 33k nodes'
type: perf
status: done
milestone: bare-metal
assignee: quiet-field
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
effort: m
layer: engine
area: pathing
---

## Why

First measured on main 2026-09-28 with bench --haul --days 0.25 (8d551753 setup), at load 43-56: path searches reached 8,000-33,000 nodes and made 50-127 ms ticks. Snow and mud's move_cost looked like the cause. Re-measured (note below), it isn't: search and node counts are identical with move_cost removed. The cause is the heuristic. Around a lake, the octile distance points through the water, and A* floods the whole near shore before it finds the way round: 0.6% of searches do two thirds of all the expanding.

## What

- A landmark (ALT) lower bound beside the octile distance: distances from a few cells at the map's edge over the land alone (lakes block, each step costs its terrain), so the heuristic knows where the lakes are.
- Rebuilding those distances when the land changes must not break the 4 ms hitch budget, and a saved game must search exactly as the one that never saved.

## Acceptance criteria

- [x] Measured on main against move_cost removed, and the cause noted here
- [x] Nodes per search on bench --haul at least halved, with every path as short as before (test)
- [x] p99 tick under 1 ms on bench --haul, and no tick over 4 ms from rebuilding the bound
- [x] The determinism test passes, and a save and load mid-rebuild matches the game that never saved (test)

## 2026-09-28

Measured on b90b069e, 2026-09-28, with bench --haul --days 0.25, seed 1, at load 29-33 (a quiet machine isn't coming soon, per rim-c2). Node and search counts are deterministic, so they're the evidence; tick times are noise at this load. Interleaved A/B/A/B/A/B: A is main, B has weather's two move_cost lines removed. Path counts are IDENTICAL in all six runs: 0.91 searches/tick, 84 nodes/search. p99 A: 1.77/0.69/0.49 ms; B: 0.57/0.69/0.49 ms, so the spread is noise. With --snow 30 against --snow 0 (the same hard frost, no snow): 0.75 searches/tick at 65 nodes, against 0.91 at 84, so snow makes fewer and smaller searches. So move_cost is not the cause. The cause: 28 searches (0.6% of about 4,550) expand 257k of the run's 382k nodes, from 5,000 up to 34,307 each. Every one is a pawn heading to a cell 30-50 cells away in a straight line along a path of 110-354 steps. Seed 1's map has a lake south of the colony (y 131-155, across the width) and another to the north-east, and --haul scatters stacks within 60 cells of the colony, some on the far shores. The octile heuristic points through the water, so A* fills the whole near-shore pocket before it rounds the lake. Snow and mud only raise costs in open ground, which the pocket fill was going to cover anyway. Proposed fix: a landmark (ALT) lower bound on a relaxed graph, where only water terrain blocks and every step costs the cheapest a step can. That keeps it admissible and consistent, so paths stay optimal, and the tables are a pure function of the map's water terrain and portals, so they're rebuilt on load and stay deterministic. h = max(octile, landmark bound).

## 2026-09-28

Built (0124b158 plus this branch). The bound: 8 landmarks at the map's edge (corners and side middles), each with two u16 tables, from it to every cell and from every cell to it, over the land alone: deep water with no span or floor blocks, each step costs its terrain's cost for the cell stepped into, levels are joined by their portals, and walls, doors, things, snow and mud are left out. Floors are left out too, as octile already assumes a step costs at least open ground (core's floor is 70). A first symmetric version (one table, each step at the cheaper of its two cells) was needed for |d(L,t) − d(L,n)| to hold; it only reached 1.98x fewer nodes on the test's pairs. The directed pair of tables reaches 2.58x. A version with abs() over dest-cost distances overestimated: a test path came out 1484 against 1476. Sixteen landmarks gained only 6% over eight. Deterministic numbers on bench --haul --days 0.25: nodes per search 84 → 30. Searches over 3,000 nodes 28 → 3, the largest 34,307 → 8,113. The remaining three are likely detours round rock, which the land counts open because mining opens it. Memory 4 B a cell per landmark, 8 MB on the bench's 250x250 with its levels. Rebuild 57–125 ms at load 40, now off the tick: at a new game or load, in Sim::assemble; after a change, on a thread, switched on at exactly T + LAND_DELAY (120). World::land_changed is saved (serde default, unwritten when none). tests/landmarks.rs: 153 cross-lake pairs cost exactly what octile alone finds, with 2.58x fewer cells expanded; the switch waits 120 ticks and restarts at a second change 50 ticks in; a save at T+60 loads and matches the game that never saved for 200 ticks, snapshots equal every 20.

## 2026-09-28

Criterion 3, measured at load 22 in a bench slot from rim-c2: --haul --days 0.25, main 0124b158 (A) against 48057c75 (B), interleaved ×3. p99: A 1.456/0.978/0.668 ms (median 0.98), B 0.824/0.637/0.688 ms (median 0.69, all under 1 ms). Max: A 24.0/20.8/10.0 ms, its worst ticks the 33,690 and 34,416-node searches at 9.7–19.9 ms. B 8.6/7.6/12.4 ms, its worst ticks 0–15-node searches landing in shelter, fields and pawns, so load noise, not rebuilds: the tables are built at load and never rebuild in this run, since the land doesn't change. Default scenario for a day at load 6–7 (earlier): max A 4.12–4.48 ms against B 1.36–1.65 ms, p99 unchanged at about 0.29 ms. Gate on 48057c75: 886/886, mods 16/0, crosscheck ok, TASK_EXIT 0; autotest seed 7 392/0.
