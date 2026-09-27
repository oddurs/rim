---
id: 6202120c-27a8-4bba-a1ce-e1960d5519e7
title: 'mods/devtools: the console, a Luau REPL over the dev API'
type: feature
status: backlog
milestone: workbench
depends_on:
- 6ae1513a-d8d0-4744-90e5-386d6d1d1921
- cec3efdf-dc49-4fc0-a35e-5d649efe90e6
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: plugin
area: ui
---

## Problem

The command palette runs bindings, not expressions, and there is no place to ask the running sim a question. DESIGN.md §11a.

## Proposal

With `--dev` (or the setting), `` ` `` opens a console: a Luau prompt in a dev VM with the sim's read API, `dev.*` (dev commands), `sel` (the selection) and helpers like `near(sel)`. History, completion from `types/rim.d.luau`, tables printed as trees, errors shown in full. The story console (1da662ac) adds its commands here.

## Acceptance criteria

- [ ] Autotest by node id: open the console, spawn a thing, see it in the world and the command in the log
- [ ] A Luau error prints its whole message and traceback
- [ ] Without `--dev` the key does nothing

## 2026-09-27

Ruled 2026-09-27 (DESIGN.md §11a): the workbench's tools are a first-party plugin, mods/devtools, loaded only with --dev. Build this item's UI there, on the dev API (6ae1513a-d8d0-4744-90e5-386d6d1d1921); anything the engine must add goes in that item.
