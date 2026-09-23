---
id: 167
title: Client-only UI Luau VM with view and act APIs
type: feature
status: planned
milestone: interface
depends_on:
- 165
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: l
layer: client
area: scripting
pillar:
- plugin-first
- determinism
---

## Why

UI mods need a scripting runtime that can never desync the simulation (DESIGN.md §11).

## What

A second, client-only Luau VM in sandbox mode (the rules of 0140: frozen engine tables, no I/O). `ui.define(id, fn)` registers a component: a function from `view` to a tree. `view` is a read-only snapshot API (colonists, selection, pawns, needs, fields, messages, clock, defs). `act` queues client Actions and sim Commands, nothing else. Mod UI scripts load from `ui/*.luau` in load order with `require` limited to dependencies (0141 semantics).

## Acceptance criteria

- [ ] UI scripts load from `mods/*/ui/*.luau` in mod load order
- [ ] `view` is read-only; `act` only queues Actions/Commands
- [ ] Determinism test: a UI mod that tries to mutate the world leaves the state hash unchanged, and co-op-style runs with and without UI mods hash the same
- [ ] A component that errors shows an error box naming the mod; the rest of the UI keeps running
- [ ] Per-mod UI time in the profiler
- [ ] The view/act API is declared once in Rust so 0085 can generate types from it
