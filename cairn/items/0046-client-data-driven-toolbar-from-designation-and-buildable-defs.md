---
id: 452e1558-998c-4ec7-924a-f16ef16d41bd
title: 'Client: data-driven toolbar from designation and buildable defs'
type: feature
status: done
milestone: castaway
depends_on:
- 793e0a96-d4af-49d4-ad58-ff7493ddfdae
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
