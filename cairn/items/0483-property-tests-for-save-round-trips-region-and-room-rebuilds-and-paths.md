---
id: 483
uid: 55952481-b664-4a1b-8dea-dc55c55b025c
title: Property tests for save round trips, region and room rebuilds, and paths
type: feature
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: m
layer: engine
area: tests
---

## Problem

The engine relies on invariants that example tests check only at a few points: incremental region and room rebuilds match a full rebuild, a save round trip is exact, a path never enters a blocked cell.

## Proposal

`proptest` as a dev-dependency of `rim_sim`, with four properties over random edits on small maps:
1. save, load, save is byte-identical;
2. regions and rooms after a random list of wall edits equal a full rebuild;
3. every path A* returns steps only through passable cells, 4-connected or with both orthogonals open;
4. patches applied in any dependency-respecting load order give the same defs.

## Acceptance criteria

- [x] The four properties run in the suite at a size that keeps them under 10 seconds together
- [x] Each fails, and shrinks to a small case, when its invariant is deliberately broken (shown)

## 2026-09-28

tests/properties.rs, proptest 1.11 as a dev-dependency (no default features; failure_persistence off, since the seed brings a failure back). Each property's cases come from its TestSeed (RngSeed::Fixed), so the nightly shift gives new cases and a failure prints the rerun line. All four run in about 0.1 s together. Findings while writing: rooms and regions are always rebuilt whole, so property 2 tests the dirty tracking (edits then a rebuild equal a fresh map from the same cells); terrain and fixture edits are separate, because setting terrain dirties every edit and hid a planted fixture bug in the first version. Patches: each (thing, field) is set by at most one mod (two setting the same field conflict by design). Deliberate breaks, each reverted: A* corner rule 'and' to 'or' shrank to one water cell ([(4, 8, Water)], start (14, 0), goal (0, 8)); set_fixture not dirtying on a door change shrank to [Fixture(0, 0, Door)]; restore bumping the tick when 20+ wood lies on the map shrank to [Stack { dx: 0, dy: 0, count: 20 }]; a set of hp dropping market_value shrank to two patches on one thing from two mods ({(4, 0): (3, 263), (4, 1): (1, 109)}). Each failure printed its seed and rerun line.
