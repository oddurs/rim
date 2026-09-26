---
id: ff818bb3-7175-48f0-a3c3-c346c9bc2469
title: Lightning casts shadows
type: feature
status: backlog
milestone: lighting
depends_on:
- 8f4f1de8-5784-4377-8cee-25bcf223275e
created: 2026-09-26
updated: 2026-09-26
priority: p3
api: none
effort: s
layer: client
area: render
---

## Why

sky.rs already flashes the screen in heavy storms. A flash from a direction, throwing every wall's shadow across the ground for a frame, is the most dramatic thing lighting can do and costs one sky rebuild. DESIGN.md §6e.

## What

- A flash is a sky body for its few frames: high elevation, random azimuth, blue-white, hard shadows, full exposure.

## Acceptance criteria

- [ ] A flash rebuilds the sky target at most twice (on and off) (counter test)
