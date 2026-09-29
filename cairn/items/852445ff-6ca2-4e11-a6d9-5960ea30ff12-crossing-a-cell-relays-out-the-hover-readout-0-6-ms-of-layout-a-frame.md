---
id: 852445ff-6ca2-4e11-a6d9-5960ea30ff12
title: 'Crossing a cell relays out the hover readout: ~0.6 ms of layout a frame'
type: perf
status: backlog
milestone: bare-metal
depends_on:
- 90e15c25-52a0-46d3-9011-e0b12fb08a97
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: perf
---

## Problem

With the shell's trees shared (90e15c25), holding still costs ~0.37 ms of UI a frame on the core UI with 30 colonists, and crossing a cell every frame ~1.0 ms. The difference is almost all layout: rim_ui's layout_us goes from ~0.2 ms to ~0.9 ms when the hover readout's text changes each frame. 06014a48 already keeps layout between frames and relays out only a changed tree and its ancestors, so the cost is in that path: shaping the readout's new text, or relaying out the right column around it.

## Measurement

`cargo test --release -p rim_ui --test shell_cost -- --ignored --nocapture` prints median frame, layout and paint time, still and crossing.

## Acceptance criteria

- [ ] Crossing a cell costs within 0.2 ms of holding still on the core UI with 30 colonists (shell_cost, medians, the same machine and load for before and after)
- [ ] The PR names where the time went, measured, before changing it
