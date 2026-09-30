---
id: 723
uid: 6863632f-652c-4a9e-a2b5-d46b7bbb1520
title: 'Whole-map indices down: chunk paint gets a far-zoom level of detail'
type: perf
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-29
created: 2026-09-29
updated: 2026-09-29
priority: p1
api: none
effort: m
layer: client
area: render
---

## Problem

After #390 the whole map is 59 calls but still about 1.27 million indices a frame, 1.25 million of them chunk meshes painted at 4 points a cell. Detail smaller than a pixel still costs vertex work: in software GL (CI), on integrated GPUs, and in upload when a region joins again. Indices should grow with what can be seen, not with the map (981e6792's scaling bench).

## Proposal

Measure first: which layers and kinds of thing the whole map's indices come from. Then give chunk paint a level of detail below a threshold in points a cell, chosen from that split (flat cells, or a look's far form). A chunk painted at another level is repainted as zoom crosses the threshold, like any zoom repaint.

## Acceptance criteria

- [x] The whole map's mesh indices by layer and kind are recorded here before any change
- [ ] Whole-map indices at least halved (bench), with screenshots at 4, 8 and 12 points a cell before and after
- [ ] No view's world + ui time worse on one CPU (the render-bench artifact's machine field)

## 2026-09-29

Measured before any change (a local probe attributing each painted cell's indices to its thing, seed 1 with 20 sprite mods, whole map at 4 points a cell; the total 1,249,365 matches the bench's mesh indices exactly). By kind: floors 380,646 (30%, 54 a cell: 9 quads of plank pattern), granite rock 276,831 (22%, 75 a cell), walls 181,752 (15%, 83), oak trees 114,720 (9%, 96), deadfall 50,292 and tall grass 30,360 (6%, 66), berry bushes 28,032 (192 a cell), doors 18,480, then a long tail of items and furniture at 48 to 432 a cell. The top four are 76%. At 4 points a cell (4 px on CI, 8 on a Mac) all of it is detail below a pixel or two.

## 2026-09-29

Second probe, by primitive and on-screen size (whole map, 4 points a cell): 8-sided polygons are 920k of the 1.25M indices (74%): 444,912 from discs under 2 points across, 428,928 from 2 to 4 points, 27,936 under 1 point and 17,520 at 4 points or more, at 24 indices each (a centre fan of 8 triangles). Rects are 259k (6 each), lines 67k and triangles 12k. Under half a point there are 39,924 of rect, 10,164 of triangle and 1,920 of polygon. So the cut isn't flattening cells, which would turn sparse grass and canopies into squares; it's polygon cost: fan from a rim vertex (sides - 2 triangles, the same shape), fewer sides when small (under 2 points: 4; under 4: 6), and skipping primitives under half a point. Estimate: about 0.62M. Sizes scale with zoom, so it's a level of detail with no fixed threshold.
