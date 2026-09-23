---
id: 172
title: Hot reload of UI scripts and theme
type: feature
status: planned
milestone: interface
depends_on:
- 167
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: ui
pillar:
- plugin-first
---

## Why

Restyling and editing panels should feel like web development: save and see it.

## What

In dev mode a file watcher reloads a changed `ui/*.luau` or `theme.toml` into a fresh UI VM and swaps it in. The simulation is untouched (unlike 0084, which replays the sim). A broken file shows the error in place and keeps the last good version.

## Acceptance criteria

- [ ] Saving a UI script or theme updates the running game within a second
- [ ] The simulation's state hash is unaffected by any number of reloads
- [ ] A syntax or runtime error shows in place, naming file and line; the last good UI keeps running
