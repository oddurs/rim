---
id: 84
title: Hot reload of defs and scripts in dev mode
type: feature
status: backlog
milestone: sdk
depends_on:
- 64
created: 2026-09-22
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Fast iteration for modders.

## What

Hot reload is deterministic replay (DESIGN.md §10): when a file changes, reload defs and scripts, rebuild the world from its seed and replay the command log to the current tick. The modder sees what the change would have done in this exact game. Snapshots from the save format make it faster later.

## Acceptance criteria

- [ ] File watcher reloads a mod without restarting
- [ ] World is rebuilt by replaying the command log to the current tick
- [ ] Load errors show in-game and keep the last good version running
- [ ] Under a second for a 10-minute session on the target map
