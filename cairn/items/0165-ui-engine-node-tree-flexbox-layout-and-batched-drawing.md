---
id: 165
title: 'UI engine: node tree, flexbox layout and batched drawing'
type: feature
status: planned
milestone: interface
depends_on:
- 163
- 164
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: l
layer: client
area: ui
pillar:
- performance
---

## Why

The core of the UI engine: turn a tree of plain nodes into laid-out, drawn pixels, fast enough to rebuild every frame.

## What

Primitive nodes (`box`, `row`, `col`, `text`, `image`, `spacer`, `scroll`) with style from tokens. Layout by taffy's flexbox subset: direction, gap, padding, grow, align, min/max size. Draw calls are batched; layout is cached and only recomputed when a tree or the viewport changes; scroll areas clip.

## Acceptance criteria

- [ ] Every primitive lays out and draws; scroll areas clip and scroll
- [ ] Layout reruns only when the tree or viewport changes (counter proves it)
- [ ] A tree serialises to indented text for snapshot tests
- [ ] UI build, layout and draw times appear in the profiler
- [ ] 300-node tree under 0.5 ms per frame on the reference machine
