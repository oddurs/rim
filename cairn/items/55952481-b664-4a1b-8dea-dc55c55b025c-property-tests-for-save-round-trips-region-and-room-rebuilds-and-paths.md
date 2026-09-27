---
id: 55952481-b664-4a1b-8dea-dc55c55b025c
title: Property tests for save round trips, region and room rebuilds, and paths
type: feature
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
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

- [ ] The four properties run in the suite at a size that keeps them under 10 seconds together
- [ ] Each fails, and shrinks to a small case, when its invariant is deliberately broken (shown)
