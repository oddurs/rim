---
id: 46
title: 'Client: data-driven toolbar from designation and buildable defs'
type: feature
status: done
milestone: castaway
depends_on:
- 45
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: none
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

A mod that adds a buildable gets a toolbar button with no client change.

## Acceptance criteria

- [x] Tools generated from defs
- [x] Drag rectangles to designate, build or cancel

## 2026-09-23

Verified by rim --autotest (crates/rim_client/src/autotest.rs), which drives the real client through the same Actions keyboard and mouse produce, checks state and saves screenshots; 56/56 checks pass, screenshots reviewed by eye. Runs in CI on macOS. Toolbar: one button per designation and buildable def (11). Chop drag designates only chop targets; cancel drag removes blueprints. Changed: walls (any blocking buildable) now drag as a room outline instead of a filled rectangle, with a matching preview.
