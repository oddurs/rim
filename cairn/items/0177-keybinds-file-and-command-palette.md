---
id: 177
title: Keybinds file and command palette
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 171
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Every action, core or modded, should be named, rebindable and findable without learning hotkeys.

## What

Actions are registered by id (core and mods) and bound in `keybinds.toml`. Ctrl+K opens a palette that fuzzy-searches every action by name.

## Acceptance criteria

- [ ] All core actions are named and bound in `keybinds.toml`
- [ ] Mods register actions; they appear in the palette immediately
- [ ] Rebinding by editing the file, and conflicts between bindings are reported
