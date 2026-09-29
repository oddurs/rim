---
id: 709
uid: 300358b3-c32a-441b-97e5-b943ca8521a5
title: 'Scrolling in windows is smooth: pixel deltas, easing and no snap-back past the end'
type: feature
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: client
area: ui
api: none
---

## Problem

The user finds scrolling in RimOS windows janky. What the code does:

- The offset moves only when an event arrives (crates/rim_ui/src/lib.rs:914-920), with no easing or momentum of its own.
- On a trackpad, 1 pt of finger travel moves the content 4 px (UI wheel = y / NOTCH, times 40), which is too fast. A fast flick is cut off by the ±4-notch clamp a frame (crates/rim_client/src/main.rs:991-1026).
- Horizontal travel is ignored.
- `clamp_scroll` runs after paint (lib.rs:1641-1643), so one frame is drawn past the end and then snaps back. It also re-lays out the area every frame (1f51a5b4).
- Any wheel input rebuilds every mount (lib.rs:1297-1311).
- The Work board's column header isn't sticky.

## Proposal

A trackpad's pixel deltas map 1:1, and a mouse wheel's notch eases over a few frames. The offset is clamped before paint. Only the scrolled area repaints. Momentum comes from the OS's own momentum events.

## Acceptance criteria

- [ ] A trackpad scroll of N points moves the content N logical px (test)
- [ ] No frame is drawn past a scroll area's end (test)
- [ ] A wheel event rebuilds only the mount it scrolls (count in the UI bench)
