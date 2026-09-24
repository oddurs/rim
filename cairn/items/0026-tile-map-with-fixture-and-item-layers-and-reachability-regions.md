---
id: f2ea0bd1-a7a0-4b13-84e9-8249f70275e3
title: Tile map with fixture and item layers, and reachability regions
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: m
layer: engine
area: map
pillar:
- performance
---

## Why

Unreachable targets must be rejected in O(1) before any A* runs.

## Acceptance criteria

- [x] Regions rebuild only when passability changes
- [x] can_reach handles Cell and Touch goals
