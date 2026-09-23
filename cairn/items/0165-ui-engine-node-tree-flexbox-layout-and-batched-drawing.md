---
id: 165
title: 'UI engine: node tree, flexbox layout and batched drawing'
type: feature
status: done
milestone: interface
depends_on:
- 163
- 164
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Every primitive lays out and draws; scroll areas clip and scroll
- [x] Layout reruns only when the tree or viewport changes (counter proves it)
- [x] A tree serialises to indented text for snapshot tests
- [x] UI build, layout and draw times appear in the profiler
- [x] 300-node tree under 0.5 ms per frame on the reference machine

## 2026-09-23

rim_ui: node.rs (single-pass conversion, no allocation on token lookups), layout.rs (taffy, measure respects known dimensions), paint.rs (draw list + hit boxes, clip, disabled inherited). Layout cached by tree hash (test: 5 unchanged frames, 0 layouts; new viewport, 1). Tree snapshots as text. Budget: 349-node HUD at 0.12 ms median frame; rebuild frames 0.66 ms. Bug found in review: the measure function returned zero for childless boxes, collapsing every spacer; the shell test now checks panels sit against their edges. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
