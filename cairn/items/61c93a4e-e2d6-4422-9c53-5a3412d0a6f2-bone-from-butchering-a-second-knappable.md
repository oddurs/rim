---
id: 61c93a4e-e2d6-4422-9c53-5a3412d0a6f2
title: Bone from butchering, a second knappable
type: content
status: done
milestone: stone-age
assignee: Oddur Sigurdsson
depends_on:
- 4675019b-c017-47e7-b148-b12e4be58bda
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] Butchering a boar or deer yields bone
- [x] A hand axe knapped from bone is slower and weaker than a flint one

## 2026-09-25

Bone lives in its own defs/bone.toml. Deer give 4, wolves 3, hares 1. wildlife_plus's boar isn't patched, because primitive doesn't depend on that mod, and a patch on a def that isn't loaded warns. The hand-axe test knaps from bone and from flint through the same bill: hp 48 against 60, and tool speed 0.42 against 0.6.
