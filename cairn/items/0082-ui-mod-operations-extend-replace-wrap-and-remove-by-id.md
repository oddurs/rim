---
id: 82
title: 'UI mod operations: extend, replace, wrap and remove by id'
type: feature
status: done
milestone: interface
depends_on:
- 171
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Plugins must be able to show their own state and restyle or rearrange the game's, without forking core's UI.

## What

Every component has a namespaced id. Mods get four operations, applied in load order after all UI scripts load:

- `ui.extend(id, children)` adds children to a container (a tab, a panel in a region, a toolbar button).
- `ui.replace(id, fn)` takes over a component.
- `ui.wrap(id, fn(inner))` decorates one.
- `ui.remove(id)` hides one.

Two mods replacing or removing the same id is a conflict, reported like def patch conflicts (DESIGN.md §11). Devtools show which mod contributed each node.

## Acceptance criteria

- [x] All four operations work on core's HUD components
- [x] `wildlife_plus` adds a wild-animal counter to the top bar with `ui.extend`
- [x] Two mods replacing one id is reported as a conflict naming both, and load order decides the winner
- [x] Operating on an unknown id is a warning naming the mod and the id
- [x] Each node records which mod contributed it, for devtools

## 2026-09-23

extend/replace/wrap/remove apply wherever a node carries the id (components and inner nodes). wildlife_plus adds a wildlife counter to core:topbar.right. Conflicts (two mods replacing one id) and unknown ids are reported; nodes record their owner. Tests: mods_extend_replace_wrap_and_remove, wildlife_plus_extends_the_top_bar. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
