---
id: 452628d7-a341-478d-9670-735b6eef1445
title: Client-only UI Luau VM with view and act APIs
type: feature
status: done
milestone: interface
depends_on:
- ad0ab61f-b5e7-4a3d-b216-d1b9e7251cfb
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] UI scripts load from `mods/*/ui/*.luau` in mod load order
- [x] `view` is read-only; `act` only queues Actions/Commands
- [x] Determinism test: a UI mod that tries to mutate the world leaves the state hash unchanged, and co-op-style runs with and without UI mods hash the same
- [x] A component that errors shows an error box naming the mod; the rest of the UI keeps running
- [x] Per-mod UI time in the profiler
- [x] The view/act API is declared once in Rust so 0085 can generate types from it

## 2026-09-23

Client-only Luau VM in sandbox mode; view (read-only, lent world pointer) and act (queues UiActions). Test ui_cannot_change_the_simulation: a hostile UI mod can't overwrite ui/view/act or reach rim/os/io, and 600 ticks with UI frames hash identically to 600 without. Component errors become red boxes naming the mod. Per-mod UI time shows in F3 as ui:<mod>. The API surface is declared in one place (vm.rs view!/act! macros) for 0085 to generate types from. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
