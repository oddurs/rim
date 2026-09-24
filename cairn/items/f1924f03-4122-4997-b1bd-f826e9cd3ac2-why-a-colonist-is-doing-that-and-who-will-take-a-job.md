---
id: f1924f03-4122-4997-b1bd-f826e9cd3ac2
title: Why a colonist is doing that, and who will take a job
type: feature
status: backlog
milestone: colony
depends_on:
- 0e73145a-39c4-4f1d-88a4-59d803a2f535
- f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
pillar:
- growth
effort: m
layer: client
area: ui
---




## Why

"Why is nobody cooking?" is the question players ask most and the one the genre answers worst. The data already exists in the pools; show it (DESIGN.md §4d).

## What

- The why panel on a colonist: what they picked with its score, and each work type they passed over with its reason (unreachable, reserved, no materials, priority 0), each linked to the map.
- Hovering a Work Board column lights its waiting jobs on the map.
- Hovering a job on the map ranks who would take it and roughly when ("Bo in ~20s, then Cyd").
- `rim.explain_work(pawn)` and `view.explain_work(pawn)` expose the same to scripts and mods.

## Acceptance criteria

- [ ] The why panel names the real reason for each skipped work type (tests for each reason)
- [ ] The "who takes this" ranking matches who actually takes it on a test map
- [ ] Costs nothing measurable when nobody is inspecting
