---
id: 1b85ddb0-5430-4706-8ed7-09fe956f9fef
title: 'Build toolbar: group by menu so mods don''t push it off screen'
type: bug
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: m
layer: core
area: ui
---

## Why

Every buildable thing gets its own toolbar button, whatever its `build.menu`. With core, primitive, wildlife_plus and crafting loaded, the bar at 1600×960 already runs past the right edge. "Clear zone" is clipped, and each mod that adds a building pushes more off. `menu` exists for this: "Anything with [build] shows up in the build toolbar under its `menu`".

## What

- One button per `menu` ("structure", "furniture", "production"), opening its things. Designations stay as they are.
- A mod's new menu gets a button without any UI change.

## Acceptance criteria

- [x] The toolbar fits at 1280 px wide with every shipped mod loaded
- [x] Buildings are grouped by `build.menu`
- [x] The autotest's toolbar checks still find each thing's button

## 2026-09-26

Built as the dock (core:toolbar in mods/core/ui/toolbar.luau): Orders Q, Build B, Zones Z; build groups are build.menu in definition order. The client files each tool (ToolView.category/group), so a mod's menu needs no UI change. Only the open palette is built. Widest palette with every shipped mod: 692 pt. Escape moved into the core:escape binding (tool+palette, then palette, then selection). Z now opens the Zones palette; the stockpile list is a button there until the sheets item gives it a screen key.
