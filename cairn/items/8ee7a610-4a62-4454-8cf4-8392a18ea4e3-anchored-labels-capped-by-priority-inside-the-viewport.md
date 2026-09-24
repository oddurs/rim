---
id: 8ee7a610-4a62-4454-8cf4-8392a18ea4e3
title: Anchored labels capped by priority inside the viewport
type: perf
status: backlog
milestone: scale
created: 2026-09-24
updated: 2026-09-24
priority: p2
api: none
effort: s
layer: engine
area: ui
---


## Budget

Anchored-label collision is per label. At 200 pawns that is the frame.

## Measurement (before)

To take: the budget scene at 200 pawns, labels on.

## Approach

Cap labels to the top N by priority (selected, colonists, hostiles, others)
inside the viewport before collision; the rest draw nothing.

## Acceptance criteria

- [ ] Benchmark shows the budget is met at 200 pawns with labels on
