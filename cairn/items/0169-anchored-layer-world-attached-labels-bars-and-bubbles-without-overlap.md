---
id: 16eecb22-0f2b-4532-bc5d-3795083879a8
title: 'Anchored layer: world-attached labels, bars and bubbles without overlap'
type: feature
status: done
milestone: interface
depends_on:
- b144ca3c-2985-47be-b3ce-d07d4f17ceaa
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
effort: m
layer: client
area: ui
pillar:
- performance
---

## Why

Names, health bars, the sleep marker, the order label and speech bubbles belong to things in the world, and today they overlap.

## What

Anchored nodes attach to an entity or a cell and follow the camera. A placement pass nudges overlapping labels apart by priority (selected, colonists, hostiles, others) and culls offscreen ones. Bubbles have a lifetime and a tail pointing at their anchor.

## Acceptance criteria

- [x] Two adjacent pawns' names never overlap (test with a crowd of 10)
- [x] Priority order: selected, colonists, hostiles, others
- [x] Offscreen anchors cost nothing
- [x] A bubble component with lifetime and tail; core shows one when a colonist joins
- [x] 50 anchored labels under 0.2 ms per frame

## 2026-09-23

mods/core/ui/labels.luau. Placement tries 12 slots away from the anchor by priority (selected, colonists, hostiles, others) and drops a label rather than overlap (test: 10 colonists on two cells). Offscreen anchors are skipped. Bubbles with a tail say hello when a colonist joins. Anchored layouts are cached by content. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
