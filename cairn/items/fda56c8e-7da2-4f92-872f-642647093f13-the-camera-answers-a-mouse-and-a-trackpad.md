---
id: fda56c8e-7da2-4f92-872f-642647093f13
title: The camera answers a mouse and a trackpad
type: feature
status: done
milestone: pointer
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: m
layer: client
area: ui
---

## Why

Panning is WASD, the arrows or a middle-button drag, so a trackpad can't pan with the pointer, and every scroll zooms, so two fingers zoom where a Mac user expects them to move the view.

## What

- Each scroll event is classed: precise (a trackpad, fractions, both axes) pans one to one with the fingers, in points divided by the zoom, momentum included; stepped (a wheel, whole notches) zooms at the pointer. Cmd or Ctrl with any scroll zooms.
- Right-drag past 6 px pans (and orders nothing); `-` and `=` step the zoom.
- A `scroll = "auto" | "zoom" | "pan"` setting, saved with the others, with palette commands.
- Several events a frame are summed into one camera move.

## Acceptance criteria

- [x] Fractional two-axis scroll pans the map by exactly its delta; whole notches zoom at the pointer
- [x] Cmd or Ctrl with a scroll zooms
- [x] A right-drag pans and gives no order
- [x] The scroll setting overrides the guess and survives a restart

## 2026-09-26

Scroll events are sorted per event (classify): whole notches on one axis are a wheel, anything else is precise travel in points; a round-numbered event within 300 ms of precise scrolling still counts as the trackpad. Auto pans travel and zooms notches; Cmd/Ctrl zooms either; the scroll setting pins zoom or pan. Right-drag pans past 6 pt and gives no order. Pinch still needs the macOS magnify event (2f756dfc).
