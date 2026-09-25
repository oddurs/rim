---
id: cf55217a-a9a4-412c-b368-dce307540b82
title: Strikes and exits on worksites
type: feature
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-25
depends_on:
- 74ff83ce-1d16-43f7-a49b-ca3340d509d3
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: m
layer: client
area: render
pillar:
- plugin-first
---

## Why

Each blow and each finish should feel physical, without the sim sending events for decoration (DESIGN.md §6b).

## What

- Strikes derived by the client: a busy site's `done` crossing a multiple of `every` since the last frame. Wind-up, lunge, impact flash, shake, and the style's particles, thrown toward the worker.
- Exits when a site the client was drawing finishes: fall (away from the worker, swinging to an open side), crumble, pop, settle.
- One particle pool allocated once; at most 48 per site; seeded by entity and strike, so a replay draws the same.
- Level of detail: no particles, shake or lunge below 20 points a cell; a strike flashes the cell outline instead.
- A readout for busy, hovered or selected sites when zoomed in: a bar and "Chop · 62% · 3 s", or hp.

## Acceptance criteria

- [x] Strikes key off work done, so a faster worker strikes faster
- [x] A felled tree falls away from the cutter, or to an open side
- [x] No allocation per strike; the particle pool is fixed
- [x] Zoomed out, strikes show as a flash and nothing else
