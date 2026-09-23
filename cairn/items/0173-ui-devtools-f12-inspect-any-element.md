---
id: 173
title: 'UI devtools (F12): inspect any element'
type: feature
status: planned
milestone: interface
depends_on:
- 171
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: m
layer: tooling
area: ui
pillar:
- plugin-first
---

## Why

Modders need to see what's on screen and who put it there, like browser devtools.

## What

F12 toggles an inspector: hover highlights the node under the cursor and shows its id, owning mod (after mod operations), layout box, tokens and build time; a tree view lists the whole UI; a toggle outlines every layout box.

## Acceptance criteria

- [ ] Hovering any element shows id, owning mod and layout box
- [ ] Tree view of the whole UI, collapsible
- [ ] Layout-bounds outline toggle
- [ ] Devtools are themselves a core UI component (dogfood)
