---
id: 668c7762-97c3-4b65-bf33-55bca0ea9e66
title: 'Context: an action row and an inspector tab registry'
type: feature
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
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

- [x] Core's tabs and actions go through the registry
- [x] A mod adds a tab and an action without `ui.wrap`
- [x] Existing inspector ids and tests still pass

## 2026-09-26

Inspector moved to mods/core/ui/inspector.luau, which returns the tab and action registries. Core's tabs (overview, skills, work) and actions (core:draft R, core:center C) register through them; binding ids unchanged. Actions apply to things too (Centre). The palette test no longer depends on file load order.
