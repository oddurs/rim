---
id: 668c7762-97c3-4b65-bf33-55bca0ea9e66
title: 'Context: an action row and an inspector tab registry'
type: feature
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: m
layer: core
area: ui
---

## Why

The inspector's tabs are a fixed list in hud.luau, and the actions for a selection (draft, follow) live only on keys. A mod that adds a tab or an action has to wrap the whole panel.

## What

- `inspector.tab{ id, label, applies = fn(sel) -> bool, build = fn(view, sel) }` and `inspector.action{ id, label, key?, applies, run }`, both registries in a core module.
- Core's own tabs (overview, skills, work) and actions (draft, follow) register through them; ids `core:inspector.tabs.<id>` stay.
- An action row under the inspector title shows the actions that apply.

## Acceptance criteria

- [ ] Core's tabs and actions go through the registry
- [ ] A mod adds a tab and an action without `ui.wrap`
- [ ] Existing inspector ids and tests still pass
