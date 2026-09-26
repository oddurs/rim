---
id: 61c93a4e-e2d6-4422-9c53-5a3412d0a6f2
title: Bone from butchering, a second knappable
type: content
status: backlog
milestone: stone-age
depends_on:
- 4675019b-c017-47e7-b148-b12e4be58bda
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: s
layer: plugin
area: building
---

## Why

Split from e3846c47. A hunted animal should give you something to make tools from as well as meat. Bone is a poorer flint.

## What

- **Bone** is an item: tagged `knappable`, lithic stuff at `tool_speed` 0.7 and hp 0.8.
- **A patch** adds bone to the butchering yield of core's animals.

## Acceptance criteria

- [ ] Butchering a boar or deer yields bone
- [ ] A hand axe knapped from bone is slower and weaker than a flint one
