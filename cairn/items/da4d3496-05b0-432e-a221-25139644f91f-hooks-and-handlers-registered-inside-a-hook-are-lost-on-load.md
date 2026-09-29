---
id: da4d3496-05b0-432e-a221-25139644f91f
title: Hooks and handlers registered inside a hook are lost on load
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: scripting
---

## What

`rim.every`, `rim.on` and `rim.planner` (script.rs ~877, ~895, ~938) register at any time. Only `on_migrate` and `on_generate_level` check `r.loaded`. A registration made inside a hook or handler lives in the running game, but a load re-runs only the scripts' top-level code, so the loaded game lacks it. It also shifts the registration indices that `disabled()` saves, so the wrong hook can be switched off after a load.

## How it fails

The live and loaded games diverge whenever a mod registers lazily (for example `rim.every(7, f)` from a handler).

## Reproduce

Not yet run; verified by reading. A probe handler calls `rim.every(7, counter)` at tick 5. Save at tick 10, then compare the counter in the live and loaded games.

## Fix

Refuse registration outside load (a script error that names the function), as `on_migrate` is refused, so the rule is the same in the live and loaded games.

## Acceptance

- [ ] Registering a hook, handler or planner after load is an error the mod sees
- [ ] A test that fails before the fix and passes after
