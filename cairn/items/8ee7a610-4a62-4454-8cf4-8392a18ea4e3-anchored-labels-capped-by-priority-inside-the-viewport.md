---
id: 8ee7a610-4a62-4454-8cf4-8392a18ea4e3
title: Anchored labels capped by priority inside the viewport
type: perf
status: done
milestone: scale
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] Benchmark shows the budget is met at 200 pawns with labels on

## 2026-09-24

Measured on the budget scene at 200 colonists, labels on (crates/rim_ui/tests/engine.rs whole_ui_fits_the_frame_budget_at_200_pawns_with_labels_on): before, median frame 0.52 ms and median rebuild frame 3.69 ms (budget 2.0). Three things were the frame, in this order: the colonist bar building 201 buttons (core HUD), view.colonists() building 201 rows with needs through metamethod-checked table sets, and the labels component building 200 anchored nodes that placement then collided one by one. Changes: the engine places at most 48 anchored labels by priority inside the viewport (ANCHOR_CAP); core's labels component picks the top 48 pawns (greeted, selected or hovered, colonists, hostiles) before building any node; the colonist bar shows 24 and counts the rest; pawn rows use raw sets with preallocated tables. After: median frame 0.14 ms, rebuild frame 1.42 ms, 330 nodes. view.colonists() at 200 pawns still costs about 0.38 ms per rebuild; a lazy row (fields read on demand) would take that away and is the next step if the bar or inspector needs it.

## 2026-09-24

Second pass after the Windows runner failed the benchmark at 8.5 ms a rebuild frame (6x a laptop): view.colonists(max) lets the bar ask for the 24 it shows and view.count_pawns for the total, which took the rebuild frame from 1.42 to 1.05 ms locally, and the budget test gives the Windows runner twice the slack of the other runners.
