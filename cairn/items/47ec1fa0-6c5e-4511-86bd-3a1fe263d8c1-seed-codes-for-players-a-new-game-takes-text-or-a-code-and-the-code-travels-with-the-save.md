---
id: 47ec1fa0-6c5e-4511-86bd-3a1fe263d8c1
title: 'Seed codes for players: a new game takes text or a code, and the code travels with the save'
type: feature
status: backlog
milestone: workbench
depends_on:
- c2579dbc-dfcf-418d-994f-187289e86387
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Problem

A world's seed is a number only the command line sees, so players can't share a map. DESIGN.md §7b.

## Proposal

New game shows a seed code and accepts any text (`hash_str`); the code includes map size and the mod lock's hash; the pause menu and bug capture show it.

## Acceptance criteria

- [ ] The same code with the same mods gives the same map (test)
- [ ] Text seeds work (test)
