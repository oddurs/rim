---
id: 1b85ddb0-5430-4706-8ed7-09fe956f9fef
title: 'Build toolbar: group by menu so mods don''t push it off screen'
type: bug
status: backlog
milestone: interface
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: m
layer: client
area: ui
---

## Why

Every buildable thing gets its own toolbar button, whatever its `build.menu`. With core, primitive, wildlife_plus and crafting loaded, the bar at 1600×960 already runs past the right edge. "Clear zone" is clipped, and each mod that adds a building pushes more off. `menu` exists for this: "Anything with [build] shows up in the build toolbar under its `menu`".

## What

- One button per `menu` ("structure", "furniture", "production"), opening its things. Designations stay as they are.
- A mod's new menu gets a button without any UI change.

## Acceptance criteria

- [ ] The toolbar fits at 1280 px wide with every shipped mod loaded
- [ ] Buildings are grouped by `build.menu`
- [ ] The autotest's toolbar checks still find each thing's button
