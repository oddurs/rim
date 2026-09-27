---
id: 24bad102-2095-49dc-8386-3f1e2bc343ed
title: 'Lighting auto preset: step down when the lighting itself runs slow'
type: feature
status: backlog
milestone: lighting
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: m
layer: client
area: render
depends_on:
- 12fbe8fb-c581-4f2b-81e6-bdd2a2eb72b3
---

## Why

A player who never opens the settings should still get a lighting preset their machine can carry. The presets (12fbe8fb) shipped without `auto`: the first version watched mean frame time. That mixes in vsync, the sim, the UI and everything else, so a slow sim would have turned the lights down without making anything faster. It was cut in review.

## What

- `auto` in `[lighting] quality`, starting at `medium`.
- It steps down on what the lighting passes cost, not on the frame. GPU timer queries work only on desktop GL 3.3+ that isn't Apple's tile renderer (bench.rs), so elsewhere it needs another signal, or it stays where it is and says so.
- It never steps up mid-game, and it says in the log when it steps.

## Acceptance criteria

- [ ] With the lighting slowed artificially, `auto` steps down one preset per window and stops at `low` (test)
- [ ] A slow sim with cheap lighting leaves the preset where it is (test)
