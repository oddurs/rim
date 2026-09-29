---
id: 455
uid: 2763e32b-d779-471e-9f74-1d24f810c4a8
title: Unseen rock is plain until someone stands beside it
type: feature
status: backlog
milestone: rock-face
depends_on:
- 346
- 537
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: s
layer: client
area: render
---

## Problem

Prospecting (db7f1e06) says a rock cell shows its kind and whether it is veined once a pawn has stood next to it, and the view (5689930d) keeps that seen bit. If the kind's colour showed before then, prospecting would be free. DESIGN.md §6g, "Finding it".

## Proposal

- An unseen solid cell draws its mass, outline, relief and contours, which are always shown because it blocks. Its fill is the stratum's `unseen` colour (core surface `#6f6a64`, darker by level), with neutral fractures and grit that are the same for every kind. No kind colour, no pattern, no ore.
- A cell that becomes seen draws its kind in over 0.5 s. Its chunk is live while the fade runs and then goes back to the cache.
- `[[stratum]]` gains `unseen = "#rrggbb"`.

## Acceptance criteria

- [ ] Two unseen cells of different kinds draw identically (pixel test)
- [ ] A miner walking along a face reveals the cells beside the path and no others (test on the seen bit, shot of the result)
- [ ] After a reveal finishes, the chunk rebuilds no more (mesh cache test)
