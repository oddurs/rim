---
id: 006ef572-3963-4d91-bd4c-8952bfaccd2a
title: 'Work Board: passion on skills, shown as a flame'
type: feature
status: backlog
milestone: mood
depends_on:
- f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: additive
effort: m
layer: engine
area: ui
---

## Why

DESIGN.md §4d has the Work Board show passion as a flame, and passion is what makes a colonist's skills feel like theirs. Skills (29c323f5) have no passion yet, so the board (f1b96df4) shipped with the skill bar alone.

## What

- A passion per colonist per skill (none, interested, burning), rolled at arrival from the world RNG and saved.
- Passion speeds learning; burning passion ties into mood once mood exists.
- The board shows it as a flame on the cell and in the inspector's skills tab.

## Acceptance criteria

- [ ] Passion is rolled deterministically, saved and loaded
- [ ] A passionate colonist learns measurably faster (test)
- [ ] The board and the inspector show it
