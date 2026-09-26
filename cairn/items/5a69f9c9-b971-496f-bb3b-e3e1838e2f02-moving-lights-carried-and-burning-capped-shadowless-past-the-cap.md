---
id: 5a69f9c9-b971-496f-bb3b-e3e1838e2f02
title: 'Moving lights: carried and burning, capped, shadowless past the cap'
type: feature
status: blocked
milestone: lighting
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 9bd9e8ab-6eef-44dc-b814-0b376ceec1e5
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

A colonist carrying a torch at night, a burning roof, fire arrows: these move or change every frame and can't be baked. They get the same shadows as static lights, within a budget. DESIGN.md §6e.

## What

- Any light the renderer sees change position or appear/disappear often is dynamic: drawn each frame into its own target with the same march, at the preset's rays and step.
- A cap per preset (4 / 8 / 16 / 32), nearest the view centre first. Past the cap a light still glows, without shadows.
- Fire spreading (9bd9e8ab) makes burning cells dynamic lights until they settle, so a spreading fire never rebakes the static target every frame.

## Acceptance criteria

- [ ] 64 moving lights at `medium`: 8 cast shadows, all glow, frame inside budget (bench)
- [ ] A burning building never triggers more than one static rebake a second (test)

## 2026-09-26

Blocked until something moves while emitting light. Core has no carried light, and burning (9bd9e8ab) is blocked itself, so a dynamic-light path now would have no caller. Everything that emits light today (campfire, stove) is a fixture, which the static bake covers. Unblock when fire spreads or a carried light lands.
