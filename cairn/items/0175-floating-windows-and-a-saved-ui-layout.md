---
id: 175
title: Floating windows and a saved UI layout
type: feature
status: backlog
milestone: colony
depends_on:
- 171
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Why

Inspection panels, work priorities and trade all want movable windows that remember where the player put them.

## What

A `window` component on the windows layer: draggable, resizable, closable, stacking on focus. Positions persist per player in a plain `ui_layout.toml`.

## Acceptance criteria

- [ ] Windows drag, resize, close and raise on click
- [ ] Layout persists in `ui_layout.toml` and survives restarts
- [ ] A hand-edited layout file is respected; a broken one is ignored with a warning
