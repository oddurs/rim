---
id: af316e9d-645f-4f7b-935b-9c9a2b1e8ba7
title: 'Main menu: continue, load a colony, or start a new one'
type: feature
status: backlog
milestone: persistence
depends_on:
- 01e86e7b-17ac-4e5a-8920-d72a438037f1
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: none
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

The game starts straight into a new colony. With saves (DESIGN.md §7a) a
player needs to come back to one: continue the last, pick another, or start
fresh. Split from 01e86e7b, which makes every game save itself; this is the
screen in front of it.

## What

A screen before any world exists: Continue (the newest save), a list of
colonies (name, day, colonists, last played) to load, and New colony. Until
it lands, `rim --continue` and `rim --load FILE` do the same from the
command line.

## Acceptance criteria

- [ ] Continue loads the newest save
- [ ] The list shows every save with its colony's day and colonists, newest first
- [ ] New colony starts a game with a new save
- [ ] A save that fails to load says why and doesn't crash the menu
