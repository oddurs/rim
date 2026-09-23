---
id: 141
title: 'Mod modules: require("@mod/path") limited to declared dependencies'
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 140
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: breaking
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

Today mods share APIs by assigning into the global `rim` table: names collide silently and nothing records who depends on whom. DESIGN.md §10: hard dependencies use `require`.

## What

A script can `return` a table of exports. `require("@core/storyteller")` returns it. Only mods listed in `depends` or `optional` can be required; an optional mod that isn't installed returns nil.

## Acceptance criteria

- [ ] `require("@<mod>/<script>")` resolves across mods; `require("./x")` within a mod
- [ ] Requiring a mod not in `depends`/`optional` is a load error naming both mods
- [ ] Module cycles are a load error
- [ ] Storyteller exports `register_incident`; wildlife_plus uses the require form
- [ ] Generated `.luaurc` aliases so luau-lsp resolves the same paths
