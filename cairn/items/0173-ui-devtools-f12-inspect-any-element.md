---
id: 959bba6d-ac66-47e9-9dad-09f005158cd3
title: 'UI devtools (F12): inspect any element'
type: feature
status: done
milestone: interface
depends_on:
- 5514aac6-0300-4ff6-be19-fd44a5e50089
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Hovering any element shows id, owning mod and layout box
- [x] Tree view of the whole UI, collapsible
- [x] Layout-bounds outline toggle
- [x] Devtools are themselves a core UI component (dogfood)

## 2026-09-23

mods/core/ui/devtools.luau: inspect (id, owner, kind, box, path), layout outlines, the whole tree with owners, kit gallery. Screenshots reviewed. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
